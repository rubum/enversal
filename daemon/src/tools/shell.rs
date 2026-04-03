use crate::error::{DaemonError, DaemonResult};
use crate::registry::{ActiveEnv, EnvironmentRegistry};
use crate::tools::ToolResultExt;
use sandbox::{Executor, SandboxPolicy};
use std::collections::HashMap;
use std::sync::Arc;

pub struct ShellTool;

#[async_trait::async_trait]
impl super::Tool for ShellTool {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &brain::AgentContext,
        env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String> {
        let cmd = args
            .get("cmd")
            .and_then(|v| v.as_str())
            .ok_ok_or_else(|| DaemonError::ToolError("Missing 'cmd' argument".into()))?;

        // 1. Gather Limits for Sandbox Policy
        let mut policy = SandboxPolicy {
            allowed_read_paths: vec![],
            allowed_write_paths: vec![],
            block_network: true,
        };

        {
            let map = registry.active_environments.read().await;
            if let Some(state_entry) = map.get(env_id) {
                let env = &state_entry.active_env;
                let limits = match env {
                    ActiveEnv::Isolone(i) => &i.limits,
                    ActiveEnv::Commune(c) => &c.limits,
                };
                policy.block_network = !limits.allowed_network_domains.contains(&"*".to_string())
                    && limits.allowed_network_domains.is_empty();
                policy.allowed_read_paths = limits.allowed_read_paths.clone();
                policy.allowed_write_paths = limits.allowed_write_paths.clone();
            }
        }

        // 2. Map Runtimes (Python/Node) to Environment Variables
        let mut env_vars = HashMap::new();
        let reg = registry.runtime_registry.read().await;
        if let Some(agent_runtimes) = reg.get(&context.agent_id) {
            // Python Venv
            if let Some(venv_path) = agent_runtimes.get("python") {
                let bin_path = venv_path.join("bin");
                let existing_path = env_vars
                    .get("PATH")
                    .cloned()
                    .unwrap_or_else(|| "/usr/bin:/bin".to_string());
                env_vars.insert(
                    "PATH".to_string(),
                    format!("{}:{}", bin_path.display(), existing_path),
                );
            }
            // Node.js Workspace
            if let Some(workspace_path) = agent_runtimes.get("workspace") {
                let node_bin = workspace_path.join("node_modules").join(".bin");
                let existing_path = env_vars
                    .get("PATH")
                    .cloned()
                    .unwrap_or_else(|| "/usr/bin:/bin".to_string());
                env_vars.insert(
                    "PATH".to_string(),
                    format!("{}:{}", node_bin.display(), existing_path),
                );

                // Allow write access to the workspace
                policy.allowed_write_paths.push(workspace_path.clone());
                env_vars.insert(
                    "WORKSPACE".to_string(),
                    workspace_path.display().to_string(),
                );
            }
        }

        // 3. Run Sandboxed Command
        executor
            .run_sandboxed(&context.agent_id, &policy, cmd, &env_vars)
            .map_err(|e| DaemonError::SandboxError(format!("Sandbox Execution Failed: {}", e)))
    }
}
