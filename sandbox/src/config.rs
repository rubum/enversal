//! Types and schemas related to native sandbox restrictions.

use std::path::PathBuf;

/// Defines exactly what an Executor is natively allowed to do on the host OS.
///
/// The Control Plane compiles this construct and mounts it prior to calling
/// the Executor, ensuring isolation levels match the environment's `ResourceLimits`.
#[derive(Debug, Clone, Default)]
pub struct SandboxPolicy {
    /// Whitelisted file system locations that can be read safely by the execution thread.
    pub allowed_read_paths: Vec<PathBuf>,
    /// Explicitly granted paths for standard I/O streams and artifact generation.
    pub allowed_write_paths: Vec<PathBuf>,
    /// Should the Linux Kernel / MacOS Seatbelt explicitly drop all packet egress capability?
    pub block_network: bool,
}
