use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unsupported or unknown encoding: {0}")]
    UnsupportedEncoding(String),
    #[error("{0}")]
    Message(String),
}

pub type Result<T> = std::result::Result<T, AppError>;
