//! Ollama local AI backend integration.

use super::{AgentContext, CognitiveEngine, ReasoningOutput};
use crate::prompt::{parse_tool_call, MASTER_SYSTEM_INSTRUCTION};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Plugs into a local Ollama instance (defaulting to http://localhost:11434).
pub struct OllamaEngine {
    /// The base URL for the Ollama server.
    pub host: String,
    /// Connection pool for optimized HTTP requests.
    pub client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: ChatMessage,
}

impl OllamaEngine {
    /// Initializes a new connection to a local Ollama instance.
    pub fn new(host: Option<String>) -> Self {
        Self {
            host: host.unwrap_or_else(|| "http://localhost:11434".to_string()),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CognitiveEngine for OllamaEngine {
    async fn reason(&self, agent_context: &AgentContext) -> Result<ReasoningOutput> {
        let url = format!("{}/api/chat", self.host);

        let system_prompt = format!(
            "{}\n\n### YOUR SPECIFIC GOAL\n{}",
            MASTER_SYSTEM_INSTRUCTION, agent_context.system_prompt
        );

        let mut messages = vec![ChatMessage {
            role: "system".into(),
            content: system_prompt,
        }];

        for obs in &agent_context.recent_observations {
            messages.push(ChatMessage {
                role: "user".into(),
                content: obs.clone(),
            });
        }

        let request = OllamaChatRequest {
            model: agent_context.model.clone(),
            messages,
            stream: false,
        };

        let pending_request = self.client.post(url).json(&request).send();

        let response = tokio::time::timeout(std::time::Duration::from_secs(300), pending_request)
            .await
            .map_err(|_| {
                anyhow::anyhow!("Ollama RPC Local Timeout: Exceeded 300 seconds without response. This may occur during initial model loading.")
            })??;

        let response_text = response.text().await?;
        let ollama_res: OllamaChatResponse = serde_json::from_str(&response_text).map_err(|e| {
            anyhow::anyhow!(
                "Failed to decode Ollama response: {}. Raw body: {}",
                e,
                response_text
            )
        })?;

        let text = ollama_res.message.content;

        // Parse XML tool calls using shared logic
        if let Some(tool_call) = parse_tool_call(&text) {
            return Ok(ReasoningOutput::ToolCall(tool_call));
        }

        Ok(ReasoningOutput::Message(text))
    }
}
