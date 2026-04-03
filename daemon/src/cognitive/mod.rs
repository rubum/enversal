use crate::control::TelemetryEvent;
use crate::registry::{ActiveEnv, EnvironmentRegistry};
use crate::tools::ToolDispatcher;
use brain::prompt::{
    parse_evaluation, parse_finish, parse_plan, EVALUATOR_SYSTEM_INSTRUCTION,
    MASTER_SYSTEM_INSTRUCTION,
};
use brain::{AgentContext, CognitiveEngine, ReasoningOutput};
use owo_colors::OwoColorize;
use sandbox::Executor;
use std::sync::Arc;
use uuid::Uuid;

/// The possible states of an environment's reasoning loop.
#[derive(Debug, Clone, PartialEq)]
enum LoopState {
    /// Normal task execution.
    Executing,
    /// Auditor is verifying the task results.
    Evaluating,
}

/// Spawns the asynchronous reasoning loop for an environment.
pub fn spawn_loop(
    env_id: String,
    registry: EnvironmentRegistry,
    brain: Arc<dyn CognitiveEngine + Send + Sync>,
    executor: Arc<dyn Executor + Send + Sync>,
    telemetry_tx: tokio::sync::broadcast::Sender<TelemetryEvent>,
) {
    let dispatcher = ToolDispatcher::new();
    let env_id_inner = env_id.clone();

    tokio::spawn(async move {
        println!(
            "Control Plane: Starting cognitive loop for Environment {}",
            env_id_inner
        );
        let mut observations = vec![
            "System Snapshot: Enversal OS Environment Initialized. Sandbox is active. Filesystem is ready. Tools available: sandbox_exec, provision_env, git_clone, npm_install.".to_string()
        ];
        let mut loop_state = LoopState::Executing;
        let mut plan_submitted = false;

        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            let (context, initial_snapshot) = {
                let map = registry.active_environments.read().await;
                if let Some(state_entry) = map.get(&env_id_inner) {
                    let env = &state_entry.active_env;
                    let limits = match env {
                        ActiveEnv::Isolone(i) => &i.limits,
                        ActiveEnv::Commune(c) => &c.limits,
                    };

                    let read_paths = limits
                        .allowed_read_paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");
                    let write_paths = limits
                        .allowed_write_paths
                        .iter()
                        .map(|p| p.display().to_string())
                        .collect::<Vec<_>>()
                        .join(", ");

                    let snapshot = format!(
                        "System Snapshot: Enversal OS Environment Initialized. \
                         Sandbox is active. Filesystem is ready. \
                         READ ACCESS: [{}] | WRITE ACCESS: [{}] | \
                         Tools available: sandbox_exec, provision_env, git_clone, npm_install.",
                        if read_paths.is_empty() {
                            "None"
                        } else {
                            &read_paths
                        },
                        if write_paths.is_empty() {
                            "None"
                        } else {
                            &write_paths
                        }
                    );

                    let ctx = match env {
                        ActiveEnv::Isolone(i) => {
                            let system_prompt = if loop_state == LoopState::Evaluating {
                                EVALUATOR_SYSTEM_INSTRUCTION.to_string()
                            } else {
                                format!(
                                    "{}\n\n### YOUR SPECIFIC GOAL\n{}",
                                    MASTER_SYSTEM_INSTRUCTION, i.goal
                                )
                            };
                            Some(AgentContext {
                                agent_id: i.id,
                                model: i
                                    .agent
                                    .as_ref()
                                    .map(|a| a.config.model.clone())
                                    .unwrap_or_else(|| "gemini-2.1-flash".into()),
                                system_prompt,
                                recent_observations: observations.clone(),
                            })
                        }
                        ActiveEnv::Commune(c) => {
                            let system_prompt = if loop_state == LoopState::Evaluating {
                                EVALUATOR_SYSTEM_INSTRUCTION.to_string()
                            } else {
                                format!(
                                    "{}\n\n### YOUR SPECIFIC GOAL\n{}",
                                    MASTER_SYSTEM_INSTRUCTION, c.goal
                                )
                            };
                            let model = c
                                .leader_id
                                .and_then(|id| c.agents.get(&id))
                                .map(|a| a.config.model.clone())
                                .unwrap_or_else(|| "gemini-2.1-flash".into());
                            Some(AgentContext {
                                agent_id: Uuid::new_v4(),
                                model,
                                system_prompt,
                                recent_observations: observations.clone(),
                            })
                        }
                    };
                    (ctx, Some(snapshot))
                } else {
                    (None, None)
                }
            };

            // Inject the snapshot into observations if it's the first run
            if observations.len() == 1 && observations[0].contains("Environment Initialized") {
                if let Some(s) = initial_snapshot {
                    observations[0] = s;
                }
            }

            if let Some(ctx) = context {
                match brain.reason(&ctx).await {
                    Ok(ReasoningOutput::Message(msg)) => {
                        let role_label = if loop_state == LoopState::Evaluating {
                            "Auditor"
                        } else {
                            "Agent"
                        };
                        let _ = telemetry_tx.send(TelemetryEvent {
                            event_type: "Reasoning".into(),
                            content: msg.clone(),
                            agent_name: role_label.into(),
                        });

                        println!("\n{}", "--- Reasoning ---".cyan().bold());
                        termimad::print_text(&msg);
                        println!("{}\n", "-----------------".cyan().bold());
                        observations.push(format!("Self: {}", msg));

                        // DETECTION: Mandatory Plan
                        if loop_state == LoopState::Executing && !plan_submitted {
                            if let Some(plan) = parse_plan(&msg) {
                                println!("\n{}", "--- Agent Plan ---".purple().bold());
                                termimad::print_text(&plan);
                                println!("{}\n", "------------------".purple().bold());

                                let _ = telemetry_tx.send(TelemetryEvent {
                                    event_type: "Plan".into(),
                                    content: plan,
                                    agent_name: "Agent".into(),
                                });
                                plan_submitted = true;
                            } else {
                                // If the agent doesn't provide a plan in its first reasoning turn, remind it.
                                // We only do this if no tool call was made.
                                observations.push(
                                    "System: Please provide your <plan> before taking actions."
                                        .into(),
                                );
                            }
                        }

                        // DETECTION: Completion Tag
                        if loop_state == LoopState::Executing {
                            if let Some(report) = parse_finish(&msg) {
                                println!(
                                    "{}",
                                    "--- Task Completion Signal Detected ---".green().bold()
                                );
                                let _ = telemetry_tx.send(TelemetryEvent {
                                    event_type: "System".into(),
                                    content: format!(
                                        "Agent finished work. Transitioning to EVALUATION phase."
                                    ),
                                    agent_name: "Control Plane".into(),
                                });
                                observations.push(format!("Final Report: {}", report));
                                loop_state = LoopState::Evaluating;
                                continue;
                            }
                        }

                        // DETECTION: Evaluation Approval/Rejection
                        if loop_state == LoopState::Evaluating {
                            match parse_evaluation(&msg) {
                                Some(Ok(_)) => {
                                    println!(
                                        "{}",
                                        "--- TASK APPROVED BY AUDITOR ---".green().bold()
                                    );
                                    let _ = telemetry_tx.send(TelemetryEvent {
                                        event_type: "System".into(),
                                        content: "Auditor approved the task. Shutting down environment as requested.".into(),
                                        agent_name: "Control Plane".into(),
                                    });
                                    registry.remove_env(&env_id_inner).await;
                                    break;
                                }
                                Some(Err(error)) => {
                                    println!("{}", "--- TASK REJECTED BY AUDITOR ---".red().bold());
                                    println!("{} {}", "REASON:".red().bold(), error);
                                    let _ = telemetry_tx.send(TelemetryEvent {
                                        event_type: "System".into(),
                                        content: format!("Auditor REJECTED task: {}", error),
                                        agent_name: "Control Plane".into(),
                                    });
                                    observations.push(format!("Auditor Correction: {}", error));
                                    loop_state = LoopState::Executing;
                                    continue;
                                }
                                None => {
                                    // Evaluator is still reasoning/using tools
                                }
                            }
                        }
                    }
                    Ok(ReasoningOutput::ToolCall(call)) => {
                        let role_label = if loop_state == LoopState::Evaluating {
                            "Auditor"
                        } else {
                            "Agent"
                        };
                        println!(
                            "{} requested TOOL: {} with ARGS: {}",
                            role_label.yellow().bold(),
                            call.tool_name.green(),
                            call.arguments.to_string().dimmed()
                        );

                        let _ = telemetry_tx.send(TelemetryEvent {
                            event_type: "Tool Request".into(),
                            content: format!(
                                "Tool: {} with args: {}",
                                call.tool_name, call.arguments
                            ),
                            agent_name: role_label.into(),
                        });

                        match dispatcher
                            .dispatch(
                                &call.tool_name,
                                &call.arguments,
                                &ctx,
                                &env_id_inner,
                                &registry,
                                &executor,
                            )
                            .await
                        {
                            Ok(output) => {
                                let _ = telemetry_tx.send(TelemetryEvent {
                                    event_type: "Tool Result".into(),
                                    content: output.clone(),
                                    agent_name: "OS".into(),
                                });
                                println!("{} {}", "Tool Output:".green().bold(), output);
                                observations
                                    .push(format!("Tool result ({}): {}", call.tool_name, output));
                            }
                            Err(e) => {
                                let _ = telemetry_tx.send(TelemetryEvent {
                                    event_type: "Error".into(),
                                    content: e.to_string(),
                                    agent_name: "OS".into(),
                                });
                                eprintln!("{} {}", "Tool Execution Failed:".red().bold(), e);
                                observations
                                    .push(format!("Tool ERROR ({}): {}", call.tool_name, e));
                            }
                        }
                    }
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("{} {:?}", "Reasoning Engine Failed:".red().bold(), e);
                        observations.push(format!("System Error: {}", e));
                    }
                }
            } else {
                break;
            }

            if observations.len() > 10 {
                observations.remove(0);
            }
        }
        println!(
            "Control Plane: Terminating cognitive loop for Environment {}",
            env_id_inner
        );
    });
}
