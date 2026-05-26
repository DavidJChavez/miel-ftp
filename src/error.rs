use std::sync::Arc;

use thiserror::Error;

pub type AppResult<T> = Result<T, AppError>;

/// Error clonable para mensajes de iced (`Message` debe ser `Clone`).
#[derive(Debug, Clone, Error)]
#[error("{0}")]
pub struct AppErrorMsg(pub Arc<AppError>);

impl From<AppError> for AppErrorMsg {
    fn from(err: AppError) -> Self {
        AppErrorMsg(Arc::new(err))
    }
}

impl From<AppErrorMsg> for String {
    fn from(msg: AppErrorMsg) -> Self {
        msg.0.to_string()
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("FTP error: {0}")]
    Ftp(#[from] suppaftp::FtpError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No connection selected")]
    NoConnection,

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Keyring error: {0}")]
    Keyring(String),

    #[error("Operation timed out after {0}s")]
    Timeout(u64),
}
