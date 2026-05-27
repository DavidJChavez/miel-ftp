use std::future::Future;

use crate::error::{AppError, AppResult};

pub async fn with_timeout<T, F>(duration: std::time::Duration, fut: F) -> AppResult<T>
where
    F: Future<Output = AppResult<T>>,
{
    match tokio::time::timeout(duration, fut).await {
        Ok(result) => result,
        Err(_) => Err(AppError::Timeout(duration.as_secs())),
    }
}

pub fn is_retryable(err: &AppError) -> bool {
    match err {
        AppError::Timeout(_) => true,
        AppError::Io(io) => matches!(
            io.kind(),
            std::io::ErrorKind::ConnectionReset
                | std::io::ErrorKind::ConnectionAborted
                | std::io::ErrorKind::UnexpectedEof
                | std::io::ErrorKind::TimedOut
                | std::io::ErrorKind::WouldBlock
        ),
        AppError::Ftp(ftp) => {
            let msg = ftp.to_string().to_lowercase();
            msg.contains("connection")
                || msg.contains("timeout")
                || msg.contains("broken pipe")
                || msg.contains("reset")
        }
        AppError::Validation(_)
        | AppError::Config(_)
        | AppError::Keyring(_)
        | AppError::Json(_)
        | AppError::NoConnection
        | AppError::Connection(_)
        | AppError::Cancelled => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_errors_are_not_retryable() {
        assert!(!is_retryable(&AppError::Validation("x".into())));
    }

    #[test]
    fn timeout_errors_are_retryable() {
        assert!(is_retryable(&AppError::Timeout(30)));
    }

    #[test]
    fn cancelled_errors_are_not_retryable() {
        assert!(!is_retryable(&AppError::Cancelled));
    }
}
