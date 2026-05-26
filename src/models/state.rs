use uuid::Uuid;

use crate::models::{
    connection::{Connection, ConnectionStatus},
    connection_form::ConnectionForm,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    transfer::Transfer,
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
    pub selected_remote: Option<String>,

    // Local dashboard
    pub local_path: String,
    pub local_entries: Vec<FtpEntry>,
    pub selected_local: Option<String>,

    // Active transfer
    pub active_transfer: Option<Transfer>,
    pub transfer_bytes: u64,

    // UI feedback
    pub status_message: Option<String>,

    // FTP log (oculto por defecto)
    pub ftp_log: FtpLog,
}

impl State {
    pub fn connection(&self, id: Uuid) -> Option<&Connection> {
        self.connections.iter().find(|c| c.id == id)
    }

    pub fn connection_cloned(&self, id: Uuid) -> Option<Connection> {
        self.connection(id).cloned()
    }
}
