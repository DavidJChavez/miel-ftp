use uuid::Uuid;

use crate::error::AppErrorMsg;
use crate::models::{ftp_entry::FtpEntry, ftp_task::FtpTaskResult};

#[derive(Debug, Clone)]
pub enum Message {
    // Sidebar / Site Manager
    ConnectionSelected(Uuid),
    AddConnectionPressed,
    EditConnectionPressed(Uuid),
    ConnectionFormNameChanged(String),
    ConnectionFormHostChanged(String),
    ConnectionFormPortChanged(String),
    ConnectionFormUsernameChanged(String),
    ConnectionFormPasswordChanged(String),
    ConnectionFormSave,
    ConnectionFormCancel,
    ConnectionFormDelete,
    ConnectionDeleted,

    // FTP Connection
    ConnectPressed,
    Disconnected,
    ConnectResult(FtpTaskResult<()>),

    // Remote navigation
    RemoteEntryOpened(FtpEntry),
    RemoteDirLoaded(FtpTaskResult<(String, Vec<FtpEntry>)>),

    // Local navigation
    LocalEntryOpened(FtpEntry),
    LocalDirLoaded(Result<(String, Vec<FtpEntry>), AppErrorMsg>),

    // Transfer
    UploadPressed,
    DownloadPressed,
    TransferProgress {
        id: Uuid,
        bytes: u64,
        total: Option<u64>,
    },
    TransferFinished {
        id: Uuid,
        result: FtpTaskResult<()>,
    },
    CancelTransfer(Uuid),
    RetryTransfer(Uuid),
    ClearCompletedTransfers,
    ToggleTransferQueue,
    LocalFileSelected(String),
    RemoteFileSelected(String),

    LocalGoUp,
    LocalRefresh,
    RemoteGoUp,
    RemoteRefresh,

    // FTP log
    ToggleFtpLog,
    ClearFtpLog,

    /// Completado de tarea en segundo plano sin efecto en el estado.
    Noop,
}
