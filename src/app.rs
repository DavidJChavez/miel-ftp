use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use futures::{SinkExt, StreamExt};
use iced::futures::channel::mpsc;
use iced::stream;
use uuid::Uuid;

use crate::config::{load_connections, save_connections};
use crate::error::{AppError, AppErrorMsg};
use crate::ftp::{self, FtpSessionManager};
use crate::models::{
    connection::{Connection, ConnectionStatus},
    connection_form::ConnectionForm,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    ftp_task::FtpTaskResult,
    message::Message,
    state::State,
    transfer::{TransferEntry, TransferKind, TransferStatus},
};
use iced::futures::Stream;
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
        transfers: vec![],
        queue_panel_visible: false,
        status_message: None,
        ftp_log: FtpLog::default(),
        ftp_manager: FtpSessionManager::new(),
    };

    (
        state,
        Task::perform(load_local_dir(home), Message::LocalDirLoaded),
    )
}

pub fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::ConnectionSelected(id) => {
            let prev = state.selected_connection;
            state.selected_connection = Some(id);
            state.remote_status = ConnectionStatus::Connecting;
            state.status_message = None;

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, String::from("/"), prev),
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
            let mgr = state.ftp_manager.clone();

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
            Task::batch([
                Task::perform(
                    async move {
                        mgr.disconnect(id).await;
                    },
                    |_| Message::Noop,
                ),
                Task::done(Message::ConnectionDeleted),
            ])
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

            let mgr = state.ftp_manager.clone();
            Task::perform(map_connect(mgr, conn), Message::ConnectResult)
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
                    let mgr = state.ftp_manager.clone();
                    Task::perform(
                        map_list_dir(mgr, conn, state.remote_path.clone(), Some(id)),
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
            let mgr = state.ftp_manager.clone();
            Task::perform(
                async move {
                    mgr.disconnect_all().await;
                },
                |_| Message::Noop,
            )
        }

        Message::Noop => Task::none(),

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
                    ftp::client::log_ftp_error("list_dir", &e.0);
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

            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, new_path, Some(id)),
                Message::RemoteDirLoaded,
            )
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

            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, parent, Some(id)),
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

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, state.remote_path.clone(), Some(id)),
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

            let Some(connection_id) = state.selected_connection else {
                return Task::none();
            };

            let local_path = PathBuf::from(&state.local_path).join(&entry.name);
            let total_bytes = std::fs::metadata(&local_path).ok().map(|m| m.len());

            state.transfers.push(TransferEntry {
                id: Uuid::new_v4(),
                connection_id,
                kind: TransferKind::Upload,
                filename: entry.name,
                local_path,
                remote_path: state.remote_path.clone(),
                total_bytes,
                transferred_bytes: 0,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });

            start_next_transfer_if_idle(state)
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

            let Some(connection_id) = state.selected_connection else {
                return Task::none();
            };

            let local_dir = PathBuf::from(&state.local_path);
            let total_bytes = entry.size;

            let name = entry.name.clone();
            state.transfers.push(TransferEntry {
                id: Uuid::new_v4(),
                connection_id,
                kind: TransferKind::Download,
                filename: name.clone(),
                local_path: local_dir.join(&name),
                remote_path: state.remote_path.clone(),
                total_bytes,
                transferred_bytes: 0,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });

            start_next_transfer_if_idle(state)
        }

        Message::TransferProgress { id, bytes, total } => {
            if let Some(idx) = state.transfer_index(id) {
                state.transfers[idx].transferred_bytes = bytes;
                if state.transfers[idx].total_bytes.is_none() {
                    state.transfers[idx].total_bytes = total;
                }
            }
            Task::none()
        }

        Message::TransferFinished { id, result } => {
            apply_ftp_log(state, &result.log);

            let mut success = false;
            if let Some(idx) = state.transfer_index(id) {
                match &result.result {
                    Ok(()) => {
                        state.transfers[idx].status = TransferStatus::Done;
                        success = true;
                    }
                    Err(e) if matches!(*e.0, AppError::Cancelled) => {
                        state.transfers[idx].status = TransferStatus::Cancelled;
                        state.status_message = Some("Transferencia cancelada".into());
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        state.transfers[idx].status = TransferStatus::Failed(msg.clone());
                        state.status_message = Some(msg);
                    }
                }
            }

            if success {
                state.status_message = Some("Transferencia completada".into());
            }

            let refresh = if success {
                refresh_dirs_task(state)
            } else {
                Task::none()
            };

            Task::batch([refresh, start_next_transfer_if_idle(state)])
        }

        Message::CancelTransfer(id) => {
            if let Some(idx) = state.transfer_index(id) {
                match &state.transfers[idx].status {
                    TransferStatus::Queued => {
                        state.transfers[idx].status = TransferStatus::Cancelled;
                        return start_next_transfer_if_idle(state);
                    }
                    TransferStatus::Active => {
                        state.transfers[idx]
                            .cancel
                            .store(true, std::sync::atomic::Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
            Task::none()
        }

        Message::RetryTransfer(id) => {
            let Some(entry) = state
                .transfers
                .iter()
                .find(|t| t.id == id && t.status.is_terminal())
                .cloned()
            else {
                return Task::none();
            };

            state.transfers.push(TransferEntry {
                id: Uuid::new_v4(),
                connection_id: entry.connection_id,
                kind: entry.kind,
                filename: entry.filename,
                local_path: entry.local_path,
                remote_path: entry.remote_path,
                total_bytes: entry.total_bytes,
                transferred_bytes: 0,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });

            start_next_transfer_if_idle(state)
        }

        Message::ClearCompletedTransfers => {
            state.transfers.retain(|t| !t.status.is_terminal());
            Task::none()
        }

        Message::ToggleTransferQueue => {
            state.queue_panel_visible = !state.queue_panel_visible;
            Task::none()
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
    use crate::ui::transfer_queue_panel::transfer_queue_panel;
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
        state.active_transfer(),
        state.queued_count(),
        state.queue_panel_visible,
        state.ftp_log.is_visible(),
    );

    let mut footer = column![];
    if state.queue_panel_visible {
        footer = footer.push(transfer_queue_panel(&state.transfers));
    }
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

fn start_next_transfer_if_idle(state: &mut State) -> Task<Message> {
    if state.transfers.iter().any(|t| t.status.is_active()) {
        return Task::none();
    }

    let Some(idx) = state
        .transfers
        .iter()
        .position(|t| matches!(t.status, TransferStatus::Queued))
    else {
        return Task::none();
    };

    state.transfers[idx].status = TransferStatus::Active;

    let entry = state.transfers[idx].clone();
    let mgr = state.ftp_manager.clone();

    let Some(conn) = state.connection_cloned(entry.connection_id) else {
        state.transfers[idx].status = TransferStatus::Failed("Conexión no encontrada".into());
        return Task::none();
    };

    Task::stream(transfer_stream(entry, conn, mgr))
}

fn transfer_stream(
    entry: TransferEntry,
    conn: Connection,
    mgr: FtpSessionManager,
) -> impl Stream<Item = Message> {
    stream::channel(32, async move |mut output| {
        let (progress_tx, mut progress_rx) = mpsc::unbounded::<u64>();
        let id = entry.id;
        let total_hint = entry.total_bytes;

        let work = async {
            match entry.kind {
                TransferKind::Upload => {
                    mgr.upload_stream(
                        conn,
                        entry.local_path,
                        entry.remote_path,
                        entry.cancel.clone(),
                        progress_tx,
                    )
                    .await
                }
                TransferKind::Download => {
                    let local_dir = entry
                        .local_path
                        .parent()
                        .map(PathBuf::from)
                        .unwrap_or_else(|| PathBuf::from("."));
                    mgr.download_stream(
                        conn,
                        entry.remote_path,
                        entry.filename,
                        local_dir,
                        entry.cancel.clone(),
                        progress_tx,
                    )
                    .await
                }
            }
        };

        let mut work = pin!(work);
        let mut progress_open = true;

        loop {
            tokio::select! {
                response = &mut work => {
                    let result = FtpTaskResult::from_response(response.result, response.log);
                    let _ = output.send(Message::TransferFinished { id, result }).await;
                    break;
                }
                bytes = progress_rx.next(), if progress_open => {
                    match bytes {
                        Some(n) => {
                            let _ = output
                                .send(Message::TransferProgress {
                                    id,
                                    bytes: n,
                                    total: total_hint,
                                })
                                .await;
                        }
                        None => progress_open = false,
                    }
                }
            }
        }
    })
}

fn refresh_dirs_task(state: &State) -> Task<Message> {
    let local_path = state.local_path.clone();
    let remote_path = state.remote_path.clone();

    let Some(id) = state.selected_connection else {
        return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
    };

    let Some(conn) = state.connection_cloned(id) else {
        return Task::perform(load_local_dir(local_path), Message::LocalDirLoaded);
    };

    let mgr = state.ftp_manager.clone();
    Task::batch([
        Task::perform(load_local_dir(local_path), Message::LocalDirLoaded),
        Task::perform(
            map_list_dir(mgr, conn, remote_path, Some(id)),
            Message::RemoteDirLoaded,
        ),
    ])
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

async fn map_connect(mgr: FtpSessionManager, conn: Connection) -> FtpTaskResult<()> {
    let response = ftp::client::connect(&mgr, conn).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_list_dir(
    mgr: FtpSessionManager,
    conn: Connection,
    path: String,
    previous_selection: Option<uuid::Uuid>,
) -> FtpTaskResult<(String, Vec<FtpEntry>)> {
    if let Some(prev) = previous_selection
        && prev != conn.id
    {
        mgr.disconnect(prev).await;
    }
    mgr.disconnect_all_except(conn.id).await;

    let response = ftp::client::list_dir(&mgr, conn, path).await;
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
