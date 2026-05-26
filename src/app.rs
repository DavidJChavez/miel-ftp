use std::collections::HashSet;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use futures::{SinkExt, StreamExt};
use iced::futures::channel::mpsc;
use iced::keyboard::{Key, Modifiers, key};
use iced::stream;
use iced::widget::stack;
use uuid::Uuid;

use crate::config::{load_connections, save_connections};
use crate::error::{AppError, AppErrorMsg};
use crate::ftp::{self, FtpSessionManager};
use crate::models::{
    connection::{Connection, ConnectionStatus},
    connection_form::ConnectionForm,
    context_menu::ContextMenu,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    ftp_task::FtpTaskResult,
    message::Message,
    panel::PanelKind,
    prompt::{PromptDialog, validate_entry_name},
    sort::{SortSpec, apply_view},
    state::State,
    transfer::{TransferEntry, TransferKind, TransferStatus},
};
use iced::futures::Stream;
use iced::{Element, Length, Subscription, Task, Theme, event};
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
        selected_remote: HashSet::new(),
        last_clicked_remote: None,
        sort_remote: SortSpec::default(),
        filter_remote: String::new(),
        local_path: home.clone(),
        local_entries: vec![],
        selected_local: HashSet::new(),
        last_clicked_local: None,
        sort_local: SortSpec::default(),
        filter_local: String::new(),
        context_menu: None,
        prompt: None,
        focused_panel: PanelKind::Local,
        modifiers: Modifiers::default(),
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
            state.selected_remote.clear();
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
            state.selected_remote.clear();

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
            state.selected_local.clear();

            Task::perform(load_local_dir(new_path), Message::LocalDirLoaded)
        }

        Message::LocalGoUp => {
            let parent = PathBuf::from(&state.local_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| state.local_path.clone());
            state.selected_local.clear();

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
            state.selected_remote.clear();

            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, parent, Some(id)),
                Message::RemoteDirLoaded,
            )
        }

        Message::LocalCrumbClicked(path) => {
            state.selected_local.clear();
            Task::perform(load_local_dir(path), Message::LocalDirLoaded)
        }

        Message::RemoteCrumbClicked(path) => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };
            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            state.selected_remote.clear();
            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, path, Some(id)),
                Message::RemoteDirLoaded,
            )
        }

        Message::LocalSortBy(key) => {
            state.sort_local = state.sort_local.toggle_key(key);
            Task::none()
        }

        Message::RemoteSortBy(key) => {
            state.sort_remote = state.sort_remote.toggle_key(key);
            Task::none()
        }

        Message::LocalFilterChanged(v) => {
            state.filter_local = v;
            Task::none()
        }

        Message::RemoteFilterChanged(v) => {
            state.filter_remote = v;
            Task::none()
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

        Message::LocalFileSelected { name, shift, ctrl } => {
            handle_file_selection(state, PanelKind::Local, name, shift, ctrl);
            Task::none()
        }

        Message::RemoteFileSelected { name, shift, ctrl } => {
            handle_file_selection(state, PanelKind::Remote, name, shift, ctrl);
            Task::none()
        }

        Message::LocalSelectAll => {
            state.selected_local = state.local_entries.iter().map(|e| e.name.clone()).collect();
            Task::none()
        }

        Message::RemoteSelectAll => {
            state.selected_remote = state
                .remote_entries
                .iter()
                .map(|e| e.name.clone())
                .collect();
            Task::none()
        }

        Message::ClearSelection(panel) => {
            state.clear_selection(panel);
            Task::none()
        }

        Message::PanelFocused(panel) => {
            state.focused_panel = panel;
            Task::none()
        }

        Message::ModifiersChanged(m) => {
            state.modifiers = m;
            Task::none()
        }

        Message::KeyPressed { key, modifiers } => handle_keypress(state, key, modifiers),

        Message::MkdirPressed(panel) => {
            state.context_menu = None;
            state.prompt = Some(PromptDialog::mkdir(panel));
            Task::none()
        }

        Message::RenamePressed(panel) => {
            state.context_menu = None;
            let selected = state.selected_for_panel(panel);
            if selected.len() != 1 {
                return Task::none();
            }
            let old = selected.iter().next().cloned().unwrap_or_default();
            state.prompt = Some(PromptDialog::rename(panel, old));
            Task::none()
        }

        Message::DeletePressed(panel) => {
            state.context_menu = None;
            let names: Vec<String> = state.selected_for_panel(panel).iter().cloned().collect();
            if names.is_empty() {
                return Task::none();
            }
            state.prompt = Some(PromptDialog::confirm_delete(panel, names));
            Task::none()
        }

        Message::PromptValueChanged(v) => {
            if let Some(prompt) = &mut state.prompt {
                match prompt {
                    PromptDialog::Mkdir { value, .. } => *value = v,
                    PromptDialog::Rename { new, .. } => *new = v,
                    PromptDialog::ConfirmDelete { .. } => {}
                }
            }
            Task::none()
        }

        Message::PromptCancel => {
            state.prompt = None;
            Task::none()
        }

        Message::PromptSubmit => {
            let Some(prompt) = state.prompt.clone() else {
                return Task::none();
            };
            state.prompt = None;
            execute_prompt(state, prompt)
        }

        Message::LocalOpFinished(result) => match result {
            Ok(()) => {
                state.status_message = Some("Operación completada".into());
                refresh_dirs_task(state)
            }
            Err(e) => {
                state.status_message = Some(e.to_string());
                Task::none()
            }
        },

        Message::RemoteOpFinished(outcome) => {
            apply_ftp_log(state, &outcome.log);
            match outcome.result {
                Ok(()) => {
                    state.status_message = Some("Operación completada".into());
                    refresh_dirs_task(state)
                }
                Err(e) => {
                    let msg = e.to_string();
                    state.status_message = Some(msg);
                    Task::none()
                }
            }
        }

        Message::ContextMenuOpened { panel, target } => {
            state.context_menu = Some(ContextMenu {
                panel,
                target: target.clone(),
            });
            match panel {
                PanelKind::Local => {
                    state.selected_local.clear();
                    state.selected_local.insert(target);
                    state.last_clicked_local = state.selected_local.iter().next().cloned();
                }
                PanelKind::Remote => {
                    state.selected_remote.clear();
                    state.selected_remote.insert(target);
                    state.last_clicked_remote = state.selected_remote.iter().next().cloned();
                }
            }
            Task::none()
        }

        Message::ContextMenuClosed => {
            state.context_menu = None;
            Task::none()
        }

        Message::UploadPressed => enqueue_uploads(state),

        Message::DownloadPressed => enqueue_downloads(state),

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
    use crate::ui::context_menu::context_menu_overlay;
    use crate::ui::file_panel::file_panel;
    use crate::ui::ftp_log_panel::ftp_log_panel;
    use crate::ui::prompt_modal::prompt_modal;
    use crate::ui::status_bar::status_bar;
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
            PanelKind::Local,
            &state.local_path,
            &state.local_entries,
            true,
            &state.selected_local,
            state.sort_local,
            &state.filter_local,
        ),
        divider(crate::ui::theme::BORDER_SUBTLE),
        file_panel(
            PanelKind::Remote,
            &state.remote_path,
            &state.remote_entries,
            is_connected,
            &state.selected_remote,
            state.sort_remote,
            &state.filter_remote,
        ),
    ]
    .height(Length::Fill);

    let status = status_bar(
        state.focused_panel,
        state.entries_for_panel(state.focused_panel),
        state.selected_for_panel(state.focused_panel),
        match state.focused_panel {
            PanelKind::Local => state.sort_local,
            PanelKind::Remote => state.sort_remote,
        },
        match state.focused_panel {
            PanelKind::Local => &state.filter_local,
            PanelKind::Remote => &state.filter_remote,
        },
    );

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
    footer = footer.push(status);
    footer = footer.push(transfer);

    let mut base: Element<Message> = if let Some(msg) = &state.status_message {
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
        base = stack![base, connection_modal(form),].into();
    }

    if let Some(prompt) = &state.prompt {
        base = stack![base, prompt_modal(prompt),].into();
    }

    if let Some(menu) = &state.context_menu {
        base = stack![base, context_menu_overlay(menu),].into();
    }

    base
}

pub fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        event::listen_with(keyboard_event),
        event::listen_with(modifiers_event),
    ])
}

fn keyboard_event(
    event: iced::Event,
    status: event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    if matches!(status, event::Status::Captured) {
        return None;
    }
    if let iced::Event::Keyboard(iced::keyboard::Event::KeyPressed { key, modifiers, .. }) = event {
        Some(Message::KeyPressed { key, modifiers })
    } else {
        None
    }
}

fn modifiers_event(
    event: iced::Event,
    _status: event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    if let iced::Event::Keyboard(iced::keyboard::Event::ModifiersChanged(m)) = event {
        Some(Message::ModifiersChanged(m))
    } else {
        None
    }
}

fn handle_keypress(state: &mut State, key: Key, modifiers: Modifiers) -> Task<Message> {
    if state.prompt.is_some() {
        if matches!(key, Key::Named(key::Named::Escape)) {
            state.prompt = None;
        }
        return Task::none();
    }

    if state.context_menu.is_some() {
        if matches!(key, Key::Named(key::Named::Escape)) {
            state.context_menu = None;
        }
        return Task::none();
    }

    let panel = state.focused_panel;
    let selected_count = state.selected_for_panel(panel).len();
    let ctrl = modifiers.command();

    match key {
        Key::Named(key::Named::F5) => match panel {
            PanelKind::Local => Task::perform(
                load_local_dir(state.local_path.clone()),
                Message::LocalDirLoaded,
            ),
            PanelKind::Remote => {
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
        },
        Key::Named(key::Named::Backspace) => match panel {
            PanelKind::Local => {
                let parent = PathBuf::from(&state.local_path)
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| state.local_path.clone());
                state.selected_local.clear();
                Task::perform(load_local_dir(parent), Message::LocalDirLoaded)
            }
            PanelKind::Remote => {
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
                state.selected_remote.clear();
                let mgr = state.ftp_manager.clone();
                Task::perform(
                    map_list_dir(mgr, conn, parent, Some(id)),
                    Message::RemoteDirLoaded,
                )
            }
        },
        Key::Named(key::Named::Delete) => {
            let names: Vec<String> = state.selected_for_panel(panel).iter().cloned().collect();
            if names.is_empty() {
                Task::none()
            } else {
                state.prompt = Some(PromptDialog::confirm_delete(panel, names));
                Task::none()
            }
        }
        Key::Named(key::Named::F2) if selected_count == 1 => {
            let old = state
                .selected_for_panel(panel)
                .iter()
                .next()
                .cloned()
                .unwrap_or_default();
            state.prompt = Some(PromptDialog::rename(panel, old));
            Task::none()
        }
        Key::Named(key::Named::Enter) if selected_count == 1 => {
            let name = state
                .selected_for_panel(panel)
                .iter()
                .next()
                .cloned()
                .unwrap_or_default();
            let entry = state
                .entries_for_panel(panel)
                .iter()
                .find(|e| e.name == name)
                .cloned();
            let Some(entry) = entry else {
                return Task::none();
            };
            if !entry.is_dir {
                return Task::none();
            }
            match panel {
                PanelKind::Local => {
                    let new_path = join_local_path(&state.local_path, &entry.name);
                    state.selected_local.clear();
                    Task::perform(load_local_dir(new_path), Message::LocalDirLoaded)
                }
                PanelKind::Remote => {
                    let Some(id) = state.selected_connection else {
                        return Task::none();
                    };
                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::none();
                    };
                    let new_path = join_remote_path(&state.remote_path, &entry.name);
                    state.selected_remote.clear();
                    let mgr = state.ftp_manager.clone();
                    Task::perform(
                        map_list_dir(mgr, conn, new_path, Some(id)),
                        Message::RemoteDirLoaded,
                    )
                }
            }
        }
        Key::Character(c) if ctrl && c.as_str() == "a" => Task::done(match panel {
            PanelKind::Local => Message::LocalSelectAll,
            PanelKind::Remote => Message::RemoteSelectAll,
        }),
        Key::Named(key::Named::Escape) => Task::done(Message::ClearSelection(panel)),
        _ => Task::none(),
    }
}

pub fn theme(_state: &State) -> Theme {
    crate::ui::theme::miel_theme()
}

fn handle_file_selection(
    state: &mut State,
    panel: PanelKind,
    name: String,
    shift: bool,
    ctrl: bool,
) {
    let (entries, sort, filter) = match panel {
        PanelKind::Local => (
            &state.local_entries,
            state.sort_local,
            state.filter_local.as_str(),
        ),
        PanelKind::Remote => (
            &state.remote_entries,
            state.sort_remote,
            state.filter_remote.as_str(),
        ),
    };

    let visible = apply_view(entries, sort, filter);
    let names: Vec<String> = visible.iter().map(|e| e.name.clone()).collect();

    let selected = match panel {
        PanelKind::Local => &mut state.selected_local,
        PanelKind::Remote => &mut state.selected_remote,
    };

    let last_clicked = match panel {
        PanelKind::Local => &mut state.last_clicked_local,
        PanelKind::Remote => &mut state.last_clicked_remote,
    };

    let use_shift = shift || state.modifiers.shift();
    let use_ctrl = ctrl || state.modifiers.command();

    if use_shift {
        if let Some(anchor) = last_clicked.clone() {
            let anchor_idx = names.iter().position(|n| n == &anchor);
            let current_idx = names.iter().position(|n| n == &name);
            if let (Some(a), Some(c)) = (anchor_idx, current_idx) {
                let (start, end) = if a <= c { (a, c) } else { (c, a) };
                if !use_ctrl {
                    selected.clear();
                }
                for n in &names[start..=end] {
                    selected.insert(n.clone());
                }
            }
        } else {
            selected.clear();
            selected.insert(name.clone());
        }
    } else if use_ctrl {
        if selected.contains(&name) {
            selected.remove(&name);
        } else {
            selected.insert(name.clone());
        }
    } else {
        selected.clear();
        selected.insert(name.clone());
    }

    *last_clicked = Some(name);
}

fn execute_prompt(state: &mut State, prompt: PromptDialog) -> Task<Message> {
    match prompt {
        PromptDialog::Mkdir { panel, value } => {
            let name = match validate_entry_name(&value) {
                Ok(n) => n,
                Err(e) => {
                    state.status_message = Some(e);
                    return Task::none();
                }
            };
            match panel {
                PanelKind::Local => {
                    let path = state.local_path.clone();
                    Task::perform(local_mkdir(path, name), Message::LocalOpFinished)
                }
                PanelKind::Remote => {
                    let Some(id) = state.selected_connection else {
                        return Task::none();
                    };
                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::none();
                    };
                    let remote_dir = state.remote_path.clone();
                    let mgr = state.ftp_manager.clone();
                    Task::perform(
                        map_mkdir(mgr, conn, remote_dir, name),
                        Message::RemoteOpFinished,
                    )
                }
            }
        }
        PromptDialog::Rename { panel, old, new } => {
            let new_name = match validate_entry_name(&new) {
                Ok(n) => n,
                Err(e) => {
                    state.status_message = Some(e);
                    return Task::none();
                }
            };
            if old == new_name {
                return Task::none();
            }
            match panel {
                PanelKind::Local => {
                    let path = state.local_path.clone();
                    Task::perform(local_rename(path, old, new_name), Message::LocalOpFinished)
                }
                PanelKind::Remote => {
                    let Some(id) = state.selected_connection else {
                        return Task::none();
                    };
                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::none();
                    };
                    let remote_dir = state.remote_path.clone();
                    let mgr = state.ftp_manager.clone();
                    Task::perform(
                        map_rename(mgr, conn, remote_dir, old, new_name),
                        Message::RemoteOpFinished,
                    )
                }
            }
        }
        PromptDialog::ConfirmDelete { panel, names } => match panel {
            PanelKind::Local => {
                let path = state.local_path.clone();
                let entries: Vec<(String, bool)> = names
                    .iter()
                    .filter_map(|n| {
                        state
                            .local_entries
                            .iter()
                            .find(|e| &e.name == n)
                            .map(|e| (n.clone(), e.is_dir))
                    })
                    .collect();
                state.clear_selection(PanelKind::Local);
                Task::perform(local_remove_many(path, entries), Message::LocalOpFinished)
            }
            PanelKind::Remote => {
                let Some(id) = state.selected_connection else {
                    return Task::none();
                };
                let Some(conn) = state.connection_cloned(id) else {
                    return Task::none();
                };
                let remote_dir = state.remote_path.clone();
                let entries: Vec<(String, bool)> = names
                    .iter()
                    .filter_map(|n| {
                        state
                            .remote_entries
                            .iter()
                            .find(|e| &e.name == n)
                            .map(|e| (n.clone(), e.is_dir))
                    })
                    .collect();
                state.clear_selection(PanelKind::Remote);
                let mgr = state.ftp_manager.clone();
                Task::perform(
                    map_remove_many(mgr, conn, remote_dir, entries),
                    Message::RemoteOpFinished,
                )
            }
        },
    }
}

fn enqueue_uploads(state: &mut State) -> Task<Message> {
    let Some(connection_id) = state.selected_connection else {
        return Task::none();
    };

    let names: Vec<String> = state.selected_local.iter().cloned().collect();
    if names.is_empty() {
        return Task::none();
    }

    for filename in names {
        let entry = state
            .local_entries
            .iter()
            .find(|e| e.name == filename)
            .cloned();
        let Some(entry) = entry else {
            continue;
        };
        if entry.is_dir {
            continue;
        }

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
    }

    start_next_transfer_if_idle(state)
}

fn enqueue_downloads(state: &mut State) -> Task<Message> {
    let Some(connection_id) = state.selected_connection else {
        return Task::none();
    };

    let names: Vec<String> = state.selected_remote.iter().cloned().collect();
    if names.is_empty() {
        return Task::none();
    }

    for filename in names {
        let entry = state
            .remote_entries
            .iter()
            .find(|e| e.name == filename)
            .cloned();
        let Some(entry) = entry else {
            continue;
        };
        if entry.is_dir {
            continue;
        }

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
    }

    start_next_transfer_if_idle(state)
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

async fn map_mkdir(
    mgr: FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    name: String,
) -> FtpTaskResult<()> {
    let response = ftp::client::mkdir(&mgr, conn, remote_dir, name).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_rename(
    mgr: FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    old: String,
    new: String,
) -> FtpTaskResult<()> {
    let response = ftp::client::rename(&mgr, conn, remote_dir, old, new).await;
    FtpTaskResult::from_response(response.result, response.log)
}

async fn map_remove_many(
    mgr: FtpSessionManager,
    conn: Connection,
    remote_dir: String,
    entries: Vec<(String, bool)>,
) -> FtpTaskResult<()> {
    let mut all_log = Vec::new();
    for (name, is_dir) in entries {
        let response =
            ftp::client::remove(&mgr, conn.clone(), remote_dir.clone(), name, is_dir).await;
        all_log.extend(response.log);
        if let Err(e) = response.result {
            return FtpTaskResult::from_response(Err(e), all_log);
        }
    }
    FtpTaskResult::from_response(Ok(()), all_log)
}

async fn local_mkdir(path: String, name: String) -> Result<(), AppErrorMsg> {
    let mut dir = PathBuf::from(path);
    dir.push(name);
    tokio::fs::create_dir(&dir).await.map_err(AppError::from)?;
    Ok(())
}

async fn local_rename(path: String, old: String, new: String) -> Result<(), AppErrorMsg> {
    let from = PathBuf::from(&path).join(old);
    let to = PathBuf::from(&path).join(new);
    tokio::fs::rename(from, to).await.map_err(AppError::from)?;
    Ok(())
}

async fn local_remove_many(path: String, entries: Vec<(String, bool)>) -> Result<(), AppErrorMsg> {
    for (name, is_dir) in entries {
        let target = PathBuf::from(&path).join(name);
        if is_dir {
            tokio::fs::remove_dir(&target)
                .await
                .map_err(AppError::from)?;
        } else {
            tokio::fs::remove_file(&target)
                .await
                .map_err(AppError::from)?;
        }
    }
    Ok(())
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
            let modified = meta.as_ref().and_then(|m| {
                m.modified().ok().map(|t| {
                    let datetime: chrono::DateTime<chrono::Utc> = t.into();
                    datetime.format("%Y-%m-%d %H:%M").to_string()
                })
            });
            FtpEntry {
                name: e.file_name().to_string_lossy().to_string(),
                is_dir,
                size,
                modified,
            }
        })
        .collect();

    Ok((path, entries))
}
