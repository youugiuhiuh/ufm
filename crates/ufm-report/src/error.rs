use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReportError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Export formatting error: {0}")]
    Format(String),
}
