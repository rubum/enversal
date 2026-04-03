//! Struct definitions mapping directly to the `enversal.yaml` schema.
//!
//! Provides strong typing via `serde` for validating environments before submitting them
//! to the Control Plane for execution.

use serde::{Deserialize, Serialize};

/// The root manifest describing the entire environment configuration.
/// Deserialized directly from user-provided blueprints (e.g. `data_analysis.yaml`).
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnversalManifest {
    pub version: String,
    pub environment: EnvironmentConfig,
    pub resources: ResourcesConfig,
    pub context: ContextConfig,
    pub security: SecurityConfig,
    pub agents: AgentsConfig,
}

/// The core metadata parameters for the Environment.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EnvironmentConfig {
    pub name: String,
    #[serde(rename = "type")]
    pub env_type: String, // e.g. "commune" or "isolone"
    pub goal: String,
    /// The AI backend provider (e.g. "gemini", "ollama") to use for reasoning.
    #[serde(default)]
    pub ai_provider: String,
}

/// The physical resource boundaries applied strictly to this environment sandbox.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ResourcesConfig {
    pub cpu_cores: u32,
    pub ram_mb: u32,
    pub log_size_mb: u32,
    pub storage_limit_mb: u32,
    pub db_access: bool,
    pub network: NetworkConfig,
    pub filesystem: FilesystemConfig,
}

/// Granular filesystem access control.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct FilesystemConfig {
    pub allowed_read_paths: Vec<String>,
    pub allowed_write_paths: Vec<String>,
}

/// The zero-trust domain whitelist policies.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct NetworkConfig {
    pub allow_outbound: bool,
    pub allowed_domains: Vec<String>,
}

/// Characteristics governing how the LLMs retain execution context.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContextConfig {
    pub shared_memory_type: String,
    pub max_tokens_per_agent: u32,
}

/// Identity and provisioning secrets configurations.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct SecurityConfig {
    pub mtls_enabled: bool,
    pub vault_provider: String,
}

/// Represents the entities provisioned within the initial scope.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentsConfig {
    pub leader: AgentSpec,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_workers: Option<Vec<AgentSpec>>,
}

/// A specific agent template detailing capabilities and persona.
#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentSpec {
    pub name: String,
    pub model: String,
    pub capabilities: Vec<String>,
    pub system_prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}
