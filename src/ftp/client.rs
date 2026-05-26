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

pub async fn mkdir(
    manager: &FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    name: String,
) -> FtpResponse<()> {
    manager.mkdir(conn, remote_dir, name).await
}

pub async fn rename(
    manager: &FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    old: String,
    new: String,
) -> FtpResponse<()> {
    manager.rename(conn, remote_dir, old, new).await
}

pub async fn remove(
    manager: &FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    name: String,
    is_dir: bool,
) -> FtpResponse<()> {
    manager.remove(conn, remote_dir, name, is_dir).await
}

pub fn log_ftp_error(context: &str, err: &AppError) {
    error!(%context, error = %err, "FTP operation failed");
}
