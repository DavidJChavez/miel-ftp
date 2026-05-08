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
            name: String::from("producción"),
            host: String::from("ftp.miservidor.com"),
            port: 21,
            username: String::from("user"),
            password: String::from("pass"),
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
            Task::none()
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
        Message::LocalGoUp
        | Message::LocalRefresh
        | Message::RemoteGoUp
        | Message::RemoteRefresh => Task::none(),
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
