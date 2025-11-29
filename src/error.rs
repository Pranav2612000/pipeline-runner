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
}
