use iced::{Element, Length, Task, Theme};
use uuid::Uuid;

use crate::models::state::State;
pub use crate::models::{
    connection::{Connection, ConnectionStatus},
    ftp_entry::FtpEntry,
    message::Message,
    transfer::Transfer,
};

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
        selected_remote: None,
        local_path: String::from("/"),
        local_entries: vec![],
        selected_local: None,
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

        // Transfer
        Message::LocalFileSelected(name) => {
            state.selected_local = Some(name);
            Task::none()
        }

        Message::RemoteFileSelected(name) => {
            state.selected_remote = Some(name);
            Task::none()
        }

        Message::UploadPressed => {
            let Some(filename) = state.selected_local.clone() else {
                return Task::none();
            };

            let entry = state
                .local_entries
                .iter()
                .find(|e| e.name == filename)
                .cloned();
            let Some(entry) = entry else {
                return Task::none();
            };

            if entry.is_dir {
                return Task::none();
            }

            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            let local_path = std::path::PathBuf::from(&state.local_path).join(&entry.name);

            state.active_transfer = Some(Transfer {
                filename: entry.name.clone(),
                is_upload: true,
            });

            Task::perform(
                crate::ftp::client::upload(conn, local_path, state.remote_path.clone()),
                |result| match result {
                    Ok(_) => Message::TransferComplete(String::new()),
                    Err(e) => Message::TransferError(e),
                },
            )
        }

        Message::DownloadPressed => {
            let Some(filename) = state.selected_remote.clone() else {
                return Task::none();
            };
            let entry = state
                .remote_entries
                .iter()
                .find(|e| e.name == filename)
                .cloned();
            let Some(entry) = entry else {
                return Task::none();
            };

            if entry.is_dir {
                return Task::none();
            }

            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::none();
            };

            let local_dir = std::path::PathBuf::from(&state.local_path);

            state.active_transfer = Some(Transfer {
                filename: entry.name.clone(),
                is_upload: false,
            });

            Task::perform(
                crate::ftp::client::download(
                    conn,
                    state.remote_path.clone(),
                    entry.name.clone(),
                    local_dir,
                ),
                |result| match result {
                    Ok(_) => Message::TransferComplete(String::new()),
                    Err(e) => Message::TransferError(e),
                },
            )
        }

        Message::TransferComplete(_) => {
            state.active_transfer = None;

            // Refresh both dashboards
            let local_path = state.local_path.clone();
            let remote_path = state.remote_path.clone();

            let Some(id) = state.selected_connection else {
                return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
            };

            let Some(conn) = state.connections.iter().find(|c| c.id == id).cloned() else {
                return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
            };

            Task::batch([
                Task::perform(load_local_dir(local_path), Message::LocalDirLoaded),
                Task::perform(
                    crate::ftp::client::list_dir(conn, remote_path),
                    Message::RemoteDirLoaded,
                ),
            ])
        }

        Message::TransferError(e) => {
            eprintln!("transfer error: {e}");
            state.active_transfer = None;
            Task::none()
        }

        _ => Task::none(),
    }
}

pub fn view(state: &State) -> Element<Message> {
    use crate::ui::file_panel::{PanelKind, file_panel};
    use iced::widget::{column, container, row};

    let is_connected = matches!(state.remote_status, ConnectionStatus::Connected);

    let divider = |color| {
        container(iced::widget::Space::new().width(1))
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(iced::Background::Color(color)),
                ..container::Style::default()
            })
    };

    let panels = row![
        crate::ui::sidebar::sidebar(
            &state.connections,
            state.selected_connection,
            &state.remote_status,
        ),
        divider(crate::ui::theme::BORDER),
        file_panel(
            &PanelKind::Local,
            &state.local_path,
            &state.local_entries,
            true,
            state.selected_local.as_deref()
        ),
        divider(crate::ui::theme::BORDER_SUBTLE),
        file_panel(
            &PanelKind::Remote,
            &state.remote_path,
            &state.remote_entries,
            is_connected,
            state.selected_remote.as_deref()
        ),
    ]
    .height(iced::Length::Fill);

    column![
        panels,
        crate::ui::transfer_bar::transfer_bar(state.active_transfer.as_ref()),
    ]
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
