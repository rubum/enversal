use crate::error::{DaemonError, DaemonResult};
use crate::registry::EnvironmentRegistry;
use brain::AgentContext;
use sandbox::Executor;
use std::collections::HashMap;
use std::sync::Arc;

pub mod git;
pub mod node;
pub mod python;
pub mod shell;

/// A standard interface for providing AI agents with functional capabilities.
#[async_trait::async_trait]
pub trait Tool: Send + Sync {
    /// Executes the tool with the provided arguments.
    async fn execute(
        &self,
        args: &serde_json::Value,
        context: &AgentContext,
        env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String>;
}

/// A registry that maps tool names to their respective implementations.
pub struct ToolDispatcher {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolDispatcher {
    pub fn new() -> Self {
        let mut tools: HashMap<String, Box<dyn Tool>> = HashMap::new();
        tools.insert("sandbox_exec".to_string(), Box::new(shell::ShellTool));
        tools.insert("provision_env".to_string(), Box::new(python::PythonTool));
        tools.insert("git_clone".to_string(), Box::new(git::GitTool));
        tools.insert("npm_install".to_string(), Box::new(node::NodeTool));

        Self { tools }
    }

    pub async fn dispatch(
        &self,
        name: &str,
        args: &serde_json::Value,
        context: &AgentContext,
        env_id: &str,
        registry: &EnvironmentRegistry,
        executor: &Arc<dyn Executor + Send + Sync>,
    ) -> DaemonResult<String> {
        let tool = self
            .tools
            .get(name)
            .ok_ok_or_else(|| DaemonError::ToolError(format!("Tool '{}' not found", name)))?;
        tool.execute(args, context, env_id, registry, executor).await
    }
}

pub trait ToolResultExt<T> {
    fn ok_ok_or_else<F>(self, f: F) -> DaemonResult<T>
    where
        F: FnOnce() -> DaemonError;
}

impl<T> ToolResultExt<T> for Option<T> {
    fn ok_ok_or_else<F>(self, f: F) -> DaemonResult<T>
    where
        F: FnOnce() -> DaemonError,
    {
        self.ok_or_else(f)
    }
}
