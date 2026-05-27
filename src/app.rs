use std::collections::HashSet;
use std::path::PathBuf;
use std::pin::pin;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

use futures::{SinkExt, StreamExt};
use iced::futures::channel::mpsc;
use iced::keyboard::{Key, Modifiers, key};
use iced::stream;
use iced::time;
use iced::widget::stack;
use uuid::Uuid;

use crate::config::{load_connections, load_settings, save_connections, save_settings};
use crate::error::{AppError, AppErrorMsg};
use crate::ftp::{self, FtpSessionManager};
use crate::models::{
    connection::{Connection, ConnectionStatus, Protocol},
    connection_form::ConnectionForm,
    context_menu::ContextMenu,
    ftp_entry::FtpEntry,
    ftp_log::FtpLog,
    ftp_task::FtpTaskResult,
    message::Message,
    panel::PanelKind,
    prompt::{PromptDialog, validate_entry_name},
    quickconnect::QuickconnectForm,
    remote_edit::RemoteEdit,
    sort::{SortSpec, apply_view},
    state::State,
    sync::{SyncAction, SyncDiff, SyncEntry, SyncState},
    toast::ToastKind,
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

    let settings = load_settings().unwrap_or_else(|e| {
        warn!(error = %e, "no se pudieron cargar las preferencias");
        crate::models::settings::AppSettings::default()
    });

    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| {
            warn!("no se encontró el directorio home; usando /");
            String::from("/")
        });

    let settings_bandwidth_draft = settings.bandwidth.display_value();
    let settings_max_concurrent_draft = settings.max_concurrent_clamped().to_string();

    let state = State {
        connections,
        selected_connection: None,
        quickconnect_expanded: false,
        quickconnect: QuickconnectForm::new(),
        connection_form: None,
        remote_status: ConnectionStatus::Disconnected,
        remote_path: String::from("/"),
        remote_entries: vec![],
        remote_loading: false,
        selected_remote: HashSet::new(),
        last_clicked_remote: None,
        sort_remote: SortSpec::default(),
        filter_remote: String::new(),
        local_path: home.clone(),
        local_entries: vec![],
        local_loading: false,
        selected_local: HashSet::new(),
        last_clicked_local: None,
        sort_local: SortSpec::default(),
        filter_local: String::new(),
        context_menu: None,
        prompt: None,
        focused_panel: PanelKind::Local,
        modifiers: Modifiers::default(),
        drop_hover: false,
        sync_panel: None,
        transfers: vec![],
        queue_panel_visible: false,
        pending_remote_edits: vec![],
        status_message: None,
        toasts: vec![],
        settings,
        settings_modal_open: false,
        settings_custom_draft: String::new(),
        settings_bandwidth_draft,
        settings_max_concurrent_draft,
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
            state.remote_loading = true;
            state.status_message = None;

            let Some(conn) = state.connection_cloned(id) else {
                state.remote_loading = false;
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
                conn.mode,
                conn.protocol,
                conn.accept_invalid_certs,
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

        Message::ConnectionFormModeChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.active_mode = v;
                form.error = None;
            }
            Task::none()
        }

        Message::ConnectionFormProtocolChanged(protocol) => {
            if let Some(form) = &mut state.connection_form {
                form.set_protocol(protocol);
                form.error = None;
            }
            Task::none()
        }

        Message::ConnectionFormAcceptInvalidCertsChanged(v) => {
            if let Some(form) = &mut state.connection_form {
                form.accept_invalid_certs = v;
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

            let bookmarks = form
                .editing_id
                .and_then(|id| state.connection(id).map(|c| c.bookmarks.clone()))
                .unwrap_or_default();

            match Connection::try_new(
                form.editing_id,
                name,
                host,
                port,
                username,
                SecretString::from(password),
                form.ftp_mode(),
                form.protocol,
                form.ftp_security(),
                form.accept_invalid_certs,
                bookmarks,
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
                    state.push_toast("Conexión guardada", ToastKind::Success);
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
            state.remote_loading = true;

            let mgr = state.ftp_manager.clone();
            Task::perform(map_connect(mgr, conn), Message::ConnectResult)
        }

        Message::ToggleQuickconnect => {
            state.quickconnect_expanded = !state.quickconnect_expanded;
            Task::none()
        }

        Message::QuickconnectHostChanged(v) => {
            state.quickconnect.host = v;
            state.quickconnect.error = None;
            Task::none()
        }

        Message::QuickconnectPortChanged(v) => {
            state.quickconnect.port = v;
            state.quickconnect.error = None;
            Task::none()
        }

        Message::QuickconnectUsernameChanged(v) => {
            state.quickconnect.username = v;
            state.quickconnect.error = None;
            Task::none()
        }

        Message::QuickconnectPasswordChanged(v) => {
            state.quickconnect.password = v;
            state.quickconnect.error = None;
            Task::none()
        }

        Message::QuickconnectConnect => {
            let form = state.quickconnect.clone();
            let port: u16 = match form.port.trim().parse() {
                Ok(p) => p,
                Err(_) => {
                    state.quickconnect.error = Some("puerto inválido".into());
                    return Task::none();
                }
            };

            match Connection::try_new(
                None,
                format!("{}:{}", form.host.trim(), port),
                form.host,
                port,
                form.username,
                SecretString::from(form.password),
                crate::models::connection::FtpMode::Passive,
                Protocol::Ftp,
                crate::models::connection::FtpSecurity::Plain,
                false,
                vec![],
            ) {
                Ok(conn) => {
                    let id = conn.id;
                    state.connections.push(conn);
                    state.selected_connection = Some(id);
                    state.remote_status = ConnectionStatus::Connecting;
                    state.remote_loading = true;
                    state.quickconnect.error = None;
                    let Some(conn) = state.connection_cloned(id) else {
                        return Task::none();
                    };
                    let mgr = state.ftp_manager.clone();
                    Task::perform(map_connect(mgr, conn), Message::ConnectResult)
                }
                Err(e) => {
                    state.quickconnect.error = Some(e.to_string());
                    Task::none()
                }
            }
        }

        Message::BookmarkAdd => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };
            let path = state.remote_path.clone();
            let Some(conn) = state.connection_mut(id) else {
                return Task::none();
            };
            conn.add_bookmark(path);
            if let Err(e) = save_connections(&state.connections) {
                state.push_toast(format!("Error al guardar marcador: {e}"), ToastKind::Error);
            } else {
                state.push_toast("Marcador añadido", ToastKind::Success);
            }
            Task::none()
        }

        Message::BookmarkRemove(path) => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };
            if let Some(conn) = state.connection_mut(id) {
                conn.remove_bookmark(&path);
            }
            if let Err(e) = save_connections(&state.connections) {
                state.push_toast(format!("Error al eliminar marcador: {e}"), ToastKind::Error);
            }
            Task::none()
        }

        Message::BookmarkNavigate(path) => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };
            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            state.remote_loading = true;
            state.selected_remote.clear();
            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, path, Some(id)),
                Message::RemoteDirLoaded,
            )
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
                    state.remote_loading = true;
                    Task::perform(
                        map_list_dir(mgr, conn, state.remote_path.clone(), Some(id)),
                        Message::RemoteDirLoaded,
                    )
                }
                Err(e) => {
                    let msg = e.to_string();
                    state.remote_status = ConnectionStatus::Error(msg.clone());
                    state.remote_loading = false;
                    state.push_toast(msg, ToastKind::Error);
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
            state.local_loading = false;
            Task::none()
        }
        Message::LocalDirLoaded(Err(e)) => {
            error!(error = %e, "error al cargar directorio local");
            state.local_loading = false;
            state.push_toast(e.to_string(), ToastKind::Error);
            Task::none()
        }

        Message::RemoteDirLoaded(outcome) => {
            state.remote_loading = false;
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
                    state.push_toast(msg, ToastKind::Error);
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
            state.remote_loading = true;

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
            state.local_loading = true;

            Task::perform(load_local_dir(new_path), Message::LocalDirLoaded)
        }

        Message::LocalGoUp => {
            let parent = PathBuf::from(&state.local_path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| state.local_path.clone());
            state.selected_local.clear();
            state.local_loading = true;

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
            state.remote_loading = true;

            let mgr = state.ftp_manager.clone();
            Task::perform(
                map_list_dir(mgr, conn, parent, Some(id)),
                Message::RemoteDirLoaded,
            )
        }

        Message::LocalCrumbClicked(path) => {
            state.selected_local.clear();
            state.local_loading = true;
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
            state.remote_loading = true;
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

        Message::LocalRefresh => {
            state.local_loading = true;
            Task::perform(
                load_local_dir(state.local_path.clone()),
                Message::LocalDirLoaded,
            )
        }

        Message::RemoteRefresh => {
            let Some(id) = state.selected_connection else {
                return Task::none();
            };

            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };

            state.remote_loading = true;
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
            let mut remote_edit_path = None;
            if let Some(idx) = state.transfer_index(id) {
                match &result.result {
                    Ok(()) => {
                        state.transfers[idx].status = TransferStatus::Done;
                        success = true;
                        let local_path = state.transfers[idx].local_path.clone();
                        if state
                            .pending_remote_edits
                            .iter()
                            .any(|e| e.temp_path == local_path)
                        {
                            remote_edit_path = Some(local_path);
                        }
                    }
                    Err(e) if matches!(*e.0, AppError::Cancelled) => {
                        state.transfers[idx].status = TransferStatus::Cancelled;
                        state.push_toast("Transferencia cancelada", ToastKind::Info);
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        state.transfers[idx].status = TransferStatus::Failed(msg.clone());
                        state.push_toast(msg, ToastKind::Error);
                    }
                }
            }

            if success {
                state.push_toast("Transferencia completada", ToastKind::Success);
            }

            let refresh = if success {
                refresh_dirs_task(state)
            } else {
                Task::none()
            };

            let mut followups = vec![refresh, fill_transfer_slots(state)];
            if let Some(path) = remote_edit_path {
                followups.push(Task::done(Message::RemoteEditUploaded(path)));
            }
            Task::batch(followups)
        }

        Message::CancelTransfer(id) => {
            if let Some(idx) = state.transfer_index(id) {
                match &state.transfers[idx].status {
                    TransferStatus::Queued => {
                        state.transfers[idx].status = TransferStatus::Cancelled;
                        return fill_transfer_slots(state);
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
                resume_from: 0,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });

            fill_transfer_slots(state)
        }

        Message::ResumeTransfer(id) => {
            let Some(entry) = state
                .transfers
                .iter()
                .find(|t| t.id == id && t.status.is_terminal() && t.transferred_bytes > 0)
                .cloned()
            else {
                return Task::none();
            };

            let resume_from = entry.transferred_bytes;

            state.transfers.push(TransferEntry {
                id: Uuid::new_v4(),
                connection_id: entry.connection_id,
                kind: entry.kind,
                filename: entry.filename,
                local_path: entry.local_path,
                remote_path: entry.remote_path,
                total_bytes: entry.total_bytes,
                transferred_bytes: resume_from,
                resume_from,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });

            fill_transfer_slots(state)
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

        Message::ToggleSettingsModal => {
            state.settings_modal_open = !state.settings_modal_open;
            if state.settings_modal_open {
                state.settings_bandwidth_draft = state.settings.bandwidth.display_value();
                state.settings_max_concurrent_draft =
                    state.settings.max_concurrent_clamped().to_string();
            } else {
                state.settings_custom_draft.clear();
                state.settings_bandwidth_draft.clear();
                state.settings_max_concurrent_draft.clear();
            }
            Task::none()
        }

        Message::SettingsHideSystemToggled(v) => {
            state.settings.hide_system_files = v;
            persist_settings(state);
            Task::none()
        }

        Message::SettingsShowDotfilesToggled(v) => {
            state.settings.show_dotfiles = v;
            persist_settings(state);
            Task::none()
        }

        Message::SettingsCustomDraftChanged(v) => {
            state.settings_custom_draft = v;
            Task::none()
        }

        Message::SettingsCustomAdded(name) => {
            let name = name.trim().to_string();
            if name.is_empty() {
                return Task::none();
            }
            if !state.settings.custom_hidden.contains(&name) {
                state.settings.custom_hidden.push(name);
                persist_settings(state);
            }
            state.settings_custom_draft.clear();
            Task::none()
        }

        Message::SettingsCustomRemoved(name) => {
            state.settings.custom_hidden.retain(|n| n != &name);
            persist_settings(state);
            Task::none()
        }

        Message::SettingsBandwidthDraftChanged(v) => {
            state.settings_bandwidth_draft = v.clone();
            if let Some(bandwidth) = crate::models::settings::Bandwidth::from_kbps_input(&v) {
                state.settings.bandwidth = bandwidth;
                persist_settings(state);
            }
            Task::none()
        }

        Message::FilesHoverEntered => {
            state.drop_hover = true;
            Task::none()
        }

        Message::FilesHoverLeft => {
            state.drop_hover = false;
            Task::none()
        }

        Message::SettingsMaxConcurrentChanged(v) => {
            state.settings_max_concurrent_draft = v.clone();
            if let Some(max) = crate::models::settings::AppSettings::from_max_concurrent_input(&v) {
                state.settings.max_concurrent = max;
                persist_settings(state);
            }
            Task::none()
        }

        Message::OpenSyncPanel => {
            if !matches!(state.remote_status, ConnectionStatus::Connected) {
                state.push_toast("Conéctate antes de sincronizar", ToastKind::Error);
                return Task::none();
            }
            state.sync_panel = Some(SyncState {
                entries: vec![],
                analyzing: true,
            });
            let local_path = state.local_path.clone();
            let remote_path = state.remote_path.clone();
            let local_entries = state.local_entries.clone();
            let Some(id) = state.selected_connection else {
                return Task::none();
            };
            let Some(conn) = state.connection_cloned(id) else {
                return Task::none();
            };
            let mgr = state.ftp_manager.clone();
            Task::perform(
                analyze_sync(mgr, conn, local_path, remote_path, local_entries),
                Message::SyncAnalyzeResult,
            )
        }

        Message::CloseSyncPanel => {
            state.sync_panel = None;
            Task::none()
        }

        Message::SyncAnalyzeResult(entries) => {
            if let Some(panel) = &mut state.sync_panel {
                panel.entries = entries;
                panel.analyzing = false;
            }
            Task::none()
        }

        Message::SyncEntryActionChanged(name, action) => {
            if let Some(panel) = &mut state.sync_panel
                && let Some(entry) = panel.entries.iter_mut().find(|e| e.name == name)
            {
                entry.action = action;
            }
            Task::none()
        }

        Message::SyncApply => {
            let Some(panel) = state.sync_panel.clone() else {
                return Task::none();
            };
            apply_sync_actions(state, &panel.entries);
            state.sync_panel = None;
            state.push_toast("Sincronización encolada", ToastKind::Success);
            fill_transfer_slots(state)
        }

        Message::EditRemoteFile(filename) => {
            state.context_menu = None;
            let Some(connection_id) = state.selected_connection else {
                return Task::none();
            };
            let Some(conn) = state.connection_cloned(connection_id) else {
                return Task::none();
            };
            let remote_dir = state.remote_path.clone();
            let temp_dir = std::env::temp_dir();
            let temp_path = temp_dir.join(&filename);
            let mgr = state.ftp_manager.clone();
            let cancel = Arc::new(AtomicBool::new(false));
            let (progress_tx, _progress_rx) = mpsc::unbounded();
            Task::perform(
                async move {
                    let response = mgr
                        .download_stream(
                            conn,
                            remote_dir.clone(),
                            filename.clone(),
                            temp_dir,
                            0,
                            cancel,
                            progress_tx,
                            None,
                        )
                        .await;
                    match response.result {
                        Ok(()) => Ok((temp_path, remote_dir, filename)),
                        Err(e) => Err(AppErrorMsg::from(e)),
                    }
                },
                |result| match result {
                    Ok((temp_path, remote_dir, filename)) => Message::RemoteEditDownloaded {
                        temp_path,
                        remote_dir,
                        filename,
                    },
                    Err(e) => Message::RemoteEditFailed(e),
                },
            )
        }

        Message::RemoteEditFailed(e) => {
            state.push_toast(e.to_string(), ToastKind::Error);
            Task::none()
        }

        Message::RemoteEditDownloaded {
            temp_path,
            remote_dir,
            filename,
        } => {
            let Some(connection_id) = state.selected_connection else {
                return Task::none();
            };
            if let Err(e) = open::that(&temp_path) {
                state.push_toast(format!("No se pudo abrir el editor: {e}"), ToastKind::Error);
                return Task::none();
            }
            let remote_path = join_remote_path(&remote_dir, &filename);
            state.pending_remote_edits.push(RemoteEdit {
                temp_path: temp_path.clone(),
                remote_dir,
                filename,
                connection_id,
            });
            state.push_toast(
                format!("Archivo abierto ({remote_path}). Re-sube desde el menú contextual."),
                ToastKind::Info,
            );
            Task::none()
        }

        Message::ReuploadRemoteEdit(temp_path) => {
            let Some(edit) = state
                .pending_remote_edits
                .iter()
                .find(|e| e.temp_path == temp_path)
                .cloned()
            else {
                return Task::none();
            };
            state.transfers.push(TransferEntry {
                id: Uuid::new_v4(),
                connection_id: edit.connection_id,
                kind: TransferKind::Upload,
                filename: edit.filename.clone(),
                local_path: edit.temp_path.clone(),
                remote_path: edit.remote_dir.clone(),
                total_bytes: std::fs::metadata(&edit.temp_path).ok().map(|m| m.len()),
                transferred_bytes: 0,
                resume_from: 0,
                status: TransferStatus::Queued,
                cancel: Arc::new(AtomicBool::new(false)),
            });
            fill_transfer_slots(state)
        }

        Message::RemoteEditUploaded(temp_path) => {
            state
                .pending_remote_edits
                .retain(|e| e.temp_path != temp_path);
            state.push_toast("Edición remota subida", ToastKind::Success);
            Task::none()
        }

        Message::ToastTick => {
            state.expire_toasts();
            Task::none()
        }

        Message::DismissToast(id) => {
            state.toasts.retain(|t| t.id != id);
            Task::none()
        }

        Message::FileDropped(path) => {
            state.drop_hover = false;
            enqueue_dropped_path(state, path)
        }
    }
}

pub fn view(state: &State) -> Element<'_, Message> {
    use crate::ui::connection_modal::connection_modal;
    use crate::ui::context_menu::context_menu_overlay;
    use crate::ui::file_panel::file_panel;
    use crate::ui::ftp_log_panel::ftp_log_panel;
    use crate::ui::prompt_modal::prompt_modal;
    use crate::ui::settings_modal::settings_modal;
    use crate::ui::status_bar::status_bar;
    use crate::ui::sync_panel::sync_panel;
    use crate::ui::toast_overlay::toast_overlay;
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

    let bookmarks: &[String] = state
        .selected_connection
        .and_then(|id| state.connection(id))
        .map(|c| c.bookmarks.as_slice())
        .unwrap_or(&[]);

    let panels = row![
        crate::ui::sidebar::sidebar(
            &state.connections,
            state.selected_connection,
            &state.remote_status,
            state.quickconnect_expanded,
            &state.quickconnect,
            bookmarks,
            is_connected,
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
            false,
            state.local_loading,
            &state.settings,
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
            state.drop_hover,
            state.remote_loading,
            &state.settings,
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
        &state.settings,
        &state.remote_status,
    );

    let transfer = crate::ui::transfer_bar::transfer_bar(
        state.active_transfer(),
        state.active_transfer_count(),
        state.queued_count(),
        state.pending_remote_edits.len(),
        state.queue_panel_visible,
        state.ftp_log.is_visible(),
        state.settings_modal_open,
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
        base = stack![
            base,
            context_menu_overlay(menu, &state.pending_remote_edits),
        ]
        .into();
    }

    if state.settings_modal_open {
        base = stack![
            base,
            settings_modal(
                &state.settings,
                &state.settings_custom_draft,
                &state.settings_bandwidth_draft,
                &state.settings_max_concurrent_draft,
            ),
        ]
        .into();
    }

    if let Some(sync) = &state.sync_panel {
        base = stack![base, sync_panel(sync),].into();
    }

    base = stack![base, toast_overlay(&state.toasts),].into();

    base
}

pub fn subscription(_state: &State) -> Subscription<Message> {
    Subscription::batch([
        event::listen_with(keyboard_event),
        event::listen_with(modifiers_event),
        event::listen_with(window_event),
        time::every(Duration::from_millis(500)).map(|_| Message::ToastTick),
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

fn window_event(
    event: iced::Event,
    _status: event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    use iced::window::Event::{FileDropped, FileHovered, FilesHoveredLeft};
    if let iced::Event::Window(e) = event {
        return match e {
            FileHovered(_) => Some(Message::FilesHoverEntered),
            FilesHoveredLeft => Some(Message::FilesHoverLeft),
            FileDropped(path) => Some(Message::FileDropped(path)),
            _ => None,
        };
    }
    None
}

fn handle_keypress(state: &mut State, key: Key, modifiers: Modifiers) -> Task<Message> {
    if state.settings_modal_open {
        if matches!(key, Key::Named(key::Named::Escape)) {
            state.settings_modal_open = false;
            state.settings_custom_draft.clear();
            state.settings_bandwidth_draft.clear();
        }
        return Task::none();
    }

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

    let visible = apply_view(entries, sort, filter, &state.settings);
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
            resume_from: 0,
            status: TransferStatus::Queued,
            cancel: Arc::new(AtomicBool::new(false)),
        });
    }

    fill_transfer_slots(state)
}

fn enqueue_dropped_path(state: &mut State, path: PathBuf) -> Task<Message> {
    let Some(connection_id) = state.selected_connection else {
        state.status_message = Some("Conéctate antes de soltar archivos".into());
        return Task::none();
    };

    if !matches!(state.remote_status, ConnectionStatus::Connected) {
        state.status_message = Some("Conéctate antes de soltar archivos".into());
        return Task::none();
    }

    let mut enqueued = 0usize;
    let mut skipped_dirs = false;

    let paths_to_upload: Vec<PathBuf> = if path.is_dir() {
        match std::fs::read_dir(&path) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    let p = e.path();
                    if p.is_dir() {
                        skipped_dirs = true;
                        None
                    } else {
                        Some(p)
                    }
                })
                .collect(),
            Err(e) => {
                state.status_message = Some(e.to_string());
                return Task::none();
            }
        }
    } else {
        vec![path]
    };

    for local_path in paths_to_upload {
        let Some(filename) = local_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        let total_bytes = std::fs::metadata(&local_path).ok().map(|m| m.len());

        state.transfers.push(TransferEntry {
            id: Uuid::new_v4(),
            connection_id,
            kind: TransferKind::Upload,
            filename: filename.to_string(),
            local_path,
            remote_path: state.remote_path.clone(),
            total_bytes,
            transferred_bytes: 0,
            resume_from: 0,
            status: TransferStatus::Queued,
            cancel: Arc::new(AtomicBool::new(false)),
        });
        enqueued += 1;
    }

    if enqueued == 0 {
        state.status_message = Some(if skipped_dirs {
            "Solo se suben archivos sueltos; las carpetas anidadas se ignoran".into()
        } else {
            "No hay archivos para subir".into()
        });
        return Task::none();
    }

    if skipped_dirs {
        state.status_message =
            Some("Carpetas anidadas ignoradas; solo archivos del nivel superior".into());
    }

    fill_transfer_slots(state)
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
            resume_from: 0,
            status: TransferStatus::Queued,
            cancel: Arc::new(AtomicBool::new(false)),
        });
    }

    fill_transfer_slots(state)
}

fn fill_transfer_slots(state: &mut State) -> Task<Message> {
    let active = state.active_transfer_count();
    let slots = state.settings.max_concurrent_clamped() as usize;
    let available = slots.saturating_sub(active);
    if available == 0 {
        return Task::none();
    }

    let bandwidth = state.settings.bandwidth.limit_kbps();
    let mgr = state.ftp_manager.clone();

    let mut to_start = Vec::new();
    for transfer in state.transfers.iter_mut() {
        if to_start.len() >= available {
            break;
        }
        if !matches!(transfer.status, TransferStatus::Queued) {
            continue;
        }
        transfer.status = TransferStatus::Active;
        to_start.push(transfer.clone());
    }

    let mut tasks = Vec::new();
    for entry in to_start {
        let Some(conn) = state.connection_cloned(entry.connection_id) else {
            if let Some(idx) = state.transfer_index(entry.id) {
                state.transfers[idx].status =
                    TransferStatus::Failed("Conexión no encontrada".into());
            }
            continue;
        };
        tasks.push(Task::stream(transfer_stream(
            entry,
            conn,
            mgr.clone(),
            bandwidth,
        )));
    }

    if tasks.is_empty() {
        Task::none()
    } else {
        Task::batch(tasks)
    }
}

fn transfer_stream(
    entry: TransferEntry,
    conn: Connection,
    mgr: FtpSessionManager,
    limit_kbps: Option<u32>,
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
                        entry.resume_from,
                        entry.cancel.clone(),
                        progress_tx,
                        limit_kbps,
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
                        entry.resume_from,
                        entry.cancel.clone(),
                        progress_tx,
                        limit_kbps,
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

fn persist_settings(state: &mut State) {
    if let Err(e) = save_settings(&state.settings) {
        state.status_message = Some(format!("Error al guardar preferencias: {e}"));
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

async fn analyze_sync(
    mgr: FtpSessionManager,
    conn: Connection,
    local_path: String,
    remote_path: String,
    local_entries: Vec<FtpEntry>,
) -> Vec<SyncEntry> {
    let response = mgr.list_dir(conn, remote_path).await;
    let remote_entries = response.result.map(|(_, e)| e).unwrap_or_default();

    let local_files: std::collections::HashMap<_, _> = local_entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| (e.name.clone(), e))
        .collect();
    let remote_files: std::collections::HashMap<_, _> = remote_entries
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| (e.name.clone(), e))
        .collect();

    let mut names: std::collections::BTreeSet<_> = local_files.keys().cloned().collect();
    names.extend(remote_files.keys().cloned());

    names
        .into_iter()
        .filter_map(
            |name| match (local_files.get(&name), remote_files.get(&name)) {
                (Some(_local), None) => Some(SyncEntry {
                    name,
                    diff: SyncDiff::OnlyLocal,
                    action: SyncAction::default_for(SyncDiff::OnlyLocal),
                }),
                (None, Some(_remote)) => Some(SyncEntry {
                    name,
                    diff: SyncDiff::OnlyRemote,
                    action: SyncAction::default_for(SyncDiff::OnlyRemote),
                }),
                (Some(local), Some(remote)) => {
                    let local_mtime = local_path_file_mtime(&local_path, &name)
                        .or_else(|| crate::ftp::sftp::parse_modified(local));
                    let remote_mtime = crate::ftp::sftp::parse_modified(remote);
                    let local_newer = match (local_mtime, remote_mtime) {
                        (Some(l), Some(r)) => l > r,
                        _ => true,
                    };
                    let diff = SyncDiff::Both { local_newer };
                    Some(SyncEntry {
                        name,
                        diff,
                        action: SyncAction::default_for(diff),
                    })
                }
                (None, None) => None,
            },
        )
        .collect()
}

fn local_path_file_mtime(local_path: &str, name: &str) -> Option<std::time::SystemTime> {
    let path = PathBuf::from(local_path).join(name);
    std::fs::metadata(path).ok()?.modified().ok()
}

fn apply_sync_actions(state: &mut State, entries: &[SyncEntry]) {
    let Some(connection_id) = state.selected_connection else {
        return;
    };

    for entry in entries {
        match entry.action {
            SyncAction::Skip => {}
            SyncAction::UploadToRemote => {
                let local_path = PathBuf::from(&state.local_path).join(&entry.name);
                let total_bytes = std::fs::metadata(&local_path).ok().map(|m| m.len());
                state.transfers.push(TransferEntry {
                    id: Uuid::new_v4(),
                    connection_id,
                    kind: TransferKind::Upload,
                    filename: entry.name.clone(),
                    local_path,
                    remote_path: state.remote_path.clone(),
                    total_bytes,
                    transferred_bytes: 0,
                    resume_from: 0,
                    status: TransferStatus::Queued,
                    cancel: Arc::new(AtomicBool::new(false)),
                });
            }
            SyncAction::DownloadToLocal => {
                let local_dir = PathBuf::from(&state.local_path);
                let total_bytes = state
                    .remote_entries
                    .iter()
                    .find(|e| e.name == entry.name)
                    .and_then(|e| e.size);
                state.transfers.push(TransferEntry {
                    id: Uuid::new_v4(),
                    connection_id,
                    kind: TransferKind::Download,
                    filename: entry.name.clone(),
                    local_path: local_dir.join(&entry.name),
                    remote_path: state.remote_path.clone(),
                    total_bytes,
                    transferred_bytes: 0,
                    resume_from: 0,
                    status: TransferStatus::Queued,
                    cancel: Arc::new(AtomicBool::new(false)),
                });
            }
            SyncAction::DeleteLocal => {
                let target = PathBuf::from(&state.local_path).join(&entry.name);
                let _ = std::fs::remove_file(target);
            }
            SyncAction::DeleteRemote => {
                // Encolar como operación remota vía transfer dummy no ideal — usar prompt flow
                // Por simplicidad: encolar download skip y log; usuario puede borrar manualmente.
                // Mejor: no implementado async aquí; omitir por ahora con toast.
            }
        }
    }
}
