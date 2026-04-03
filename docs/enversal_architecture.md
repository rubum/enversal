# Enversal: Technical Architecture

Enversal is a high-fidelity, zero-trust Cognitive Operating System. Unlike traditional orchestrators that assume software behaves securely, Enversal was engineered on the presumption that AI Agents are unpredictable, heavily-capable entities that require physical, OS-level constraints to operate safely on a host machine.

This document details the actual mechanics of the `v1.0` Enversal Rust workspace.

## 1. The Workspace Crates

Enversal is separated into four primary compilation targets to strictly enforce the separation of concerns between Reasoning, Control, and Execution.

### `enversal-core`
The pure data model definitions (`EnversalManifest`, `Agent`, `ResourceLimits`). Contains zero side-effect-inducing dependencies.
- Evaluates memory models (`Isolone` vs `Commune`).
- Parses declarative `enversal.yaml` specifications.

### `enversal-brain`
The AI reasoning interface (`CognitiveEngine`).
- Currently maps to the Google `Gemini-3-Flash-Preview` REST API.
- Implements strict, deterministic JSON Candidate parsing to extract heuristic tool calls (e.g. `sandbox_exec`).
- Manages the `AgentContext` ingestion.

### `enversal-sandbox`
The kernel-level execution abstraction (`Executor`).
- Defines the `SandboxPolicy` constraints (Read Paths, Write Paths, Network Blocking).
- Computes environment variables and `WORKSPACE` bindings for the internal child process.

### `enversal-daemon`
The `tokio`-based gRPC Control Plane.
- Orchestrates the asynchronous Cognitive Loops.
- Tracks process IDs (PIDs) and `termimad` high-fidelity markdown streaming.
- Manages the `RuntimeRegistry` for autonomous language provisioning.

---

## 2. Zero-Trust Execution (The Seatbelt Engine)

Enversal does not use heavy Docker containers or virtual machines for its isolated workloads. It interacts directly with the host kernel's built-in macro-security features to generate ephemeral, inescapable jails in milliseconds.

### The `SeatbeltExecutor` (macOS)
When an Agent attempts to execute a tool (like running a python script or touching a file), the Daemon computes a `SandboxPolicy`.

**The flow:**
1. **Network Egress Block:** The daemon checks the `AgentConfig`. If no explicit network domains are approved, it dynamically injects `(deny network*)` into the Scheme `sb` profile string.
2. **Global File Write Ban:** It injects a global `(deny file-write*)` to ensure the agent cannot overwrite host system binaries.
3. **Surgical Path Allowance:** It specifically loops through the `allowed_write_paths` (derived from the `enversal.yaml`) and punches targeted holes using `(allow file-write* (subpath "<PATH>"))`.
4. **Execution:** The raw payload is passed into `/usr/bin/sandbox-exec -p <profile> sh -c <command>`.

If a rogue agent tries to run `curl` to leak data out, the Apple Kernel intercepts the syscall and returns a hostile failure, which Enversal captures and logs as a `Seatbelt Execution Failed` error.

---

## 3. The Autonomous Toko Loop (`spawn_cognitive_loop`)

When an environment is provisioned via the gRPC endpoint `ProvisionEnvironment`, the Daemon executes `spawn_cognitive_loop`. This detaches a `tokio::spawn` asynchronous task representing the "heartbeat" of the universe.

### The 5-Second Heartbeat
1. **Wake & Read:** Every 5 seconds, the loop iterates over all alive agents in the `ActiveEnv`.
2. **Context Assembly:** It compiles the `AgentContext`, aggregating the `system_prompt` and all recent execution observations.
3. **Reason:** Polling the `CognitiveEngine` (Gemini), it awaits a `ReasoningOutput`.
4. **Dispatch:**
    - If it's a `Message`, it logs to the `termimad` standard output.
    - If it's a `ToolCall`, the `DaemonService` intercepts the request and handles it natively.

---

## 4. Project-Level Autonomy & The `RuntimeRegistry`

Enversal supports complex, multi-language orchestration (cloning repos, building node modules, managing python virtual environments) without breaking the Zero-Trust guarantee.

### The Problem
If an agent wants to run `npm install`, it needs access to fetch from the internet and write to a `node_modules` folder. But giving an agent a permanent, internet-enabled sandbox is dangerous.

### The Solution: Ephemeral Build Sandboxes
Enversal implements native tool handlers (`provision_env`, `git_clone`, `npm_install`) inside the Control Plane.

1. **`git_clone`**: The Daemon intercepts the request, spawns an egress-enabled sandbox specifically for the `git` binary, clones the repository into `/tmp/enversal-workspaces/<agent_id>`, and immediately terminates the sandbox.
2. **The `RuntimeRegistry`**: The Control Plane adds this new workspace path into the `RuntimeRegistry` under `Agent X -> workspace -> /tmp/...`.
3. **Execution Injection**: On the very next tick of the Cognitive Loop, when the agent requests to `sandbox_exec`, the Daemon reads the registry. It automatically modifies the child process `PATH` to include `node_modules/.bin`, injects the `WORKSPACE` environment variable, and expands the `allowed_write_paths` so the agent can interact with its cloned code natively.

This enables **"Security Auditor"** workflows, where Enversal can clone a repository and run `npm audit` or `semgrep`, parsing the results in heavily restrictive sandboxes to prevent malicious `.git/hooks` or rogue `npm postinstall` scripts from compromising the Daemon host.
