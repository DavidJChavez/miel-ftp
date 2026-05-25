use uuid::Uuid;

use crate::models::ftp_entry::FtpEntry;

#[derive(Debug, Clone)]
pub enum Message {
    // Sidebar
    ConnectionSelected(Uuid),
    AddConnectionPressed,

    // FTP Connection
    ConnectPressed,
    Disconnected,
    ConnectResult(Result<(), String>),

    // Remote navigation
    RemoteEntryOpened(FtpEntry),
    RemoteDirLoaded(Result<(String, Vec<FtpEntry>), String>),

    // Local navigation
    LocalEntryOpened(FtpEntry),
    LocalDirLoaded(Result<(String, Vec<FtpEntry>), String>),

    // Transfer
    UploadPressed,
    DownloadPressed,
    TransferProgress(u64),
    TransferComplete(String),
    TransferError(String),
    LocalFileSelected(String),
    RemoteFileSelected(String),

    LocalGoUp,
    LocalRefresh,
    RemoteGoUp,
    RemoteRefresh,
}
