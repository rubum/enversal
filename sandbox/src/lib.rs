//! Enversal Executor and Sandboxing engine.
//!
//! This crate serves as the strict, OS-level boundary enforcement mechanism for
//! Enversal agents. It defines the logic required to parse arbitrary payloads and
//! execute them under native security primitives (e.g. macOS Seatbelt, Linux Landlock),
//! preventing malicious escapes.

use anyhow::Result;
use core::AgentId;
use std::collections::HashMap;

pub mod config;
pub use config::SandboxPolicy;

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos::SeatbeltExecutor;

cfg_if::cfg_if! {
    if #[cfg(target_os = "macos")] {
        pub use macos::SeatbeltExecutor as NativeExecutor;
    } else if #[cfg(target_os = "linux")] {
        pub mod linux;
        pub use linux::LandlockExecutor as NativeExecutor;
    } else {
        // Fallback for Windows or unsupported OS
    }
}

/// The generic capability of an Enversal Executor.
///
/// Every platform-specific executor must implement this trait so the rest
/// of Enversal can safely request tool executions passing arbitrary commands.
/// The Executor acts as the final boundary before the raw OS kernel, responsible
/// for taking abstract `SandboxPolicy` constraints and materializing them firmly
/// onto child processes.
pub trait Executor {
    /// Initializes an ephemeral sandbox and executes a tool payload securely.
    ///
    /// # Arguments
    /// * `agent_id` - The identity of the caller (used for logging and pathing).
    /// * `policy` - The tightest allowable constraints computed for this execution run.
    /// * `command` - The specific payload/bash script to invoke natively.
    /// * `env_vars` - Injected context variables (e.g., `PATH` modifications for virtual environments).
    ///
    /// # Returns
    /// A `Result` containing the `stdout` buffer on success, or an explicit execution timeout/failure.
    fn run_sandboxed(
        &self,
        agent_id: &AgentId,
        policy: &SandboxPolicy,
        command: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<String>;
}
