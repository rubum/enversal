//! Enversal Control Plane Daemon.
//!
//! This module represents the absolute Authority / Control Plane. It runs as a persistent
//! background service, exposing a gRPC API that the `cli` uses to orchestrate and spawn
//! universes (Communes, Isolones) from provided JSON manifests.

use control::environment_control_server::EnvironmentControlServer;
use sandbox::SeatbeltExecutor;
use std::sync::Arc;
use tonic::transport::Server;

pub mod cognitive;
pub mod error;
pub mod registry;
pub mod service;
pub mod tools;

/// Auto-generated server bindings from the proto file.
pub mod control {
    tonic::include_proto!("control");
}

use crate::service::DaemonService;

/// Entrypoint for the tonic gRPC supervisor server.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    // Initialize the Native OS Executor
    let executor = Arc::new(SeatbeltExecutor);

    // Initialize the Service (includes Registry)
    // The Brain (Gemini or Ollama) is now instantiated per-environment in the service layer
    let service = DaemonService::new(executor);

    println!(
        "Enversal Control Plane Daemon listening securely on {}",
        addr
    );

    Server::builder()
        .add_service(EnvironmentControlServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}
