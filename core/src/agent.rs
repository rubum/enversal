//! Defines the Agent entity, roles, and configurations for the Enversal universe.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// A strongly-typed unique identifier for an Agent, represented as a UUID v4.
/// This ID is foundational to zero-trust mTLS verification on the data plane.
pub type AgentId = Uuid;

/// Represents a distinct capability or tool the agent is allowed to execute.
pub type ToolId = String;

/// The role an agent plays within its environment. Roles define default
/// permissions and behavioral templates.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum AgentRole {
    /// The primary planner and orchestrator of a Commune. Spawns other agents.
    Leader,
    /// A localized sub-agent explicitly created to fulfill a given persona or function.
    Worker(String),
}

/// The runtime blueprint configuring an agent's brain and capabilities.
///
/// This specifies how the agent behaves, which underlying Generative AI model
/// powers its reasoning loop, and the explicit list of tools it is legally
/// permitted to trigger through the Executor sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The assigned role within the environment (e.g. Leader or Worker).
    pub role: AgentRole,
    /// The specific LLM model backing this agent's logic (e.g., "gemini-pro").
    pub model: String,
    /// The explicit set of `ToolId`s this agent is authorized to use via the Executor.
    pub allowed_tools: HashSet<ToolId>,
}

/// Represents the physical manifestation of an AI entity inside Enversal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// The cryptographically secure, immutable identifier.
    pub id: AgentId,
    /// The human-readable handle identifying the agent in logs and CLI output.
    /// Human-friendly moniker.
    pub name: String,
    /// Philosophical and operational constraints.
    pub config: AgentConfig,
    /// Current OS Process ID if executing.
    pub current_pid: Option<u32>,
}

impl Agent {
    /// Genesis: Spawns a new agent identity.
    ///
    /// # Arguments
    /// * `name` - A human-friendly moniker for logging.
    /// * `config` - The specific brain and capability configuration.
    ///
    /// # Examples
    /// ```rust
    /// use std::collections::HashSet;
    /// use enversal_core::agent::{Agent, AgentConfig, AgentRole};
    ///
    /// let config = AgentConfig {
    ///     role: AgentRole::Leader,
    ///     model: "gemini-3.1-pro".to_string(),
    ///     allowed_tools: HashSet::from(["fs:read".to_string()]),
    /// };
    /// let agent = Agent::new("coordinator", config);
    /// assert_eq!(agent.name, "coordinator");
    /// ```
    pub fn new(name: impl Into<String>, config: AgentConfig) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            config,
            current_pid: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation() {
        let config = AgentConfig {
            role: AgentRole::Worker("Researcher".to_string()),
            model: "gemini-lite".to_string(),
            allowed_tools: HashSet::new(),
        };
        let agent = Agent::new("weather-bot", config);
        assert_eq!(agent.name, "weather-bot");
        assert_eq!(
            agent.config.role,
            AgentRole::Worker("Researcher".to_string())
        );
        assert!(
            !agent.id.is_nil(),
            "Agent ID must be a valid non-nil UUID v4"
        );
    }
}
