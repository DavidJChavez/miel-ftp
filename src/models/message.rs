use std::path::PathBuf;

use uuid::Uuid;

use crate::error::AppErrorMsg;
use crate::models::{
    connection::Protocol,
    ftp_entry::FtpEntry,
    ftp_task::FtpTaskResult,
    panel::PanelKind,
    sort::SortKey,
    sync::{SyncAction, SyncEntry},
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
    ConnectionFormModeChanged(bool),
    ConnectionFormProtocolChanged(Protocol),
    ConnectionFormAcceptInvalidCertsChanged(bool),
    ConnectionFormSave,
    ConnectionFormCancel,
    ConnectionFormDelete,
    ConnectionDeleted,

    // Quickconnect
    ToggleQuickconnect,
    QuickconnectHostChanged(String),
    QuickconnectPortChanged(String),
    QuickconnectUsernameChanged(String),
    QuickconnectPasswordChanged(String),
    QuickconnectConnect,

    // Bookmarks
    BookmarkAdd,
    BookmarkRemove(String),
    BookmarkNavigate(String),

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
    EditRemoteFile(String),
    ReuploadRemoteEdit(PathBuf),

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
    ResumeTransfer(Uuid),
    ClearCompletedTransfers,
    ToggleTransferQueue,

    LocalGoUp,
    LocalRefresh,
    RemoteGoUp,
    RemoteRefresh,

    // FTP log
    ToggleFtpLog,
    ClearFtpLog,

    // Filtros globales
    ToggleSettingsModal,
    SettingsHideSystemToggled(bool),
    SettingsShowDotfilesToggled(bool),
    SettingsCustomAdded(String),
    SettingsCustomRemoved(String),
    SettingsCustomDraftChanged(String),
    SettingsBandwidthDraftChanged(String),
    SettingsMaxConcurrentChanged(String),

    // Sync panel
    OpenSyncPanel,
    CloseSyncPanel,
    SyncAnalyzeResult(Vec<SyncEntry>),
    SyncEntryActionChanged(String, SyncAction),
    SyncApply,

    // Remote edit
    RemoteEditDownloaded {
        temp_path: PathBuf,
        remote_dir: String,
        filename: String,
    },
    RemoteEditFailed(AppErrorMsg),
    RemoteEditUploaded(PathBuf),

    // Toasts
    ToastTick,
    DismissToast(Uuid),

    // Drag & drop
    FileDropped(PathBuf),
    FilesHoverEntered,
    FilesHoverLeft,

    /// Completado de tarea en segundo plano sin efecto en el estado.
    Noop,
}
