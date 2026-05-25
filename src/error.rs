use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("FTP error: {0}")]
    Ftp(#[from] suppaftp::FtpError),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No connection selected")]
    NoConnection,
}
