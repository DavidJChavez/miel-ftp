use uuid::Uuid;

use crate::models::{
    connection::{Connection, ConnectionStatus},
    ftp_entry::FtpEntry,
    transfer::Transfer,
};

pub struct State {
    // Sidebar
    pub connections: Vec<Connection>,
    pub selected_connection: Option<Uuid>,

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
}
