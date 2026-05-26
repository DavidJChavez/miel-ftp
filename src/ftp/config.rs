use std::time::Duration;

/// Timeout para establecer la conexión TCP + handshake FTP inicial.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

/// Timeout por operación FTP (login, cwd, list, transfer, etc.).
pub const OPERATION_TIMEOUT: Duration = Duration::from_secs(60);

pub const MAX_RETRIES: u32 = 3;

pub const RETRY_BASE_DELAY: Duration = Duration::from_millis(400);

pub fn retry_delay(attempt: u32) -> Duration {
    RETRY_BASE_DELAY * (attempt + 1)
}
