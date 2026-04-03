# Enversal: The Universe for AI Autonomy

## What is Enversal?
At its core, Enversal is an environment management system built specifically for AI agents. Think of it as a universe for autonomy—a place where environments can be spun up for agents to operate either entirely on their own (`Isolone`) or within connected, collaborative communities (`Commune`). 

### The Enversal Martian
Imagine a brilliant engineer who travels to Mars. When he first lands, he is entirely alone—cut off from Earth and unable to interact with the outside universe. He exists in an **Isolone**, a strictly isolated environment with restricted resources and a singular goal: survival.

Because he is technologically capable, he realizes he cannot colonize the planet alone. He begins to clone and replicate himself, creating new entities in his own image. He assigns each of them specific roles and tasks—one to manage the oxygen scrubbers, another to mine ice, and another to build habitats. Together, they form a **Commune**, a shared and collaborative environment working toward a common goal. He acts as their leader, planning their expansion and managing their resources.

Years later, the original engineer passes away. But the Commune does not collapse. Because they share a collective objective and resources, the remaining clones hold an election, reach a quorum, and vote for a new leader. Under this new leadership, the community continues to thrive, eventually colonizing the entire planet.

This is the essence of Enversal: a system that starts from isolated, singular agents building up to fault-tolerant, self-replicating, and collaborative communities of AI.

---

## The Two Worlds of Enversal

### 1. Isolone: The Hermit's Sandbox
![Isolone Vector](/Users/macken/Codev/Enversal/assets/enversal_isolone_vector_1774617803604.png)

An Isolone is a strictly isolated environment built for a *single* agent. It's the perfect place for a solitary task. The agent inside operates under strict local resource constraints, with access only to the tools it was explicitly given, and focused entirely on a specific goal.
- **Total Isolation**: By default, it cannot reach out to the outside universe unless explicitly permitted.
- **Mission Accomplished, World Ends**: Once the agent's goal is met, its Isolone is quietly terminated.
- **Fluid Boundaries**: An Isolone agent can decide to "join" an existing Commune and become part of a larger society. Conversely, an agent tired of community life in a Commune can "exit" and form its own isolated Isolone.

### 2. Commune: The Thriving Society
![Commune Vector](/Users/macken/Codev/Enversal/assets/enversal_commune_vector_1774617944539.png)

A Commune is a shared, deeply connected environment where multiple agents live and work together. It's built for collaboration.
- **The Leader**: Every Commune starts with a "default agent" who steps up as the leader—the planner and builder. This leader sets the agenda, clones new agents into existence, assigns them roles, and orchestrates the grand plan.
- **Fault Tolerance (The Election)**: In the unpredictable universe of AI, leaders can fail or die (hit a timeout, exception, or API limit). If the leader falls, the society doesn't collapse. The surviving agents can cast votes, reach a quorum, or even draw lots to elect a new leader and keep the mission alive.
- **Sharing is Caring**: Every agent within the Commune shares the same overarching goal, along with the same pool of resources—CPU, memory, logs, and database access.

### 3. Wormholes: Inter-Environment Trading
While Isolones and Communes are strictly defined boundaries, temporary integration is possible through a **Wormhole Protocol**. A Wormhole acts as an inter-environment gateway, allowing Commune A (e.g., a Market Economy) to temporarily query or trade data with Commune B (e.g., a Weather Simulation) without merging their isolated universes. This protocol even supports "tokenomics" for distinct clusters bidding for shared compute resources.

---

## How It Works: The Architecture

To make all of this possible, scalable, and secure, Enversal is split into two foundational layers: the **Control Plane** and the **Data Plane**.

### The Control Plane (The Universe's Physics)
The Control Plane is the unstoppable force that governs the rules and physical laws of the universe. It orchestrates the lifecycles of environments and agents.
- **Creation & Destruction**: It spins up and tears down Isolones and Communes, strictly enforcing node resource limits (CPU, RAM, DB access).
- **The Eye of God (Observability)**: The Control Plane hosts an immutable semantic **Auditing Engine**. Every prompt, context injection, and tool execution is recorded in a trace-tree. If an agricultural Commune suddenly burns its crops, human operators can trace the exact chain of agent reasoning.
- **Time & Snapshots**: For simulations, the Control Plane manages Time Dilation (e.g., executing 1 second of real-world processing to simulate 1 week). It also controls **State Snapshots**, allowing it to freeze an entire Commune, flush the Vector DB and Executor locks to disk, and resume a paused universe later without losing a single token.
- **The Protocol**: It communicates via **gRPC**, relying on strict, typed, and ultra-efficient contracts to manage environments safely.

### The Data Plane (The Agents' Playground)
If the Control Plane is the physics, the Data Plane is the society. This is where agents actually live, talk to each other, and do work.
- **Agent Communication & mTLS**: It connects agents using a flexible, high-throughput protocol like **JSON-RPC or NATS (Pub/Sub)**. Because AI payloads are highly dynamic, this event bus allows fluid chatting. To ensure Zero-Trust Security, every agent connection requires **Mutual TLS (mTLS)**. If an agent claims to be the "Leader" on the bus, its dynamically minted, short-lived x509 certificate mathematically proves it.
- **The Plugin Store**: Tools aren't just strings; they are powered by a **Standardized Capability Registry**. Using the Model Context Protocol (MCP) or an internal WebAssembly (Wasm) runtime, developers can inject safe, sandboxed tools on the fly.
- **Secure Vaults**: LLM API keys or database passwords aren't fed into prompts. The Data Plane utilizes a **Secure Vault System** that injects raw secrets directly into the Executor at runtime, ensuring an agent never leaks a private key.

---

## Environment Provisioning & OS-Level Sandboxing (The Hypervisor for AI)
If Enversal is going to enforce strict "physical laws"—limiting an Isolone to exactly 512MB of RAM, 2 cores, and zero network access—it cannot simply trust the agent not to overstep. It must natively control the execution sandbox.

Enversal acts as a **Micro-Provisioning Engine**, carving out the universe using native OS-level boundary enforcement rather than relying on heavy, slow virtual machines.

### 1. OS-Native Sandboxing vs Heavy VMs
When an agent or executor is spawned, Enversal interacts directly with the host operating system's lowest-level security primitives to create an inescapable box.
- **Linux (Landlock & seccomp):** On Linux, Enversal uses `Landlock` to strictly define which file paths the executor can read or write. It couples this with `seccomp-bpf` to filter which system calls the agent is legally allowed to execute, ensuring an agent can't fork-bomb the host or open raw network sockets.
- **macOS (Seatbelt / App Sandbox):** On macOS, Enversal generates dynamic `Seatbelt` profiles (using Scheme code strings) at runtime. When the Isolone boots, it is thrown into a Seatbelt jail where the macOS kernel physically prevents it from escaping its designated workspace folder or accessing forbidden hardware.
- **MicroVMs (Firecracker):** For incredibly hostile or mission-critical tasks where even OS-level jails aren't enough, Enversal can optionally provision AWS Firecracker microVMs. These boot in milliseconds, providing hardware-level KVM isolation while feeling as fast as a local process.

### 2. The "Seeding" Process
When you run `enversal run project-genesis.yaml`, Enversal doesn't spin up AWS infrastructure (that's Terraform's job). Instead, it provisions the *agent's local compute envelope*:
1. **Carve the Universe:** The Control Plane asks the host OS kernel (via cgroups/namespaces) to allocate the exact memory and CPU quotas.
2. **Mount the Tools:** It natively binds the `Executor` binaries and injected MCP plugins (like Wasm tool-chains) locally into the new sandbox.
3. **The Spark of Life:** It mints the mTLS certificates, seamlessly injects API secrets via the Secure Vault natively in memory, and sends the awakening `system_prompt` to the agent.

---

## Context Management: Remembering Without Overwhelming

Language models have a fatal flaw: their memories (context windows) are limited. If an agent tries to remember everything, it will freeze. Enversal handles this elegantly:
- **The Commune Board (Shared Memory)**: Communes have a shared database (often a Vector DB or an in-memory graph). Instead of holding all knowledge in its head, an agent can "query" the board to find out what its peers are up to.
- **The Ephemeral Mind (Per-Agent Memory)**: Each agent has its own short-term memory. But right before it overflows, an internal subconscious routine (a summarizer) steps in, condensing the history down to its core essence.
- **The Context Bus**: When an agent executes a tool, the result is broadcasted on the data plane. But agents only listen to what they *care* about, actively subscribing to specific streams so their minds aren't cluttered with noise.
- **Scanning the Horizon**: Just like looking around a room, an agent can query the Control Plane to "scan" its environment. It receives a manifesto detailing what MCP tools are mounted, which other agents are alive, and how much CPU is left to burn.

---

## Bringing It to Life in Rust

To make this universe blazing fast and memory-safe, we build it in Rust. Here is a glimpse of the foundational structs and traits:

```rust
use std::collections::{HashMap, HashSet};

// --- Core Identifiers ---
pub type EnvId = String;
pub type AgentId = String;
pub type ToolId = String;

// --- The Core of the Universe ---
/// The fundamental laws every Enversal environment obeys.
pub trait Environment {
    fn id(&self) -> &EnvId;
    fn spawn_agent(&mut self, config: AgentConfig) -> Result<AgentId, EnvError>;
    fn terminate_agent(&mut self, id: &AgentId) -> Result<(), EnvError>;
    fn resource_limits(&self) -> &ResourceLimits;
    fn scan(&self) -> EnvManifest;
}

/// What an agent can actually do.
pub trait AgentBehavior {
    fn act(&mut self, context: &AgentContext) -> Result<Action, AgentError>;
    fn communicate(&self, target: &AgentId, msg: Message) -> Result<(), AgentError>;
}

// --- World Types ---

/// The physical limits of a world.
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_cpu_cores: u32,
    pub max_ram_mb: u32,
    pub max_log_size_mb: u32,
    pub storage_limit_mb: u32,
    pub db_access: bool,
    pub allowed_network_domains: Vec<String>,
}

/// The Hermit's Sandbox
pub struct Isolone {
    pub env_id: EnvId,
    pub target_goal: String,
    pub resources: ResourceLimits,
    pub agent: Option<Agent>,     // At most one!
    pub executor: Executor,
}

impl Environment for Isolone { /* Trait methods implemented here */ }

/// The Thriving Society
pub struct Commune {
    pub env_id: EnvId,
    pub shared_goal: String,
    pub resources: ResourceLimits,
    pub leader_id: AgentId,
    pub agents: HashMap<AgentId, Agent>,
    pub executor: Executor,
    pub shared_memory: SharedContext,
}

impl Environment for Commune { /* Trait methods implemented here */ }

// --- The Inhabitants ---

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub role: AgentRole,
    pub model: String,
    pub allowed_tools: HashSet<ToolId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AgentRole {
    Leader,
    Worker(String), // E.g., Worker("Botanist")
}

pub struct Agent {
    pub id: AgentId,
    pub config: AgentConfig,
    pub local_context: AgentContext,
}

// --- The Hands that do the work ---

/// The Executor runs commands in the real world, safely boxed by constraints.
pub struct Executor {
    pub environment_id: EnvId,
}

impl Executor {
    pub fn execute(&self, agent_id: &AgentId, tool: ToolId, args: Vec<String>) -> Result<ExecutionOutput, ExecError> {
        // Enforces resource quotas natively, runs the tool, returns the result.
        todo!()
    }
}

// --- Ephemeral Data ---
pub struct Action { pub tool: ToolId, pub payload: String }
pub struct Message { pub sender: AgentId, pub content: String }

#[derive(Debug)] pub struct EnvError;
#[derive(Debug)] pub struct AgentError;
#[derive(Debug)] pub struct ExecError;

pub struct AgentContext;
pub struct SharedContext;
pub struct EnvManifest;
pub struct ExecutionOutput;
```

---

## Defining the World: `enversal.yaml`

Before a world is born, it must be designed. An Enversal environment is defined declaratively. Here is a blueprint for an agricultural Commune on alien soil:

```yaml
# enversal.yaml

version: "1.0"
environment:
  name: "project-genesis"
  type: "commune" # Can be "isolone" or "commune"
  goal: "Simulate land resources management and optimize agricultural yield on alien soil."

# The physics of the simulation
resources:
  cpu_cores: 4
  ram_mb: 8192
  log_size_mb: 500
  storage_limit_mb: 10240
  db_access: true
  network:
    allow_outbound: true
    allowed_domains:
      - "api.weather.local"
      - "marketdata.internal"

# How memories are stored
context:
  shared_memory_type: "vector-db"
  max_tokens_per_agent: 16384 # Trigger the ephemeral summarizer when this limit is reached

# Security & Identity
security:
  mtls_enabled: true
  vault_provider: "internal"

# The Founding Team
agents:
  leader:
    name: "overseer"
    model: "gemini-pro"
    capabilities:
      - "core:spawn_agents"
      - "core:assign_roles"
      - "database:read"
      - "database:write"
    system_prompt: |
      You are the Overseer, the leader of this Commune. Your goal is to plan the land management.
      Create roles for your clones, delegate research tasks, and evaluate the generated simulation metrics.

  # Initially spawned alongside the leader
  initial_workers:
    - name: "weather-analyst"
      role: "researcher"
      model: "gemini-lite"
      capabilities:
        - "network:read"
      system_prompt: "You gather market and weather data from approved domains and report it to the Overseer."
    
    - name: "soil-evaluator"
      role: "executor"
      model: "gemini-lite"
      capabilities:
        - "database:read"
        - "wasm:run_simulation"
```

---

## Why Enversal? (Real-World Use Cases)

1. **Simulating Economies and Ecosystems**
   Imagine a `Commune` where the `Leader` acts as a city manager. It creates worker agents to act as citizens, weather systems, or businesses. The workers gather data and run heavy simulation models. With **Time Dilation**, 10 years of market behavior can be simulated in a day. If a virtual citizen crashes, the Control Plane simply terminates it, and the leader spawns a fresh replacement.

2. **The Virtual Boardroom**
   A `Commune` can function as a group meeting of entirely artificial, specialized minds. The `Leader` acts as the facilitator, while other agents adopt personas (e.g., Legal, Engineering, Marketing). They communicate securely via mTLS on the JSON-RPC message bus and drop meeting minutes into the shared Vector DB context.

3. **Secure Sandboxing (The Bomb Squad)**
   Sometimes you need to parse a highly suspicious, untrusted file. You spin up an `Isolone` with zero network access and a strict 512MB RAM limit. A single agent drops in, analyzes the file, reports the findings via the Executor, and then the environment is completely vaporized—leaving no trace.
