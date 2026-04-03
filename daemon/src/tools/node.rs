use crate::error::{DaemonError, DaemonResult};
use crate::registry::EnvironmentRegistry;
use sandbox::{Executor, SandboxPolicy};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct NodeTool;

#[async_trait::async_trait]
impl super::Tool for NodeTool {
    async fn execute(
        &self,
        _args: &serde_json::Value,
        context: &brain::AgentContext,
        _env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String> {
        let agent_id = context.agent_id;
        let mut workspace_path = None;
        {
            let reg = registry.runtime_registry.read().await;
            if let Some(agent_runtimes) = reg.get(&agent_id) {
                workspace_path = agent_runtimes.get("workspace").cloned();
            }
        }

        let path = workspace_path.ok_or_else(|| DaemonError::ToolError("No workspace found for agent".into()))?;

        let build_policy = SandboxPolicy {
            allowed_read_paths: vec![PathBuf::from("/usr"), PathBuf::from("/Library")],
            allowed_write_paths: vec![path.clone()],
            block_network: false,
        };

        let npm_cmd = format!("cd {} && npm install", path.display());
        match executor.run_sandboxed(&agent_id, &build_policy, &npm_cmd, &HashMap::new()) {
            Ok(_) => Ok("Node.js dependencies installed successfully.".to_string()),
            Err(e) => Err(DaemonError::ToolError(format!("npm install failed: {}", e))),
        }
    }
}
