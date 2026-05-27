#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDiff {
    OnlyLocal,
    OnlyRemote,
    Both { local_newer: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncAction {
    UploadToRemote,
    DownloadToLocal,
    DeleteLocal,
    DeleteRemote,
    Skip,
}

impl SyncAction {
    pub fn label(self) -> &'static str {
        match self {
            SyncAction::UploadToRemote => "subir",
            SyncAction::DownloadToLocal => "bajar",
            SyncAction::DeleteLocal => "eliminar local",
            SyncAction::DeleteRemote => "eliminar remoto",
            SyncAction::Skip => "omitir",
        }
    }
}

impl std::fmt::Display for SyncAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

impl SyncAction {
    pub fn all() -> &'static [SyncAction] {
        &[
            SyncAction::UploadToRemote,
            SyncAction::DownloadToLocal,
            SyncAction::DeleteLocal,
            SyncAction::DeleteRemote,
            SyncAction::Skip,
        ]
    }

    pub fn default_for(diff: SyncDiff) -> Self {
        match diff {
            SyncDiff::OnlyLocal => SyncAction::UploadToRemote,
            SyncDiff::OnlyRemote => SyncAction::DownloadToLocal,
            SyncDiff::Both { local_newer: true } => SyncAction::UploadToRemote,
            SyncDiff::Both { local_newer: false } => SyncAction::DownloadToLocal,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SyncEntry {
    pub name: String,
    pub diff: SyncDiff,
    pub action: SyncAction,
}

#[derive(Debug, Clone, Default)]
pub struct SyncState {
    pub entries: Vec<SyncEntry>,
    pub analyzing: bool,
}
