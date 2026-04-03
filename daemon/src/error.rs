use thiserror::Error;
use tonic::Status;

#[derive(Error, Debug)]
pub enum DaemonError {
    #[error("Environment not found: {0}")]
    NotFound(String),

    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    #[error("Tool error: {0}")]
    ToolError(String),

    #[error("Sandbox error: {0}")]
    SandboxError(String),

    #[error("Reasoning engine error: {0}")]
    ReasoningError(String),

    #[error("Internal state error: {0}")]
    InternalSyncError(String),
}

impl From<DaemonError> for Status {
    fn from(error: DaemonError) -> Self {
        match error {
            DaemonError::NotFound(msg) => Status::not_found(msg),
            DaemonError::InvalidManifest(msg) => Status::invalid_argument(msg),
            DaemonError::ToolError(msg) => Status::internal(msg),
            DaemonError::SandboxError(msg) => Status::internal(msg),
            DaemonError::ReasoningError(msg) => Status::internal(msg),
            DaemonError::InternalSyncError(msg) => Status::internal(msg),
        }
    }
}

pub type DaemonResult<T> = Result<T, DaemonError>;
