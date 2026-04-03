//! The fundamental domain models and abstractions for building Enversal environments for real and simulated AI agents.
//! This crate contains pure, platform-independent logic governing agents and environments.

/// Agent identity, roles, and core behavior schemas.
pub mod agent;
/// Logical bounds containing agents (Isolones and Communes) and their lifecycle methods.
pub mod environment;
/// Hardware and memory thresholds assigned to security boundaries.
pub mod limits;
pub mod manifest;

pub use agent::{Agent, AgentConfig, AgentId, AgentRole};
pub use environment::{Commune, Environment, Isolone};
