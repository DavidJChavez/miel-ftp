use std::path::PathBuf;

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

pub async fn upload(
    manager: &FtpSessionManager,
    conn: Connection,
    local_path: PathBuf,
    remote_path: String,
) -> FtpResponse<()> {
    manager.upload(conn, local_path, remote_path).await
}

pub async fn download(
    manager: &FtpSessionManager,
    conn: Connection,
    remote_path: String,
    filename: String,
    local_dir: PathBuf,
) -> FtpResponse<()> {
    manager
        .download(conn, remote_path, filename, local_dir)
        .await
}

pub fn log_ftp_error(context: &str, err: &AppError) {
    error!(%context, error = %err, "FTP operation failed");
}
