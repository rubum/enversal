use crate::error::{DaemonError, DaemonResult};
use crate::registry::EnvironmentRegistry;
use sandbox::{Executor, SandboxPolicy};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

pub struct PythonTool;

#[async_trait::async_trait]
impl super::Tool for PythonTool {
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &brain::AgentContext,
        _env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String> {
        let requirements = args
            .get("packages")
            .and_then(|v| v.as_array())
            .map(|v| {
                v.iter()
                    .map(|p| p.as_str().unwrap_or(""))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let agent_id = context.agent_id;
        let venv_path = PathBuf::from(format!("/tmp/enversal-venvs/{}", agent_id));
        std::fs::create_dir_all(&venv_path).ok();
        
        // Canonicalize to resolve /tmp -> /private/tmp on macOS for sandbox consistency
        let venv_path = std::fs::canonicalize(&venv_path).unwrap_or(venv_path);

        let build_policy = SandboxPolicy {
            allowed_read_paths: vec![PathBuf::from("/usr"), PathBuf::from("/Library")],
            allowed_write_paths: vec![venv_path.clone()],
            block_network: false,
        };

        let mut env_vars = HashMap::new();
        let tmp_path = venv_path.join("tmp");
        std::fs::create_dir_all(&tmp_path).ok();
        
        env_vars.insert("TMPDIR".to_string(), tmp_path.display().to_string());
        env_vars.insert("PIP_CACHE_DIR".to_string(), venv_path.join("cache").display().to_string());
        env_vars.insert("HOME".to_string(), venv_path.display().to_string());

        let build_cmd = format!(
            "python3 -m venv {} && {}/bin/pip install {}",
            venv_path.display(),
            venv_path.display(),
            requirements.join(" ")
        );

        match executor.run_sandboxed(&agent_id, &build_policy, &build_cmd, &env_vars) {
            Ok(_) => {
                let mut reg = registry.runtime_registry.write().await;
                reg.entry(agent_id)
                    .or_insert_with(HashMap::new)
                    .insert("python".to_string(), venv_path);
                Ok("Python venv provisioned successfully.".to_string())
            }
            Err(e) => Err(DaemonError::ToolError(format!("Failed to provision venv: {}", e))),
        }
    }
}
