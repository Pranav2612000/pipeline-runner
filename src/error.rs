use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum PipelineError {
    #[error("Config file {0} could not be read: {1}")]
    ConfigFileNotReadable(String, String),

    #[error("Failed to parse config: {0}")]
    ParsingError(String),

    #[error("Failed to execute job {0}| Reason: {1}")]
    ExecutionError(String, String),

    #[error("Failed to start runtime {0}")]
    RuntimeError(String),

    #[error("Artifact save error: {0}")]
    ArtifactError(ArtifactError),
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ArtifactError {
    #[error("Artifact not found: {0}")]
    ArtifactNotFoundError(String),

    #[error("Artifact copy failed: {0}")]
    ArtifactCopyError(String),

    #[error("Artifact cleanup failed: {0}")]
    ArtifactCleanupError(String),
}
