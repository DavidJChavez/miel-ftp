use tracing::error;

use crate::error::AppError;
use crate::ftp::manager::FtpSessionManager;
use crate::models::connection::Connection;

pub struct FtpResponse<T> {
    pub result: crate::error::AppResult<T>,
    pub log: Vec<String>,
}

/// API legacy: cada llamada usa el gestor de sesiones (sin reconectar si la sesión sigue viva).
/// Preferir pasar `Arc<FtpSessionManager>` desde `State` en código nuevo.
pub async fn connect(manager: &FtpSessionManager, conn: Connection) -> FtpResponse<()> {
    manager.connect(conn).await
}

pub async fn list_dir(
    manager: &FtpSessionManager,
    conn: Connection,
    path: String,
) -> FtpResponse<(String, Vec<crate::models::ftp_entry::FtpEntry>)> {
    manager.list_dir(conn, path).await
}

pub fn log_ftp_error(context: &str, err: &AppError) {
    error!(%context, error = %err, "FTP operation failed");
}
