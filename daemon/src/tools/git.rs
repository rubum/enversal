use crate::error::{DaemonError, DaemonResult};
use crate::registry::EnvironmentRegistry;
use crate::tools::ToolResultExt;
use sandbox::{Executor, SandboxPolicy};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct GitTool;

#[async_trait::async_trait]
impl super::Tool for GitTool {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &brain::AgentContext,
        _env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String> {
        let url = args
            .get("url")
            .and_then(|v| v.as_str())
            .ok_ok_or_else(|| DaemonError::ToolError("Missing 'url' argument".into()))?;

        let agent_id = context.agent_id;
        let repo_path = PathBuf::from(format!("/tmp/enversal-workspaces/{}/repo", agent_id));
        std::fs::create_dir_all(&repo_path).ok();

        let clone_policy = SandboxPolicy {
            allowed_read_paths: vec![PathBuf::from("/usr")],
            allowed_write_paths: vec![repo_path.clone()],
            block_network: false,
        };

        let clone_cmd = format!("git clone {} {}", url, repo_path.display());
        match executor.run_sandboxed(&agent_id, &clone_policy, &clone_cmd, &HashMap::new()) {
            Ok(_) => {
                let mut reg = registry.runtime_registry.write().await;
                reg.entry(agent_id)
                    .or_insert_with(HashMap::new)
                    .insert("workspace".to_string(), repo_path);
                Ok("Repository cloned successfully into workspace.".to_string())
            }
            Err(e) => Err(DaemonError::ToolError(format!("Git clone failed: {}", e))),
        }
    }
}
