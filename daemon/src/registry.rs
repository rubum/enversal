use crate::control::TelemetryEvent;
use enversal_core::environment::{Commune, Isolone};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Represents the core stateful service running the Control Plane.
/// Holds an initialized, contained environment map logic.
#[non_exhaustive]
pub enum ActiveEnv {
    Isolone(Isolone),
    Commune(Commune),
}

/// A wrapper holding the environment constraint state and its live telemetry broadcaster.
pub struct EnvState {
    pub active_env: ActiveEnv,
    pub telemetry_tx: tokio::sync::broadcast::Sender<TelemetryEvent>,
}

/// Centralized registry for managing active environments and agent runtimes.
///
/// This struct wraps the shared state and provides thread-safe accessors
/// to avoid direct RwLock manipulation throughout the codebase.
#[derive(Clone, Default)]
pub struct EnvironmentRegistry {
    /// In-memory persistence of active Isolones and Communes.
    pub active_environments: Arc<RwLock<HashMap<String, EnvState>>>,
    /// Global registry tracking provisioned languages (Python, Node) per agent.
    pub runtime_registry: Arc<RwLock<HashMap<Uuid, HashMap<String, PathBuf>>>>,
}

impl EnvironmentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn insert_env(&self, id: String, state: EnvState) {
        let mut map = self.active_environments.write().await;
        map.insert(id, state);
    }

    pub async fn get_env_ids(&self) -> Vec<String> {
        let map = self.active_environments.read().await;
        map.keys().cloned().collect()
    }

    pub async fn remove_env(&self, id: &str) {
        let mut map = self.active_environments.write().await;
        map.remove(id);
    }
}
