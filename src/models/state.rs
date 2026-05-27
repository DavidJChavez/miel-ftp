use std::collections::HashSet;

use iced::keyboard::Modifiers;
use uuid::Uuid;

use crate::ftp::FtpSessionManager;
use crate::models::{
    connection::{Connection, ConnectionStatus},
    connection_form::ConnectionForm,
    context_menu::ContextMenu,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    panel::PanelKind,
    prompt::PromptDialog,
    quickconnect::QuickconnectForm,
    remote_edit::RemoteEdit,
    settings::AppSettings,
    sort::SortSpec,
    sync::SyncState,
    toast::{Toast, ToastKind},
    transfer::{TransferEntry, TransferStatus},
};

pub struct State {
    // Sidebar
    pub connections: Vec<Connection>,
    pub selected_connection: Option<Uuid>,
    pub quickconnect_expanded: bool,
    pub quickconnect: QuickconnectForm,

    // Site manager
    pub connection_form: Option<ConnectionForm>,

    // Remote dashboard
    pub remote_status: ConnectionStatus,
    pub remote_path: String,
    pub remote_entries: Vec<FtpEntry>,
    pub remote_loading: bool,
    pub selected_remote: HashSet<String>,
    pub last_clicked_remote: Option<String>,
    pub sort_remote: SortSpec,
    pub filter_remote: String,

    // Local dashboard
    pub local_path: String,
    pub local_entries: Vec<FtpEntry>,
    pub local_loading: bool,
    pub selected_local: HashSet<String>,
    pub last_clicked_local: Option<String>,
    pub sort_local: SortSpec,
    pub filter_local: String,

    // UI overlays
    pub context_menu: Option<ContextMenu>,
    pub prompt: Option<PromptDialog>,
    pub focused_panel: PanelKind,
    pub modifiers: Modifiers,
    pub drop_hover: bool,
    pub sync_panel: Option<SyncState>,

    // Transfer queue
    pub transfers: Vec<TransferEntry>,
    pub queue_panel_visible: bool,
    pub pending_remote_edits: Vec<RemoteEdit>,

    // UI feedback
    pub status_message: Option<String>,
    pub toasts: Vec<Toast>,

    // Preferencias globales
    pub settings: AppSettings,
    pub settings_modal_open: bool,
    pub settings_custom_draft: String,
    pub settings_bandwidth_draft: String,
    pub settings_max_concurrent_draft: String,

    // FTP log (oculto por defecto)
    pub ftp_log: FtpLog,

    /// Sesiones FTP persistentes (un stream por `connection.id`).
    pub ftp_manager: FtpSessionManager,
}

impl State {
    pub fn connection(&self, id: Uuid) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn connection_mut(&mut self, id: Uuid) -> Option<&mut Connection> {
        self.connections.iter_mut().find(|c| c.id == id)
    }

    pub fn connection_cloned(&self, id: Uuid) -> Option<Connection> {
        self.connection(id).cloned()
    }

    pub fn active_transfer(&self) -> Option<&TransferEntry> {
        self.transfers.iter().find(|t| t.status.is_active())
    }

    pub fn active_transfer_count(&self) -> usize {
        self.transfers
            .iter()
            .filter(|t| t.status.is_active())
            .count()
    }

    pub fn queued_count(&self) -> usize {
        self.transfers
            .iter()
            .filter(|t| matches!(t.status, TransferStatus::Queued))
            .count()
    }

    pub fn transfer_index(&self, id: Uuid) -> Option<usize> {
        self.transfers.iter().position(|t| t.id == id)
    }

    pub fn selected_for_panel(&self, panel: PanelKind) -> &HashSet<String> {
        match panel {
            PanelKind::Local => &self.selected_local,
            PanelKind::Remote => &self.selected_remote,
        }
    }

    pub fn clear_selection(&mut self, panel: PanelKind) {
        match panel {
            PanelKind::Local => self.selected_local.clear(),
            PanelKind::Remote => self.selected_remote.clear(),
        }
    }

    pub fn entries_for_panel(&self, panel: PanelKind) -> &[FtpEntry] {
        match panel {
            PanelKind::Local => &self.local_entries,
            PanelKind::Remote => &self.remote_entries,
        }
    }

    pub fn push_toast(&mut self, message: impl Into<String>, kind: ToastKind) {
        self.toasts.push(Toast::new(message, kind));
        if self.toasts.len() > 5 {
            let excess = self.toasts.len() - 5;
            self.toasts.drain(0..excess);
        }
    }

    pub fn expire_toasts(&mut self) {
        self.toasts.retain(|t| !t.is_expired());
    }
}
