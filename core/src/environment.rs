//! Defines the physical boundaries and rules for Enversal environments.

use crate::agent::{Agent, AgentConfig, AgentId};
use crate::limits::ResourceLimits;
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for an environment cluster.
pub type EnvId = Uuid;

/// Common errors that occur during lifecycle management of Agents within Environments.
#[derive(Debug)]
pub enum EnvError {
    /// The Environment has exhausted its configured capacity limits.
    CapacityExceeded,
    /// Targeted agent was not found inside the memory bus of this environment.
    AgentNotFound,
}

/// The generic bounding interface for all Enversal micro-universes.
/// This guarantees that the Control Plane can interact homogeneously across different architectures.
pub trait Environment {
    /// Returns the cryptographically secure universally unique ID representing this environment.
    fn id(&self) -> EnvId;
    /// Attempts to spawn a new agent into this environment given the sandbox limits.
    fn spawn_agent(&mut self, name: &str, config: AgentConfig) -> Result<AgentId, EnvError>;
    /// Immediately halts and garbage-collects an agent from the environment's sandbox limit accounting.
    fn terminate_agent(&mut self, id: AgentId) -> Result<(), EnvError>;
    /// Returns the immutable physics profile (Memory, CPU) of this environment.
    fn resource_limits(&self) -> &ResourceLimits;
}

/// Isolone: Isolated environment for a single Agent.
///
/// The Isolone acts as the "Hermit's Sandbox". It is a strictly restrictive universe
/// built for atomic, solitary tasks without risking broader data contamination.
/// An Isolone will safely reject any attempt to spawn more than one agent concurrently.
pub struct Isolone {
    /// Environment unique signature.
    pub id: EnvId,
    /// The purpose of this solitary environment.
    pub goal: String,
    /// The hard resource limits mapped to the native Executor.
    pub limits: ResourceLimits,
    /// The solitary agent process.
    pub agent: Option<Agent>,
}

impl Isolone {
    /// Creates a new empty Isolone with specific sandboxed laws.
    pub fn new(goal: impl Into<String>, limits: ResourceLimits) -> Self {
        Self {
            id: Uuid::new_v4(),
            goal: goal.into(),
            limits,
            agent: None,
        }
    }
}

impl Environment for Isolone {
    fn id(&self) -> EnvId {
        self.id
    }

    fn spawn_agent(&mut self, name: &str, config: AgentConfig) -> Result<AgentId, EnvError> {
        if self.agent.is_some() {
            return Err(EnvError::CapacityExceeded);
        }
        let new_agent = Agent::new(name, config);
        let id = new_agent.id;
        self.agent = Some(new_agent);
        Ok(id)
    }

    fn terminate_agent(&mut self, id: AgentId) -> Result<(), EnvError> {
        match &self.agent {
            Some(a) if a.id == id => {
                self.agent = None;
                Ok(())
            }
            _ => Err(EnvError::AgentNotFound),
        }
    }

    fn resource_limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

/// Commune: Shared, highly collaborative environment capable of hosting multiple agents.
///
/// Communes represent the "Thriving Society" of Enversal. Resources, context,
/// and goals are shared fluidly among multiple specialized entities. It expects
/// one agent to assume the Leader role to orchestrate execution among Worker agents.
pub struct Commune {
    /// Environment unique signature.
    pub id: EnvId,
    /// The collective objective guiding the local society.
    pub goal: String,
    /// The collective cap on physical host resources.
    pub limits: ResourceLimits,
    /// The Active Leader ID assigned to orchestrate standard events.
    pub leader_id: Option<AgentId>,
    /// A robust hashmap defining all currently alive Agents in the Commune.
    pub agents: HashMap<AgentId, Agent>,
}

impl Commune {
    /// Creates a scalable, multi-agent environment with communal sandboxing limits.
    pub fn new(goal: impl Into<String>, limits: ResourceLimits) -> Self {
        Self {
            id: Uuid::new_v4(),
            goal: goal.into(),
            limits,
            leader_id: None,
            agents: HashMap::new(),
        }
    }
}

impl Environment for Commune {
    fn id(&self) -> EnvId {
        self.id
    }

    fn spawn_agent(&mut self, name: &str, config: AgentConfig) -> Result<AgentId, EnvError> {
        let new_agent = Agent::new(name, config);
        let id = new_agent.id;
        self.agents.insert(id, new_agent);
        Ok(id)
    }

    fn terminate_agent(&mut self, id: AgentId) -> Result<(), EnvError> {
        if self.agents.remove(&id).is_some() {
            Ok(())
        } else {
            Err(EnvError::AgentNotFound)
        }
    }

    fn resource_limits(&self) -> &ResourceLimits {
        &self.limits
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::AgentRole;
    use std::collections::HashSet;

    fn dummy_config() -> AgentConfig {
        AgentConfig {
            role: AgentRole::Worker("tester".to_string()),
            model: "test-model".to_string(),
            allowed_tools: HashSet::new(),
        }
    }

    #[test]
    fn test_isolone_spawn_limit() {
        let mut isolone = Isolone::new("Test goal", ResourceLimits::default());

        let id1_res = isolone.spawn_agent("agent-1", dummy_config());
        assert!(id1_res.is_ok(), "First spawn should succeed");

        let id2_res = isolone.spawn_agent("agent-2", dummy_config());
        assert!(
            matches!(id2_res, Err(EnvError::CapacityExceeded)),
            "Second spawn must fail in Isolone"
        );
    }

    #[test]
    fn test_commune_spawns_multiple() {
        let mut commune = Commune::new("Test group", ResourceLimits::default());

        let id1 = commune
            .spawn_agent("agent-1", dummy_config())
            .expect("Spawn 1 failed");
        let id2 = commune
            .spawn_agent("agent-2", dummy_config())
            .expect("Spawn 2 failed");

        assert_eq!(commune.agents.len(), 2);
        assert!(commune.agents.contains_key(&id1));
        assert!(commune.agents.contains_key(&id2));
    }
}
