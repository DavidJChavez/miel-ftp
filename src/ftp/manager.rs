use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use futures::AsyncReadExt;
use suppaftp::AsyncFtpStream;
use tokio::sync::Mutex;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::ftp::client::FtpResponse;
use crate::ftp::config::{self, CONNECT_TIMEOUT, OPERATION_TIMEOUT};
use crate::ftp::retry::{is_retryable, with_timeout};
use crate::models::{connection::Connection, ftp_entry::FtpEntry};

fn cmd(line: impl AsRef<str>) -> String {
    format!("> {}", line.as_ref())
}

macro_rules! ftp_retry {
    ($manager:expr, $operation:literal, $session_id:expr, $attempt:expr) => {{
        let mut last_err: Option<AppError> = None;
        let mut ok = None;

        for try_num in 0..config::MAX_RETRIES {
            match $attempt.await {
                Ok(value) => {
                    ok = Some(value);
                    break;
                }
                Err(err) => {
                    let retryable = is_retryable(&err);
                    last_err = Some(err);

                    if !retryable || try_num + 1 >= config::MAX_RETRIES {
                        break;
                    }

                    $manager.invalidate($session_id).await;

                    warn!(
                        operation = $operation,
                        attempt = try_num + 1,
                        max = config::MAX_RETRIES,
                        "reintentando operación FTP"
                    );
                    tokio::time::sleep(config::retry_delay(try_num)).await;
                }
            }
        }

        match ok {
            Some(value) => Ok(value),
            None => {
                Err(last_err
                    .unwrap_or_else(|| AppError::Validation("operación FTP fallida".into())))
            }
        }
    }};
}

struct ActiveSession {
    stream: AsyncFtpStream,
}

/// Gestor de sesiones FTP: un `AsyncFtpStream` vivo por `connection.id`.
#[derive(Clone)]
pub struct FtpSessionManager {
    sessions: Arc<Mutex<HashMap<Uuid, ActiveSession>>>,
}

impl FtpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn disconnect(&self, id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        if let Some(mut session) = sessions.remove(&id) {
            let _ = session.stream.quit().await;
            debug!(%id, "sesión FTP cerrada");
        }
    }

    pub async fn disconnect_all_except(&self, keep: Uuid) {
        let mut sessions = self.sessions.lock().await;
        let to_remove: Vec<Uuid> = sessions.keys().copied().filter(|id| *id != keep).collect();
        for id in to_remove {
            if let Some(mut session) = sessions.remove(&id) {
                let _ = session.stream.quit().await;
            }
        }
    }

    pub async fn disconnect_all(&self) {
        let mut sessions = self.sessions.lock().await;
        for (_, mut session) in sessions.drain() {
            let _ = session.stream.quit().await;
        }
    }

    pub async fn connect(&self, conn: Connection) -> FtpResponse<()> {
        let mut log = Vec::new();
        let result = ftp_retry!(self, "connect", conn.id, async {
            self.invalidate(conn.id).await;
            self.open_session(&conn, &mut log).await
        });

        if let Err(e) = &result {
            log.push(format!("< LOGIN ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn list_dir(
        &self,
        conn: Connection,
        path: String,
    ) -> FtpResponse<(String, Vec<FtpEntry>)> {
        let mut log = Vec::new();
        let result = ftp_retry!(self, "list_dir", conn.id, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_list_dir(&conn.id, &path, &mut log).await
        });

        if let Err(e) = &result {
            log.push(format!("< LIST ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn upload(
        &self,
        conn: Connection,
        local_path: PathBuf,
        remote_path: String,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let result = ftp_retry!(self, "upload", conn.id, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_upload(&conn.id, &local_path, &remote_path, &mut log)
                .await
        });

        if let Err(e) = &result {
            log.push(format!("< STOR ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn download(
        &self,
        conn: Connection,
        remote_path: String,
        filename: String,
        local_dir: PathBuf,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let result = ftp_retry!(self, "download", conn.id, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_download(&conn.id, &remote_path, &filename, &local_dir, &mut log)
                .await
        });

        if let Err(e) = &result {
            log.push(format!("< RETR ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    async fn invalidate(&self, id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        if let Some(mut session) = sessions.remove(&id) {
            let _ = session.stream.quit().await;
        }
    }

    async fn open_session(&self, conn: &Connection, log: &mut Vec<String>) -> AppResult<()> {
        let stream = establish_connection(conn, log).await?;
        let mut sessions = self.sessions.lock().await;
        sessions.insert(conn.id, ActiveSession { stream });
        Ok(())
    }

    async fn ensure_session(&self, conn: &Connection, log: &mut Vec<String>) -> AppResult<()> {
        // Verify the existing session is alive with a NOOP before reusing it.
        // This catches sessions left in inconsistent state after LIST on servers
        // that don't cleanly reset the control connection after a data transfer.
        let noop_ok = {
            let mut sessions = self.sessions.lock().await;
            match sessions.get_mut(&conn.id) {
                Some(session) => {
                    let result = tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        session.stream.noop(),
                    )
                    .await;
                    matches!(result, Ok(Ok(())))
                }
                None => false,
            }
        };

        if noop_ok {
            log.push(cmd("sesión reutilizada"));
            return Ok(());
        }

        // Session missing or stale — discard it silently and reconnect.
        {
            let mut sessions = self.sessions.lock().await;
            sessions.remove(&conn.id);
        }
        log.push(cmd("reconectando (sesión expirada)"));
        self.open_session(conn, log).await
    }

    async fn exec_list_dir(
        &self,
        id: &Uuid,
        path: &str,
        log: &mut Vec<String>,
    ) -> AppResult<(String, Vec<FtpEntry>)> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {path}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session.stream.cwd(path).await.map_err(AppError::from)
        })
        .await?;

        let current = with_timeout(OPERATION_TIMEOUT, async {
            session.stream.pwd().await.map_err(AppError::from)
        })
        .await?;

        log.push(cmd("LIST"));
        let list = with_timeout(OPERATION_TIMEOUT, async {
            session.stream.list(None).await.map_err(AppError::from)
        })
        .await?;

        let entries = list
            .iter()
            .filter_map(|line| FtpEntry::from_list_line(line))
            .collect();

        log.push(cmd(format!("PWD → {current}")));
        Ok((current, entries))
    }

    async fn exec_upload(
        &self,
        id: &Uuid,
        local_path: &std::path::Path,
        remote_path: &str,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        // Read the file before acquiring the session lock so the mutex is not
        // held during potentially slow local I/O (which would let the FTP
        // control connection idle-timeout between CWD and STOR).
        let file_bytes = tokio::fs::read(local_path).await?;
        let filename = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archivo")
            .to_string();

        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {remote_path}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .cwd(remote_path)
                .await
                .map_err(AppError::from)
        })
        .await?;

        log.push(cmd(format!("STOR {filename}")));
        let mut cursor = futures::io::Cursor::new(file_bytes);
        with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .put_file(&filename, &mut cursor)
                .await
                .map_err(AppError::from)
        })
        .await?;

        debug!(%filename, "upload complete");
        Ok(())
    }

    async fn exec_download(
        &self,
        id: &Uuid,
        remote_path: &str,
        filename: &str,
        local_dir: &std::path::Path,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {remote_path}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .cwd(remote_path)
                .await
                .map_err(AppError::from)
        })
        .await?;

        log.push(cmd(format!("RETR {filename}")));
        let mut stream_dl = with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .retr_as_stream(filename)
                .await
                .map_err(AppError::from)
        })
        .await?;

        let mut data = Vec::new();
        with_timeout(OPERATION_TIMEOUT, async {
            stream_dl
                .read_to_end(&mut data)
                .await
                .map_err(AppError::from)
        })
        .await?;

        with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .finalize_retr_stream(stream_dl)
                .await
                .map_err(AppError::from)
        })
        .await?;

        let dest = local_dir.join(filename);
        tokio::fs::write(&dest, data).await?;

        debug!(%filename, "download complete");
        Ok(())
    }
}

async fn establish_connection(
    conn: &Connection,
    log: &mut Vec<String>,
) -> AppResult<AsyncFtpStream> {
    let addr = format!("{}:{}", conn.host, conn.port);
    log.push(cmd(format!("CONNECT {addr}")));

    let mut stream =
        with_timeout(CONNECT_TIMEOUT, async { connect_with_timeout(&addr).await }).await?;

    log.push(cmd(format!("USER {}", conn.username)));
    with_timeout(OPERATION_TIMEOUT, async {
        stream
            .login(conn.username.as_str(), conn.password())
            .await
            .map_err(AppError::from)
    })
    .await?;

    Ok(stream)
}

async fn connect_with_timeout(addr: &str) -> AppResult<AsyncFtpStream> {
    let mut addrs = tokio::net::lookup_host(addr)
        .await
        .map_err(AppError::from)?;

    let socket_addr = addrs
        .next()
        .ok_or_else(|| AppError::Validation(format!("no se pudo resolver el host: {addr}")))?;

    AsyncFtpStream::connect_timeout(socket_addr, CONNECT_TIMEOUT)
        .await
        .map_err(AppError::from)
}
