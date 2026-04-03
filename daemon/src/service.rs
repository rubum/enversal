use crate::control::environment_control_server::EnvironmentControl;
use crate::control::{
    AttachTelemetryRequest, DeployRequest, DeployResponse, EnvInfo, InspectRequest,
    InspectResponse, ListRequest, ListResponse, StatusRequest, StatusResponse, TelemetryEvent,
};
use crate::registry::{ActiveEnv, EnvState, EnvironmentRegistry};
use brain::{CognitiveEngine, GeminiEngine, OllamaEngine};
use enversal_core::environment::{Commune, Isolone};
use enversal_core::manifest::EnversalManifest;
use sandbox::Executor;
use std::sync::Arc;
use tokio_stream::StreamExt;
use tonic::{Request, Response, Status};
use uuid::Uuid;

/// The Control Plane orchestration engine.
#[derive(Clone)]
pub struct DaemonService {
    pub registry: EnvironmentRegistry,
    pub executor: Arc<dyn Executor + Send + Sync>,
}

impl DaemonService {
    pub fn new(
        executor: Arc<dyn Executor + Send + Sync>,
    ) -> Self {
        Self {
            registry: EnvironmentRegistry::new(),
            executor,
        }
    }
}

#[tonic::async_trait]
impl EnvironmentControl for DaemonService {
    async fn deploy_environment(
        &self,
        request: Request<DeployRequest>,
    ) -> Result<Response<DeployResponse>, Status> {
        let req = request.into_inner();
        println!(
            "Control Plane: Received request to provision [{}] environment",
            req.env_type
        );

        let manifest: EnversalManifest = serde_json::from_str(&req.blueprint_json)
            .map_err(|e| Status::invalid_argument(format!("Invalid JSON manifest: {}", e)))?;

        let base_limits = enversal_core::limits::ResourceLimits {
            max_cpu_cores: manifest.resources.cpu_cores,
            max_ram_mb: manifest.resources.ram_mb,
            db_access: manifest.resources.db_access,
            allowed_network_domains: manifest.resources.network.allowed_domains.clone(),
            allowed_read_paths: manifest
                .resources
                .filesystem
                .allowed_read_paths
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| !p.components().any(|c| c == std::path::Component::ParentDir))
                .collect(),
            allowed_write_paths: manifest
                .resources
                .filesystem
                .allowed_write_paths
                .iter()
                .map(std::path::PathBuf::from)
                .filter(|p| !p.components().any(|c| c == std::path::Component::ParentDir))
                .collect(),
        };

        // Determine AI Provider
        let brain: Arc<dyn CognitiveEngine + Send + Sync> = match manifest.environment.ai_provider.to_lowercase().as_str() {
            "ollama" => {
                let host = std::env::var("OLLAMA_HOST").ok();
                Arc::new(OllamaEngine::new(host))
            }
            "gemini" | "" => {
                let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| "mock-key".to_string());
                Arc::new(GeminiEngine::new(api_key))
            }
            unknown => {
                return Err(Status::invalid_argument(format!("Unsupported AI Provider: {}", unknown)));
            }
        };

        let env_id = Uuid::new_v4().to_string();
        let mut active_env = if req.env_type.to_lowercase() == "commune" {
            ActiveEnv::Commune(Commune::new(manifest.environment.goal.clone(), base_limits))
        } else {
            ActiveEnv::Isolone(Isolone::new(manifest.environment.goal.clone(), base_limits))
        };

        match &mut active_env {
            ActiveEnv::Isolone(i) => {
                let spec = &manifest.agents.leader;
                let agent_config = enversal_core::agent::AgentConfig {
                    role: enversal_core::agent::AgentRole::Leader,
                    model: spec.model.clone(),
                    allowed_tools: spec.capabilities.clone().into_iter().collect(),
                };
                i.agent = Some(enversal_core::agent::Agent::new(
                    spec.name.clone(),
                    agent_config,
                ));
            }
            ActiveEnv::Commune(c) => {
                let leader_spec = &manifest.agents.leader;
                let leader_config = enversal_core::agent::AgentConfig {
                    role: enversal_core::agent::AgentRole::Leader,
                    model: leader_spec.model.clone(),
                    allowed_tools: leader_spec.capabilities.clone().into_iter().collect(),
                };
                let leader =
                    enversal_core::agent::Agent::new(leader_spec.name.clone(), leader_config);
                c.leader_id = Some(leader.id);
                c.agents.insert(leader.id, leader);

                if let Some(workers) = &manifest.agents.initial_workers {
                    for w_spec in workers {
                        let worker_config = enversal_core::agent::AgentConfig {
                            role: enversal_core::agent::AgentRole::Worker(
                                w_spec.role.clone().unwrap_or_else(|| "worker".to_string()),
                            ),
                            model: w_spec.model.clone(),
                            allowed_tools: w_spec.capabilities.clone().into_iter().collect(),
                        };
                        let worker =
                            enversal_core::agent::Agent::new(w_spec.name.clone(), worker_config);
                        c.agents.insert(worker.id, worker);
                    }
                }
            }
        }

        let (telemetry_tx, _) = tokio::sync::broadcast::channel(100);

        self.registry.insert_env(
            env_id.clone(),
            EnvState {
                active_env,
                telemetry_tx: telemetry_tx.clone(),
            },
        ).await;

        crate::cognitive::spawn_loop(
            env_id.clone(),
            self.registry.clone(),
            brain,
            self.executor.clone(),
            telemetry_tx,
        );

        Ok(Response::new(DeployResponse {
            env_id,
            success: true,
            message: format!(
                "Successfully provisioned {} environment via Control Plane.",
                req.env_type
            ),
        }))
    }

    type AttachTelemetryStream = std::pin::Pin<
        Box<dyn tokio_stream::Stream<Item = Result<TelemetryEvent, Status>> + Send + 'static>,
    >;

    async fn attach_telemetry(
        &self,
        request: Request<AttachTelemetryRequest>,
    ) -> Result<Response<Self::AttachTelemetryStream>, Status> {
        let req = request.into_inner();
        let map = self.registry.active_environments.read().await;

        if let Some(env_state) = map.get(&req.env_id) {
            let stream =
                tokio_stream::wrappers::BroadcastStream::new(env_state.telemetry_tx.subscribe());
            let out_stream = stream.map(|res| match res {
                Ok(evt) => Ok(evt),
                Err(_) => {
                    Ok(TelemetryEvent {
                        event_type: "System".into(),
                        content: "Telemetry stream lagged (messages dropped)".into(),
                        agent_name: "Control Plane".into(),
                    })
                }
            });
            Ok(Response::new(
                Box::pin(out_stream) as Self::AttachTelemetryStream
            ))
        } else {
            Err(Status::not_found("Environment ID not found or dead."))
        }
    }

    async fn get_status(
        &self,
        request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let req = request.into_inner();
        let map = self.registry.active_environments.read().await;
        let is_running = map.contains_key(&req.env_id);

        Ok(Response::new(StatusResponse {
            status: if is_running { "Running".into() } else { "NotFound".into() },
            active_agents: 3,
        }))
    }

    async fn list_environments(
        &self,
        _request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, Status> {
        let map = self.registry.active_environments.read().await;
        let mut envs = Vec::new();
        for (id, env_state) in map.iter() {
            let env = &env_state.active_env;
            let env_type = match env {
                ActiveEnv::Isolone(_) => "Isolone",
                ActiveEnv::Commune(_) => "Commune",
            };
            envs.push(EnvInfo {
                env_id: id.clone(),
                env_type: env_type.to_string(),
                status: "Running".to_string(),
            });
        }
        Ok(Response::new(ListResponse { environments: envs }))
    }

    async fn inspect_environment(
        &self,
        request: Request<InspectRequest>,
    ) -> Result<Response<InspectResponse>, Status> {
        let req = request.into_inner();
        let map = self.registry.active_environments.read().await;

        if let Some(env_state) = map.get(&req.env_id) {
            let env = &env_state.active_env;
            let json = match env {
                ActiveEnv::Isolone(i) => serde_json::json!({
                    "id": i.id.to_string(),
                    "type": "Isolone",
                    "goal": i.goal,
                    "limits": {
                        "max_cpu_cores": i.limits.max_cpu_cores,
                        "max_ram_mb": i.limits.max_ram_mb,
                        "db_access": i.limits.db_access,
                        "allowed_network_domains": i.limits.allowed_network_domains,
                    },
                    "agent": i.agent.as_ref().map(|a| serde_json::json!({
                        "id": a.id.to_string(),
                        "name": a.name,
                        "role": format!("{:?}", a.config.role),
                        "model": a.config.model,
                        "tools": a.config.allowed_tools,
                        "pid": a.current_pid,
                    }))
                }),
                ActiveEnv::Commune(c) => serde_json::json!({
                    "id": c.id.to_string(),
                    "type": "Commune",
                    "goal": c.goal,
                    "limits": {
                        "max_cpu_cores": c.limits.max_cpu_cores,
                        "max_ram_mb": c.limits.max_ram_mb,
                        "db_access": c.limits.db_access,
                        "allowed_network_domains": c.limits.allowed_network_domains,
                    },
                    "leader_id": c.leader_id.map(|id| id.to_string()),
                    "agents": c.agents.values().map(|a| serde_json::json!({
                        "id": a.id.to_string(),
                        "name": a.name,
                        "role": format!("{:?}", a.config.role),
                        "model": a.config.model,
                        "tools": a.config.allowed_tools,
                        "pid": a.current_pid,
                    })).collect::<Vec<_>>()
                }),
            };
            Ok(Response::new(InspectResponse {
                env_json: serde_json::to_string_pretty(&json).unwrap(),
                found: true,
            }))
        } else {
            Ok(Response::new(InspectResponse {
                env_json: String::new(),
                found: false,
            }))
        }
    }
}
