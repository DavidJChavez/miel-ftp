use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures::channel::mpsc::UnboundedSender;
use futures::{AsyncReadExt, AsyncWriteExt};
use suppaftp::AsyncNativeTlsFtpStream;
use suppaftp::types::Mode as FtpDataMode;
use tokio::io::{
    AsyncReadExt as TokioAsyncReadExt, AsyncSeekExt, AsyncWriteExt as TokioAsyncWriteExt,
};
use tokio::sync::Mutex;
use tracing::{debug, warn};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::ftp::client::FtpResponse;
use crate::ftp::config::{self, CHUNK_SIZE, CONNECT_TIMEOUT, OPERATION_TIMEOUT};
use crate::ftp::retry::{is_retryable, with_timeout};
use crate::ftp::sftp::{self, SftpSessionHandle};
use crate::ftp::throttle;
use crate::ftp::tls;
use crate::models::{
    connection::{Connection, FtpMode, Protocol},
    ftp_entry::FtpEntry,
};

fn cmd(line: impl AsRef<str>) -> String {
    format!("> {}", line.as_ref())
}

macro_rules! ftp_retry {
    ($manager:expr, $operation:literal, $session_id:expr, $cancel:expr, $attempt:expr) => {{
        let mut last_err: Option<AppError> = None;
        let mut ok = None;

        for try_num in 0..config::MAX_RETRIES {
            if $cancel.load(Ordering::Relaxed) {
                last_err = Some(AppError::Cancelled);
                break;
            }

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
    stream: AsyncNativeTlsFtpStream,
}

/// Gestor de sesiones FTP/SFTP: un stream vivo por `connection.id`.
#[derive(Clone)]
pub struct FtpSessionManager {
    sessions: Arc<Mutex<HashMap<Uuid, ActiveSession>>>,
    sftp_sessions: Arc<Mutex<HashMap<Uuid, SftpSessionHandle>>>,
}

impl FtpSessionManager {
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            sftp_sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn disconnect(&self, id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        if let Some(mut session) = sessions.remove(&id) {
            let _ = session.stream.quit().await;
            debug!(%id, "sesión FTP cerrada");
        }
        drop(sessions);
        self.sftp_sessions.lock().await.remove(&id);
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
        drop(sessions);
        self.sftp_sessions.lock().await.clear();
    }

    fn is_sftp(conn: &Connection) -> bool {
        conn.effective_protocol() == Protocol::Sftp
    }

    pub async fn connect(&self, conn: Connection) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self.connect_sftp(conn).await;
        }

        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "connect", conn.id, cancel, async {
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
        if Self::is_sftp(&conn) {
            return self.list_dir_sftp(conn, path).await;
        }

        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "list_dir", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_list_dir(&conn.id, &path, &mut log).await
        });

        if let Err(e) = &result {
            log.push(format!("< LIST ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upload_stream(
        &self,
        conn: Connection,
        local_path: PathBuf,
        remote_path: String,
        resume_from: u64,
        cancel: Arc<AtomicBool>,
        progress: UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self
                .upload_stream_sftp(
                    conn,
                    local_path,
                    remote_path,
                    resume_from,
                    cancel,
                    progress,
                    limit_kbps,
                )
                .await;
        }

        let mut log = Vec::new();
        let result = ftp_retry!(self, "upload", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_upload_stream(
                &conn.id,
                &local_path,
                remote_path.as_str(),
                resume_from,
                &cancel,
                &progress,
                limit_kbps,
                &mut log,
            )
            .await
        });

        if cancel.load(Ordering::Relaxed) || matches!(&result, Err(AppError::Cancelled)) {
            log.push(cmd("CANCELLED"));
            self.invalidate(conn.id).await;
        } else if let Err(e) = &result {
            log.push(format!("< STOR ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn download_stream(
        &self,
        conn: Connection,
        remote_path: String,
        filename: String,
        local_dir: PathBuf,
        resume_from: u64,
        cancel: Arc<AtomicBool>,
        progress: UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self
                .download_stream_sftp(
                    conn,
                    remote_path,
                    filename,
                    local_dir,
                    resume_from,
                    cancel,
                    progress,
                    limit_kbps,
                )
                .await;
        }

        let mut log = Vec::new();
        let result = ftp_retry!(self, "download", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_download_stream(
                &conn.id,
                remote_path.as_str(),
                filename.as_str(),
                local_dir.as_path(),
                resume_from,
                &cancel,
                &progress,
                limit_kbps,
                &mut log,
            )
            .await
        });

        if cancel.load(Ordering::Relaxed) || matches!(&result, Err(AppError::Cancelled)) {
            log.push(cmd("CANCELLED"));
            self.invalidate(conn.id).await;
        } else if let Err(e) = &result {
            log.push(format!("< RETR ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn mkdir(
        &self,
        conn: Connection,
        remote_dir: String,
        name: String,
    ) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self.mkdir_sftp(conn, remote_dir, name).await;
        }

        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "mkdir", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_mkdir(&conn.id, remote_dir.as_str(), name.as_str(), &mut log)
                .await
        });

        if let Err(e) = &result {
            log.push(format!("< MKD ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn rename(
        &self,
        conn: Connection,
        remote_dir: String,
        old: String,
        new: String,
    ) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self.rename_sftp(conn, remote_dir, old, new).await;
        }

        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "rename", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_rename(
                &conn.id,
                remote_dir.as_str(),
                old.as_str(),
                new.as_str(),
                &mut log,
            )
            .await
        });

        if let Err(e) = &result {
            log.push(format!("< RNFR/RNTO ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    pub async fn remove(
        &self,
        conn: Connection,
        remote_dir: String,
        name: String,
        is_dir: bool,
    ) -> FtpResponse<()> {
        if Self::is_sftp(&conn) {
            return self.remove_sftp(conn, remote_dir, name, is_dir).await;
        }

        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "remove", conn.id, cancel, async {
            self.ensure_session(&conn, &mut log).await?;
            self.exec_remove(
                &conn.id,
                remote_dir.as_str(),
                name.as_str(),
                is_dir,
                &mut log,
            )
            .await
        });

        if let Err(e) = &result {
            log.push(format!("< DELETE ERROR: {e}"));
        }

        FtpResponse { result, log }
    }

    async fn invalidate(&self, id: Uuid) {
        let mut sessions = self.sessions.lock().await;
        if let Some(mut session) = sessions.remove(&id) {
            let _ = session.stream.quit().await;
        }
        drop(sessions);
        self.sftp_sessions.lock().await.remove(&id);
    }

    async fn open_session(&self, conn: &Connection, log: &mut Vec<String>) -> AppResult<()> {
        let stream = establish_connection(conn, log).await?;
        let mut sessions = self.sessions.lock().await;
        sessions.insert(conn.id, ActiveSession { stream });
        Ok(())
    }

    async fn ensure_session(&self, conn: &Connection, log: &mut Vec<String>) -> AppResult<()> {
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

    async fn exec_mkdir(
        &self,
        id: &Uuid,
        remote_dir: &str,
        name: &str,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {remote_dir}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session.stream.cwd(remote_dir).await.map_err(AppError::from)
        })
        .await?;

        log.push(cmd(format!("MKD {name}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session.stream.mkdir(name).await.map_err(AppError::from)
        })
        .await?;

        Ok(())
    }

    async fn exec_rename(
        &self,
        id: &Uuid,
        remote_dir: &str,
        old: &str,
        new: &str,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {remote_dir}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session.stream.cwd(remote_dir).await.map_err(AppError::from)
        })
        .await?;

        log.push(cmd(format!("RNFR {old} → {new}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session
                .stream
                .rename(old, new)
                .await
                .map_err(AppError::from)
        })
        .await?;

        Ok(())
    }

    async fn exec_remove(
        &self,
        id: &Uuid,
        remote_dir: &str,
        name: &str,
        is_dir: bool,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let mut sessions = self.sessions.lock().await;
        let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;

        log.push(cmd(format!("CWD {remote_dir}")));
        with_timeout(OPERATION_TIMEOUT, async {
            session.stream.cwd(remote_dir).await.map_err(AppError::from)
        })
        .await?;

        if is_dir {
            log.push(cmd(format!("RMD {name}")));
            with_timeout(OPERATION_TIMEOUT, async {
                session.stream.rmdir(name).await.map_err(AppError::from)
            })
            .await?;
        } else {
            log.push(cmd(format!("DELE {name}")));
            with_timeout(OPERATION_TIMEOUT, async {
                session.stream.rm(name).await.map_err(AppError::from)
            })
            .await?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    async fn exec_upload_stream(
        &self,
        id: &Uuid,
        local_path: &std::path::Path,
        remote_path: &str,
        resume_from: u64,
        cancel: &Arc<AtomicBool>,
        progress: &UnboundedSender<u64>,
        limit_kbps: Option<u32>,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let filename = local_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archivo")
            .to_string();

        let mut file = tokio::fs::File::open(local_path).await?;
        if resume_from > 0 {
            file.seek(std::io::SeekFrom::Start(resume_from))
                .await
                .map_err(AppError::from)?;
        }

        let mut transferred: u64 = resume_from;
        if resume_from > 0 {
            let _ = progress.unbounded_send(transferred);
        }

        let mut buf = vec![0u8; CHUNK_SIZE];

        let mut data_stream = {
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

            if resume_from > 0 {
                log.push(cmd(format!("REST {resume_from}")));
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .resume_transfer(resume_from as usize)
                        .await
                        .map_err(AppError::from)
                })
                .await?;

                log.push(cmd(format!("APPE {filename}")));
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .append_with_stream(&filename)
                        .await
                        .map_err(AppError::from)
                })
                .await?
            } else {
                log.push(cmd(format!("STOR {filename}")));
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .put_with_stream(&filename)
                        .await
                        .map_err(AppError::from)
                })
                .await?
            }
        };

        let transfer_result = async {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    let _ = data_stream.close().await;
                    drop(data_stream);
                    return Err(AppError::Cancelled);
                }

                let n = file.read(&mut buf).await.map_err(AppError::from)?;
                if n == 0 {
                    break;
                }

                with_timeout(OPERATION_TIMEOUT, async {
                    data_stream
                        .write_all(&buf[..n])
                        .await
                        .map_err(AppError::from)
                })
                .await?;

                transferred += n as u64;
                let _ = progress.unbounded_send(transferred);
                throttle::sleep_for_bytes(n as u64, limit_kbps).await;
            }
            Ok(data_stream)
        }
        .await;

        match transfer_result {
            Ok(data_stream) => {
                let mut sessions = self.sessions.lock().await;
                let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .finalize_put_stream(data_stream)
                        .await
                        .map_err(AppError::from)
                })
                .await?;
                debug!(%filename, bytes = transferred, "upload complete");
                Ok(())
            }
            Err(AppError::Cancelled) => Err(AppError::Cancelled),
            Err(e) => Err(e),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn exec_download_stream(
        &self,
        id: &Uuid,
        remote_path: &str,
        filename: &str,
        local_dir: &std::path::Path,
        resume_from: u64,
        cancel: &Arc<AtomicBool>,
        progress: &UnboundedSender<u64>,
        limit_kbps: Option<u32>,
        log: &mut Vec<String>,
    ) -> AppResult<()> {
        let dest = local_dir.join(filename);
        let mut file = if resume_from > 0 {
            tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&dest)
                .await?
        } else {
            tokio::fs::File::create(&dest).await?
        };

        let mut transferred: u64 = resume_from;
        if resume_from > 0 {
            let _ = progress.unbounded_send(transferred);
        }

        let mut buf = vec![0u8; CHUNK_SIZE];

        let mut data_stream = {
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

            if resume_from > 0 {
                log.push(cmd(format!("REST {resume_from}")));
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .resume_transfer(resume_from as usize)
                        .await
                        .map_err(AppError::from)
                })
                .await?;
            }

            log.push(cmd(format!("RETR {filename}")));
            with_timeout(OPERATION_TIMEOUT, async {
                session
                    .stream
                    .retr_as_stream(filename)
                    .await
                    .map_err(AppError::from)
            })
            .await?
        };

        let transfer_result = async {
            loop {
                if cancel.load(Ordering::Relaxed) {
                    drop(data_stream);
                    return Err(AppError::Cancelled);
                }

                let n = with_timeout(OPERATION_TIMEOUT, async {
                    data_stream.read(&mut buf).await.map_err(AppError::from)
                })
                .await?;

                if n == 0 {
                    break;
                }

                file.write_all(&buf[..n]).await.map_err(AppError::from)?;
                transferred += n as u64;
                let _ = progress.unbounded_send(transferred);
                throttle::sleep_for_bytes(n as u64, limit_kbps).await;
            }
            Ok(data_stream)
        }
        .await;

        match transfer_result {
            Ok(data_stream) => {
                let mut sessions = self.sessions.lock().await;
                let session = sessions.get_mut(id).ok_or(AppError::NoConnection)?;
                with_timeout(OPERATION_TIMEOUT, async {
                    session
                        .stream
                        .finalize_retr_stream(data_stream)
                        .await
                        .map_err(AppError::from)
                })
                .await?;
                debug!(%filename, bytes = transferred, "download complete");
                Ok(())
            }
            Err(AppError::Cancelled) => Err(AppError::Cancelled),
            Err(e) => Err(e),
        }
    }
}

impl FtpSessionManager {
    async fn connect_sftp(&self, conn: Connection) -> FtpResponse<()> {
        let mut log = Vec::new();
        log.push(cmd(format!("SFTP CONNECT {}:{}", conn.host, conn.port)));
        self.invalidate(conn.id).await;
        match SftpSessionHandle::connect(&conn).await {
            Ok(handle) => {
                self.sftp_sessions.lock().await.insert(conn.id, handle);
                log.push(cmd("SFTP autenticado"));
                FtpResponse {
                    result: Ok(()),
                    log,
                }
            }
            Err(e) => {
                log.push(format!("< SFTP ERROR: {e}"));
                FtpResponse {
                    result: Err(e),
                    log,
                }
            }
        }
    }

    async fn ensure_sftp_session(&self, conn: &Connection, log: &mut Vec<String>) -> AppResult<()> {
        if self.sftp_sessions.lock().await.contains_key(&conn.id) {
            log.push(cmd("sesión SFTP reutilizada"));
            return Ok(());
        }
        log.push(cmd("reconectando SFTP"));
        self.invalidate(conn.id).await;
        let handle = SftpSessionHandle::connect(conn).await?;
        self.sftp_sessions.lock().await.insert(conn.id, handle);
        Ok(())
    }

    async fn list_dir_sftp(
        &self,
        conn: Connection,
        path: String,
    ) -> FtpResponse<(String, Vec<FtpEntry>)> {
        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let result = ftp_retry!(self, "list_dir_sftp", conn.id, cancel, async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP LIST {path}")));
            let entries = session.list_dir(&path).await?;
            Ok((path.clone(), entries))
        });
        if let Err(e) = &result {
            log.push(format!("< SFTP LIST ERROR: {e}"));
        }
        FtpResponse { result, log }
    }

    #[allow(clippy::too_many_arguments)]
    async fn upload_stream_sftp(
        &self,
        conn: Connection,
        local_path: PathBuf,
        remote_path: String,
        resume_from: u64,
        cancel: Arc<AtomicBool>,
        progress: UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let remote_file = sftp::join_remote(
            &remote_path,
            local_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("file"),
        );
        let result = ftp_retry!(self, "upload_sftp", conn.id, cancel.clone(), async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP PUT {remote_file}")));
            session
                .upload(
                    local_path.clone(),
                    &remote_file,
                    resume_from,
                    &cancel,
                    &progress,
                    limit_kbps,
                )
                .await
        });
        if cancel.load(Ordering::Relaxed) || matches!(&result, Err(AppError::Cancelled)) {
            log.push(cmd("CANCELLED"));
            self.invalidate(conn.id).await;
        } else if let Err(e) = &result {
            log.push(format!("< SFTP PUT ERROR: {e}"));
        }
        FtpResponse { result, log }
    }

    #[allow(clippy::too_many_arguments)]
    async fn download_stream_sftp(
        &self,
        conn: Connection,
        remote_path: String,
        filename: String,
        local_dir: PathBuf,
        resume_from: u64,
        cancel: Arc<AtomicBool>,
        progress: UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let remote_file = sftp::join_remote(&remote_path, &filename);
        let result = ftp_retry!(self, "download_sftp", conn.id, cancel.clone(), async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP GET {remote_file}")));
            session
                .download(
                    &remote_file,
                    local_dir.as_path(),
                    &filename,
                    resume_from,
                    &cancel,
                    &progress,
                    limit_kbps,
                )
                .await
        });
        if cancel.load(Ordering::Relaxed) || matches!(&result, Err(AppError::Cancelled)) {
            log.push(cmd("CANCELLED"));
            self.invalidate(conn.id).await;
        } else if let Err(e) = &result {
            log.push(format!("< SFTP GET ERROR: {e}"));
        }
        FtpResponse { result, log }
    }

    async fn mkdir_sftp(
        &self,
        conn: Connection,
        remote_dir: String,
        name: String,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let path = sftp::join_remote(&remote_dir, &name);
        let result = ftp_retry!(self, "mkdir_sftp", conn.id, cancel, async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP MKDIR {path}")));
            session.mkdir(&path).await
        });
        if let Err(e) = &result {
            log.push(format!("< SFTP MKDIR ERROR: {e}"));
        }
        FtpResponse { result, log }
    }

    async fn rename_sftp(
        &self,
        conn: Connection,
        remote_dir: String,
        old: String,
        new: String,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let from = sftp::join_remote(&remote_dir, &old);
        let to = sftp::join_remote(&remote_dir, &new);
        let result = ftp_retry!(self, "rename_sftp", conn.id, cancel, async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP RENAME {from} → {to}")));
            session.rename(&from, &to).await
        });
        if let Err(e) = &result {
            log.push(format!("< SFTP RENAME ERROR: {e}"));
        }
        FtpResponse { result, log }
    }

    async fn remove_sftp(
        &self,
        conn: Connection,
        remote_dir: String,
        name: String,
        is_dir: bool,
    ) -> FtpResponse<()> {
        let mut log = Vec::new();
        let cancel = Arc::new(AtomicBool::new(false));
        let path = sftp::join_remote(&remote_dir, &name);
        let result = ftp_retry!(self, "remove_sftp", conn.id, cancel, async {
            self.ensure_sftp_session(&conn, &mut log).await?;
            let mut sessions = self.sftp_sessions.lock().await;
            let session = sessions.get_mut(&conn.id).ok_or(AppError::NoConnection)?;
            log.push(cmd(format!("SFTP DELETE {path}")));
            session.remove(&path, is_dir).await
        });
        if let Err(e) = &result {
            log.push(format!("< SFTP DELETE ERROR: {e}"));
        }
        FtpResponse { result, log }
    }
}

async fn establish_connection(
    conn: &Connection,
    log: &mut Vec<String>,
) -> AppResult<AsyncNativeTlsFtpStream> {
    let addr = format!("{}:{}", conn.host, conn.port);
    log.push(cmd(format!("CONNECT {addr}")));

    let mut stream =
        with_timeout(CONNECT_TIMEOUT, async { connect_with_timeout(&addr).await }).await?;

    if conn.effective_protocol() == Protocol::FtpsExplicit {
        log.push(cmd("AUTH TLS"));
        let connector = tls::build_tls_connector(conn.accept_invalid_certs)?;
        stream = with_timeout(OPERATION_TIMEOUT, async {
            stream
                .into_secure(connector, conn.host.as_str())
                .await
                .map_err(AppError::from)
        })
        .await?;
        log.push(cmd("PBSZ 0"));
        log.push(cmd("PROT P"));
    }

    log.push(cmd(format!("USER {}", conn.username)));
    with_timeout(OPERATION_TIMEOUT, async {
        stream
            .login(conn.username.as_str(), conn.password())
            .await
            .map_err(AppError::from)
    })
    .await?;

    let data_mode = match conn.mode {
        FtpMode::Passive => FtpDataMode::Passive,
        FtpMode::Active => FtpDataMode::Active,
    };
    log.push(cmd(format!("MODE {}", conn.mode.label())));
    stream.set_mode(data_mode);

    Ok(stream)
}

async fn connect_with_timeout(addr: &str) -> AppResult<AsyncNativeTlsFtpStream> {
    let mut addrs = tokio::net::lookup_host(addr)
        .await
        .map_err(AppError::from)?;

    let socket_addr = addrs
        .next()
        .ok_or_else(|| AppError::Validation(format!("no se pudo resolver el host: {addr}")))?;

    AsyncNativeTlsFtpStream::connect_timeout(socket_addr, CONNECT_TIMEOUT)
        .await
        .map_err(AppError::from)
}
