use uuid::Uuid;

use crate::error::AppErrorMsg;
use crate::models::{
    ftp_entry::FtpEntry, ftp_task::FtpTaskResult, panel::PanelKind, sort::SortKey,
};

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
    RemoteCrumbClicked(String),
    RemoteSortBy(SortKey),
    RemoteFilterChanged(String),

    // Local navigation
    LocalEntryOpened(FtpEntry),
    LocalDirLoaded(Result<(String, Vec<FtpEntry>), AppErrorMsg>),
    LocalCrumbClicked(String),
    LocalSortBy(SortKey),
    LocalFilterChanged(String),

    // File operations
    MkdirPressed(PanelKind),
    RenamePressed(PanelKind),
    DeletePressed(PanelKind),
    PromptValueChanged(String),
    PromptSubmit,
    PromptCancel,
    LocalOpFinished(Result<(), AppErrorMsg>),
    RemoteOpFinished(FtpTaskResult<()>),

    // Context menu
    ContextMenuOpened {
        panel: PanelKind,
        target: String,
    },
    ContextMenuClosed,

    // Selection
    LocalFileSelected {
        name: String,
        shift: bool,
        ctrl: bool,
    },
    RemoteFileSelected {
        name: String,
        shift: bool,
        ctrl: bool,
    },
    LocalSelectAll,
    RemoteSelectAll,
    ClearSelection(PanelKind),
    PanelFocused(PanelKind),
    ModifiersChanged(iced::keyboard::Modifiers),
    KeyPressed {
        key: iced::keyboard::Key,
        modifiers: iced::keyboard::Modifiers,
    },

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
