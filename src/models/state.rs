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
    sort::SortSpec,
    transfer::{TransferEntry, TransferStatus},
};

pub struct State {
    // Sidebar
    pub connections: Vec<Connection>,
    pub selected_connection: Option<Uuid>,

    // Site manager
    pub connection_form: Option<ConnectionForm>,

    // Remote dashboard
    pub remote_status: ConnectionStatus,
    pub remote_path: String,
    pub remote_entries: Vec<FtpEntry>,
    pub selected_remote: HashSet<String>,
    pub last_clicked_remote: Option<String>,
    pub sort_remote: SortSpec,
    pub filter_remote: String,

    // Local dashboard
    pub local_path: String,
    pub local_entries: Vec<FtpEntry>,
    pub selected_local: HashSet<String>,
    pub last_clicked_local: Option<String>,
    pub sort_local: SortSpec,
    pub filter_local: String,

    // UI overlays
    pub context_menu: Option<ContextMenu>,
    pub prompt: Option<PromptDialog>,
    pub focused_panel: PanelKind,
    pub modifiers: Modifiers,

    // Transfer queue
    pub transfers: Vec<TransferEntry>,
    pub queue_panel_visible: bool,

    // UI feedback
    pub status_message: Option<String>,

    // FTP log (oculto por defecto)
    pub ftp_log: FtpLog,

    /// Sesiones FTP persistentes (un stream por `connection.id`).
    pub ftp_manager: FtpSessionManager,
}

impl State {
    pub fn connection(&self, id: Uuid) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn connection_cloned(&self, id: Uuid) -> Option<Connection> {
        self.connection(id).cloned()
    }

    pub fn active_transfer(&self) -> Option<&TransferEntry> {
        self.transfers.iter().find(|t| t.status.is_active())
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
}
