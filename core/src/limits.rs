//! Provides the strict bounding schemas that dictate what an environment can compute.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Central resource limits defined per environment, enforced by the Control Plane.
///
/// These constraints dictate exactly how much compute, memory, and storage
/// an environment's agents are permitted to consume.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceLimits {
    /// Maximum CPU cores allocated to the environment's sandbox. Set to 0 for unlimited (not recommended).
    pub max_cpu_cores: u32,
    /// Total physical RAM (in megabytes) allowed for all combined operations.
    pub max_ram_mb: u32,
    /// Whether the agents have authorization to hit the generalized database block.
    pub db_access: bool,
    /// A whitelist of URLs the environment is permitted to fetch data from. Wildcards ("*") enable full egress.
    pub allowed_network_domains: Vec<String>,
    /// Whitelist of absolute filesystem paths allowed for read access within the Executor container.
    pub allowed_read_paths: Vec<PathBuf>,
    /// Whitelist of absolute filesystem paths allowed for write access within the Executor container.
    pub allowed_write_paths: Vec<PathBuf>,
}

impl Default for ResourceLimits {
    /// Generates incredibly strict, restrictive defaults (essentially an Isolone).
    fn default() -> Self {
        Self {
            max_cpu_cores: 1,
            max_ram_mb: 512,
            db_access: false,
            allowed_network_domains: vec![],
            allowed_read_paths: vec![],
            allowed_write_paths: vec![],
        }
    }
}
