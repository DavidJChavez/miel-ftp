use std::path::PathBuf;

use crate::config::{load_connections, save_connections};
use crate::error::{AppError, AppErrorMsg};
use crate::ftp::client;
use crate::models::{
    connection::{Connection, ConnectionStatus},
    connection_form::ConnectionForm,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    ftp_task::FtpTaskResult,
    message::Message,
    state::State,
    transfer::Transfer,
};
use iced::{Element, Length, Task, Theme};
use secrecy::SecretString;
use tracing::{error, warn};

pub fn boot() -> (State, Task<Message>) {
    let connections = load_connections().unwrap_or_else(|e| {
        warn!(error = %e, "no se pudieron cargar las conexiones guardadas");
        Vec::new()
    });

    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            warn!("no se encontró el directorio home; usando /");
            String::from("/")
        });

    let state = State {
        connections,
        selected_connection: None,
        connection_form: None,
        remote_status: ConnectionStatus::Disconnected,
        remote_path: String::from("/"),
        remote_entries: vec![],
        selected_remote: None,
        local_path: home.clone(),
        local_entries: vec![],
        selected_local: None,
        active_transfer: None,
        transfer_bytes: 0,
        status_message: None,
        ftp_log: FtpLog::default(),
    };

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
            state.status_message = None;

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            Task::perform(
                map_list_dir(conn, String::from("/")),
                Message::RemoteDirLoaded,
            )
        }

        Message::AddConnectionPressed => {
            state.connection_form = Some(ConnectionForm::new());
            Task::none()
        }

        Message::EditConnectionPressed(id) => {
            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            state.connection_form = Some(ConnectionForm::from_connection(
                conn.id,
                &conn.name,
                &conn.host,
                conn.port,
                &conn.username,
                conn.password(),
            ));
            Task::none()
        }

        Message::ConnectionFormNameChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.name = v;
                form.error = None;
            }
            Task::none()
        }
        Message::ConnectionFormHostChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.host = v;
                form.error = None;
            }
            Task::none()
        }
        Message::ConnectionFormPortChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.port = v;
                form.error = None;
            }
            Task::none()
        }
        Message::ConnectionFormUsernameChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.username = v;
                form.error = None;
            }
            Task::none()
        }
        Message::ConnectionFormPasswordChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.password = v;
                form.error = None;
            }
            Task::none()
        }

        Message::ConnectionFormCancel => {
            state.connection_form = None;
            Task::none()
        }

        Message::ConnectionFormSave => {
            let Some(form) = state.connection_form.clone() else {
                return Task::none();
            };

            let port: u16 = match form.port.trim().parse() {
                Ok(p) => p,
                Err(_) => {
                    state.connection_form = Some(ConnectionForm {
                        error: Some("puerto inválido".into()),
                        ..form
                    });
                    return Task::none();
                }
            };

            let name = form.name.clone();
            let host = form.host.clone();
            let username = form.username.clone();
            let password = form.password.clone();

            match Connection::try_new(
                form.editing_id,
                name,
                host,
                port,
                username,
                SecretString::from(password),
            ) {
                Ok(conn) => {
                    if let Some(id) = form.editing_id {
                        if let Some(idx) = state.connections.iter().position(|c| c.id == id) {
                            let _ = Connection::delete_password(id);
                            state.connections[idx] = conn;
                        }
                    } else {
                        state.connections.push(conn);
                    }

                    if let Err(e) = save_connections(&state.connections) {
                        state.connection_form = Some(ConnectionForm {
                            error: Some(e.to_string()),
                            ..form
                        });
                        return Task::none();
                    }

                    state.connection_form = None;
                    state.status_message = Some("Conexión guardada".into());
                }
                Err(e) => {
                    state.connection_form = Some(ConnectionForm {
                        error: Some(e.to_string()),
                        ..form
                    });
                }
            }
            Task::none()
        }

        Message::ConnectionFormDelete => {
            let Some(form) = &state.connection_form else {
                return Task::none();
            };
            let Some(id) = form.editing_id else {
                return Task::none();
            };

            state.connections.retain(|c| c.id != id);
            let _ = Connection::delete_password(id);

            if let Err(e) = save_connections(&state.connections) {
                state.status_message = Some(format!("Error al guardar: {e}"));
            } else {
                if state.selected_connection == Some(id) {
                    state.selected_connection = None;
                    state.remote_status = ConnectionStatus::Disconnected;
                    state.remote_entries.clear();
                }
                state.connection_form = None;
                state.status_message = Some("Conexión eliminada".into());
            }
            Task::done(Message::ConnectionDeleted)
        }

        Message::ConnectionDeleted => Task::none(),

        Message::ConnectPressed => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            state.remote_status = ConnectionStatus::Connecting;

            Task::perform(map_connect(conn), Message::ConnectResult)
        }

        Message::ConnectResult(outcome) => {
            apply_ftp_log(state, &outcome.log);
            match outcome.result {
                Ok(()) => {
                    let Some(id) = state.selected_connection else {
                        return Task::none();
                    };
                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::none();
                    };
                    Task::perform(
                        map_list_dir(conn, state.remote_path.clone()),
                        Message::RemoteDirLoaded,
                    )
                }
                Err(e) => {
                    let msg = e.to_string();
                    state.remote_status = ConnectionStatus::Error(msg.clone());
                    state.status_message = Some(msg);
                    Task::none()
                }
            }
        }

        Message::Disconnected => {
            state.remote_status = ConnectionStatus::Disconnected;
            state.remote_entries.clear();
            state.selected_remote = None;
            Task::none()
        }

        Message::LocalDirLoaded(Ok((path, entries))) => {
            state.local_path = path;
            state.local_entries = entries;
            Task::none()
        }
        Message::LocalDirLoaded(Err(e)) => {
            error!(error = %e, "error al cargar directorio local");
            state.status_message = Some(e.to_string());
            Task::none()
        }

        Message::RemoteDirLoaded(outcome) => {
            apply_ftp_log(state, &outcome.log);
            match outcome.result {
                Ok((path, entries)) => {
                    state.remote_status = ConnectionStatus::Connected;
                    state.remote_path = path;
                    state.remote_entries = entries;
                }
                Err(e) => {
                    client::log_ftp_error("list_dir", &e.0);
                    let msg = e.to_string();
                    state.remote_status = ConnectionStatus::Error(msg.clone());
                    state.status_message = Some(msg);
                }
            }
            Task::none()
        }

        Message::RemoteEntryOpened(entry) => {
            if !entry.is_dir {
                return Task::none();
            }

            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            let new_path = join_remote_path(&state.remote_path, &entry.name);

            Task::perform(map_list_dir(conn, new_path), Message::RemoteDirLoaded)
        }

        Message::LocalEntryOpened(entry) => {
            if !entry.is_dir {
                return Task::none();
            }

            let new_path = join_local_path(&state.local_path, &entry.name);

            Task::perform(load_local_dir(new_path), Message::LocalDirLoaded)
        }

        Message::LocalGoUp => {
            let parent = PathBuf::from(&state.local_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| state.local_path.clone());

            Task::perform(load_local_dir(parent), Message::LocalDirLoaded)
        }

        Message::RemoteGoUp => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            let parent = PathBuf::from(&state.remote_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| String::from("/"));

            Task::perform(map_list_dir(conn, parent), Message::RemoteDirLoaded)
        }

        Message::LocalRefresh => Task::perform(
            load_local_dir(state.local_path.clone()),
            Message::LocalDirLoaded,
        ),

        Message::RemoteRefresh => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            Task::perform(
                map_list_dir(conn, state.remote_path.clone()),
                Message::RemoteDirLoaded,
            )
        }

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

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            let local_path = PathBuf::from(&state.local_path).join(&entry.name);

            state.active_transfer = Some(Transfer {
                filename: entry.name.clone(),
                is_upload: true,
            });
            state.transfer_bytes = 0;

            Task::perform(
                map_upload(conn, local_path, state.remote_path.clone()),
                Message::TransferFinished,
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

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            let local_dir = PathBuf::from(&state.local_path);

            state.active_transfer = Some(Transfer {
                filename: entry.name.clone(),
                is_upload: false,
            });
            state.transfer_bytes = 0;

            Task::perform(
                map_download(
                    conn,
                    state.remote_path.clone(),
                    entry.name.clone(),
                    local_dir,
                ),
                Message::TransferFinished,
            )
        }

        Message::TransferProgress(bytes) => {
            state.transfer_bytes = bytes;
            Task::none()
        }

        Message::TransferFinished(outcome) => {
            apply_ftp_log(state, &outcome.log);
            state.active_transfer = None;
            state.transfer_bytes = 0;

            match outcome.result {
                Ok(()) => {
                    state.status_message = Some("Transferencia completada".into());

                    let local_path = state.local_path.clone();
                    let remote_path = state.remote_path.clone();

                    let Some(id) = state.selected_connection else {
                        return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
                    };

                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
                    };

                    Task::batch([
                        Task::perform(load_local_dir(local_path), Message::LocalDirLoaded),
                        Task::perform(map_list_dir(conn, remote_path), Message::RemoteDirLoaded),
                    ])
                }
                Err(e) => {
                    error!(error = %e, "transferencia fallida");
                    state.status_message = Some(e.to_string());
                    Task::none()
                }
            }
        }

        Message::ToggleFtpLog => {
            state.ftp_log.toggle_visible();
            Task::none()
        }

        Message::ClearFtpLog => {
            state.ftp_log.clear();
            Task::none()
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    use crate::ui::connection_modal::connection_modal;
    use crate::ui::file_panel::{PanelKind, file_panel};
    use crate::ui::ftp_log_panel::ftp_log_panel;
    use iced::widget::{column, container, row, text};

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
    .height(Length::Fill);

    let transfer = crate::ui::transfer_bar::transfer_bar(
        state.active_transfer.as_ref(),
        state.transfer_bytes,
        state.ftp_log.is_visible(),
    );

    let mut footer = column![];
    if state.ftp_log.is_visible() {
        footer = footer.push(ftp_log_panel(&state.ftp_log));
    }
    footer = footer.push(transfer);

    let base: Element<Message> = if let Some(msg) = &state.status_message {
        column![
            container(text(msg).size(11).color(crate::ui::theme::TEXT_MUTED))
                .padding([4, 12])
                .width(Length::Fill),
            panels,
            footer,
        ]
        .into()
    } else {
        column![panels, footer].into()
    };

    if let Some(form) = &state.connection_form {
        iced::widget::stack![base, connection_modal(form),].into()
    } else {
        base
    }
}

pub fn theme(_state: &State) -> Theme {
    crate::ui::theme::miel_theme()
}

fn join_local_path(base: &str, name: &str) -> String {
    let mut path = PathBuf::from(base);
    path.push(name);
    path.to_string_lossy().to_string()
}

fn join_remote_path(base: &str, name: &str) -> String {
    format!("{}/{}", base.trim_end_matches('/'), name)
}

fn apply_ftp_log(state: &mut State, lines: &[String]) {
    for line in lines {
        state.ftp_log.push(line.clone());
    }
}

async fn map_connect(conn: Connection) -> FtpTaskResult<()> {
    let response = client::connect(conn).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_list_dir(conn: Connection, path: String) -> FtpTaskResult<(String, Vec<FtpEntry>)> {
    let response = client::list_dir(conn, path).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_upload(
    conn: Connection,
    local_path: PathBuf,
    remote_path: String,
) -> FtpTaskResult<()> {
    let response = client::upload(conn, local_path, remote_path).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_download(
    conn: Connection,
    remote_path: String,
    filename: String,
    local_dir: PathBuf,
) -> FtpTaskResult<()> {
    let response = client::download(conn, remote_path, filename, local_dir).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn load_local_dir(path: String) -> Result<(String, Vec<FtpEntry>), AppErrorMsg> {
    let entries = std::fs::read_dir(&path)
        .map_err(AppError::from)?
        .filter_map(|e| e.ok())
        .map(|e| {
            let meta = e.metadata().ok();
            let is_dir = meta.as_ref().is_some_and(|m| m.is_dir());
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
