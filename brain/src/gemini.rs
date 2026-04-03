//! Gemini-3-Flash native integration.

use super::{AgentContext, CognitiveEngine, ReasoningOutput};
use crate::prompt::{parse_tool_call, MASTER_SYSTEM_INSTRUCTION};
use anyhow::Result;
use async_trait::async_trait;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};

/// Plugs into Google's Gemini REST API.
pub struct GeminiEngine {
    /// The secure bearer token for authentication. Zeroized automatically on drop.
    pub api_key: SecretString,
    /// Connection pool for optimized HTTP requests.
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<Part>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Part {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<Candidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Deserialize)]
struct GeminiError {
    message: String,
    code: Option<i32>,
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    content: Content,
}

impl GeminiEngine {
    /// Initializes a new connection binding to the provided Google AI Studio API key.
    pub fn new(api_key: String) -> Self {
        Self {
            api_key: SecretString::new(api_key.into()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CognitiveEngine for GeminiEngine {
    async fn reason(&self, agent_context: &AgentContext) -> Result<ReasoningOutput> {
        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            agent_context.model,
            self.api_key.expose_secret()
        );

        let system_prompt = format!(
            "{}\n\n### YOUR SPECIFIC GOAL\n{}",
            MASTER_SYSTEM_INSTRUCTION, agent_context.system_prompt
        );

        let system_instruction = Some(Content {
            role: "system".into(),
            parts: vec![Part {
                text: system_prompt,
            }],
        });

        let contents = agent_context
            .recent_observations
            .iter()
            .map(|obs| Content {
                role: "user".into(),
                parts: vec![Part { text: obs.clone() }],
            })
            .collect();

        let request = GeminiRequest {
            contents,
            system_instruction,
        };

        // Wrap the network request in a rigorous 25-second timeout boundary
        let pending_request = self.client.post(url).json(&request).send();

        let response = tokio::time::timeout(std::time::Duration::from_secs(60), pending_request)
            .await
            .map_err(|_| {
                anyhow::anyhow!("Gemini RPC Network Timeout: Exceeded 60 seconds without response.")
            })??;

        let response_text = response.text().await?;

        let response: GeminiResponse = serde_json::from_str(&response_text).map_err(|e| {
            anyhow::anyhow!(
                "Failed to decode Gemini response: {}. Raw body: {}",
                e,
                response_text
            )
        })?;

        if let Some(err) = response.error {
            return Err(anyhow::anyhow!(
                "Gemini API Error: {} (Status: {:?}, Code: {:?})",
                err.message,
                err.status,
                err.code
            ));
        }

        if let Some(candidates) = response.candidates {
            if let Some(candidate) = candidates.first() {
                if let Some(part) = candidate.content.parts.first() {
                    let text = &part.text;

                    // Parse XML tool calls using shared module logic
                    if let Some(tool_call) = parse_tool_call(text) {
                        return Ok(ReasoningOutput::ToolCall(tool_call));
                    }

                    return Ok(ReasoningOutput::Message(text.clone()));
                }
            }
        }

        anyhow::bail!("Gemini returned no candidates. Raw body: {}", response_text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gemini_struct_compilation() {
        let engine = GeminiEngine::new("mock-key".to_string());
        assert_eq!(engine.api_key.expose_secret(), "mock-key");
    }
}
