use iced::{Element, Length, Task, Theme};
use uuid::Uuid;

use crate::ui::file_panel::file_panel;

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String, // TODO: encryptar con keyring en v2
}

#[derive(Debug, Clone)]
pub struct FtpEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,        // None si es directorio
    pub modified: Option<String>, // Simplificado por ahora
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub filename: String,
    pub total: u64,
    pub transferred: u64,
}

impl Transfer {
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        self.transferred as f32 / self.total as f32
    }
}

pub struct State {
    // Sidebar
    pub connections: Vec<Connection>,
    pub selected_connection: Option<Uuid>,

    // Remote dashboard
    pub remote_status: ConnectionStatus,
    pub remote_path: String,
    pub remote_entries: Vec<FtpEntry>,

    // Local dashboard
    pub local_path: String,
    pub local_entries: Vec<FtpEntry>,

    // Active transfer
    pub active_transfer: Option<Transfer>,
}

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
    UploadPressed(FtpEntry),
    DownloadPressed(FtpEntry),
    TransferProgress(u64),
    TransferComplete,
    TransferError(String),

    LocalGoUp,
    LocalRefresh,
    RemoteGoUp,
    RemoteRefresh,
}

pub fn boot() -> (State, Task<Message>) {
    let state = State {
        connections: vec![Connection {
            id: Uuid::new_v4(),
            name: String::from("Prueba"),
            host: String::from("127.0.0.1"),
            port: 2121,
            username: String::from("david"),
            password: String::from("password"),
        }],
        selected_connection: None,
        remote_status: ConnectionStatus::Disconnected,
        remote_path: String::from("/"),
        remote_entries: vec![],
        local_path: String::from("/"),
        local_entries: vec![],
        active_transfer: None,
    };

    let home = dirs::home_dir()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    (
        state,
        Task::perform(load_local_dir(home), Message::LocalDirLoaded),
    )
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ConnectionSelected(id) => {
            state.selected_connection = Some(id);
            state.remote_status = ConnectionStatus::Connecting;

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };
            Task::perform(
                crate::ftp::client::list_dir(conn, String::from("/")),
                Message::RemoteDirLoaded,
            )
        }
        Message::LocalDirLoaded(Ok((path, entries))) => {
            state.local_path = path;
            state.local_entries = entries;
            Task::none()
        }
        Message::LocalDirLoaded(Err(e)) => {
            eprintln!("error loading local dir: {e}");
            Task::none()
        }
        Message::ConnectPressed => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            state.remote_status = ConnectionStatus::Connecting;

            Task::perform(
                crate::ftp::client::list_dir(conn, String::from("/")),
                Message::RemoteDirLoaded,
            )
        }
        Message::RemoteDirLoaded(Ok((path, entries))) => {
            state.remote_status = ConnectionStatus::Connected;
            state.remote_path = path;
            state.remote_entries = entries;
            Task::none()
        }
        Message::RemoteDirLoaded(Err(e)) => {
            eprintln!("ftp error: {e}");
            state.remote_status = ConnectionStatus::Error(e);
            Task::none()
        }
        Message::RemoteEntryOpened(entry) => {
            if !entry.is_dir {
                return Task::none();
            }

            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            let new_path = format!("{}/{}", state.remote_path.trim_end_matches('/'), entry.name);

            Task::perform(
                crate::ftp::client::list_dir(conn, new_path),
                Message::RemoteDirLoaded,
            )
        }
        Message::LocalEntryOpened(entry) => {
            if !entry.is_dir {
                return Task::none();
            }

            let new_path = format!("{}/{}", state.local_path.trim_end_matches('/'), entry.name);

            Task::perform(load_local_dir(new_path), Message::LocalDirLoaded)
        }
        Message::LocalGoUp => {
            let parent = std::path::Path::new(&state.local_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| state.local_path.clone());

            Task::perform(load_local_dir(parent), Message::LocalDirLoaded)
        }
        Message::RemoteGoUp => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            let parent = std::path::Path::new(&state.remote_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| String::from("/"));

            Task::perform(
                crate::ftp::client::list_dir(conn, parent),
                Message::RemoteDirLoaded,
            )
        }
        Message::LocalRefresh => Task::perform(
            load_local_dir(state.local_path.clone()),
            Message::LocalDirLoaded,
        ),
        Message::RemoteRefresh => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            Task::perform(
                crate::ftp::client::list_dir(conn.clone(), state.remote_path.clone()),
                Message::RemoteDirLoaded,
            )
        }
        _ => Task::none(),
    }
}

pub fn view(state: &State) -> Element<Message> {
    use crate::ui::file_panel::{PanelKind, file_panel};
    use iced::widget::{container, row};

    let is_connected = matches!(state.remote_status, ConnectionStatus::Connected);

    let divider = container(iced::widget::Space::new().width(1))
        .height(Length::Fill)
        .style(|_| container::Style {
            background: Some(iced::Background::Color(crate::ui::theme::BORDER_SUBTLE)),
            ..container::Style::default()
        });

    row![
        crate::ui::sidebar::sidebar(
            &state.connections,
            state.selected_connection,
            &state.remote_status,
        ),
        container(iced::widget::Space::new().width(1))
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(iced::Background::Color(crate::ui::theme::BORDER)),
                ..container::Style::default()
            }),
        file_panel(
            &PanelKind::Local,
            &state.local_path,
            &state.local_entries,
            true
        ),
        divider,
        file_panel(
            &PanelKind::Remote,
            &state.remote_path,
            &state.remote_entries,
            is_connected
        ),
    ]
    .height(iced::Length::Fill)
    .into()
}

pub fn theme(state: &State) -> Theme {
    crate::ui::theme::miel_theme()
}

// Helpers async

async fn load_local_dir(path: String) -> Result<(String, Vec<FtpEntry>), String> {
    let entries = std::fs::read_dir(&path)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| {
            let meta = e.metadata().ok();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = meta
                .as_ref()
                .and_then(|m| if is_dir { None } else { Some(m.len()) });
            FtpEntry {
                name: e.file_name().to_string_lossy().to_string(),
                is_dir,
                size,
                modified: None,
            }
        })
        .collect();

    Ok((path, entries))
}
