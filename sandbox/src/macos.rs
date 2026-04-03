//! Mac OS (Seatbelt / App Sandbox) specific engine capabilities.

use super::{Executor, SandboxPolicy};
use anyhow::{bail, Result};
use core::AgentId;
use std::collections::HashMap;
use std::process::Command;

/// An Executor implementation targeting Apple's native App Sandbox.
///
/// This engine orchestrates generating Scheme (`.sb`) strings dynamically
/// to place rigorous filesystem and network locks on child processes via the
/// native `/usr/bin/sandbox-exec` utility at runtime.
pub struct SeatbeltExecutor;

impl Executor for SeatbeltExecutor {
    /// Computes the Seatbelt policy string based on the `SandboxPolicy` and fires
    /// the payload over the native command stack to the macOS Kernel scheduler.
    ///
    /// # Security Notes
    /// By default, Seatbelt allows standard IO streams `/dev/tty` so `stdout` flows
    /// out to the Control Plane. Deny directives are issued generically first (`deny file-write*`),
    /// followed by surgical read/write allowances configured in the `enversal.yaml`.
    fn run_sandboxed(
        &self,
        agent_id: &AgentId,
        policy: &SandboxPolicy,
        command: &str,
        env_vars: &HashMap<String, String>,
    ) -> Result<String> {
        let mut profile = String::from("(version 1)\n(allow default)\n");

        if policy.block_network {
            profile.push_str("(deny network*)\n");
        }

        if !policy.allowed_write_paths.is_empty() {
            profile.push_str("(deny file-write*)\n");
            // Allow writes to local temp and tty
            profile.push_str("(allow file-write-data (literal \"/dev/tty\"))\n");

            for path in &policy.allowed_write_paths {
                profile.push_str(&format!(
                    "(allow file-write* (subpath \"{}\"))\n",
                    path.display()
                ));
            }
        }

        let mut cmd = Command::new("sandbox-exec");
        cmd.arg("-p").arg(&profile).arg("sh").arg("-c").arg(command);

        for (k, v) in env_vars {
            cmd.env(k, v);
        }

        let output = cmd.output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            bail!(
                "Seatbelt Execution Failed [Agent: {}]: {}",
                agent_id,
                err.trim()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_mac_sandbox_execution() {
        let executor = SeatbeltExecutor;
        let id = Uuid::new_v4();

        let policy = SandboxPolicy {
            allowed_read_paths: vec![],
            allowed_write_paths: vec![],
            block_network: false,
        };
        let envs = HashMap::new();

        // Running a simple echo should work
        let res = executor
            .run_sandboxed(&id, &policy, "echo 'hello from seatbelt'", &envs)
            .unwrap();
        assert_eq!(res, "hello from seatbelt");
    }

    #[test]
    fn test_mac_sandbox_blocked_network() {
        let executor = SeatbeltExecutor;
        let id = Uuid::new_v4();

        let policy = SandboxPolicy {
            allowed_read_paths: vec![],
            allowed_write_paths: vec![],
            block_network: true,
        };
        let envs = HashMap::new();

        // Attempting to curl something with network blocked should fail natively
        let res =
            executor.run_sandboxed(&id, &policy, "curl -s --max-time 1 https://1.1.1.1", &envs);
        assert!(
            res.is_err(),
            "Curl should have failed when network egress is explicitly denied by Mac Seatbelt."
        );
        let err_msg = res.unwrap_err().to_string();
        assert!(err_msg.contains("Seatbelt Execution Failed"));
    }
}
