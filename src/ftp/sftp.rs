use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use futures::channel::mpsc::UnboundedSender;
use russh::client;
use russh_keys::key;
use russh_sftp::client::SftpSession;
use tokio::io::{
    AsyncReadExt as TokioAsyncReadExt, AsyncSeekExt, AsyncWriteExt as TokioAsyncWriteExt,
};
use tracing::debug;

use crate::error::{AppError, AppResult};
use crate::ftp::config::CHUNK_SIZE;
use crate::ftp::throttle;
use crate::models::connection::Connection;
use crate::models::ftp_entry::FtpEntry;

struct SshHandler {
    accept_invalid_certs: bool,
}

#[async_trait]
impl client::Handler for SshHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(self.accept_invalid_certs)
    }
}

pub struct SftpSessionHandle {
    sftp: SftpSession,
}

impl SftpSessionHandle {
    pub async fn connect(conn: &Connection) -> AppResult<Self> {
        let config = Arc::new(client::Config::default());
        let handler = SshHandler {
            accept_invalid_certs: conn.accept_invalid_certs,
        };
        let addr = (conn.host.as_str(), conn.port);
        let mut session = client::connect(config, addr, handler)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        let authenticated = session
            .authenticate_password(conn.username.as_str(), conn.password())
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        if !authenticated {
            return Err(AppError::Connection("autenticación SFTP fallida".into()));
        }

        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;
        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        Ok(Self { sftp })
    }

    pub async fn list_dir(&mut self, path: &str) -> AppResult<Vec<FtpEntry>> {
        let read_dir = self
            .sftp
            .read_dir(path)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        let mut entries = Vec::new();
        for entry in read_dir {
            let name = entry.file_name();
            let attrs = entry.metadata();
            let is_dir = attrs.is_dir();
            let size = if is_dir { None } else { Some(attrs.len()) };
            let modified = attrs.modified().ok().map(format_system_time);
            entries.push(FtpEntry {
                name,
                is_dir,
                size,
                modified,
            });
        }
        Ok(entries)
    }

    pub async fn mkdir(&mut self, path: &str) -> AppResult<()> {
        self.sftp
            .create_dir(path)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))
    }

    pub async fn rename(&mut self, from: &str, to: &str) -> AppResult<()> {
        self.sftp
            .rename(from, to)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))
    }

    pub async fn remove(&mut self, path: &str, is_dir: bool) -> AppResult<()> {
        if is_dir {
            self.sftp
                .remove_dir(path)
                .await
                .map_err(|e| AppError::Connection(e.to_string()))
        } else {
            self.sftp
                .remove_file(path)
                .await
                .map_err(|e| AppError::Connection(e.to_string()))
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn upload(
        &mut self,
        local_path: PathBuf,
        remote_path: &str,
        resume_from: u64,
        cancel: &Arc<AtomicBool>,
        progress: &UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> AppResult<()> {
        let mut file = tokio::fs::File::open(&local_path)
            .await
            .map_err(AppError::from)?;
        if resume_from > 0 {
            file.seek(std::io::SeekFrom::Start(resume_from))
                .await
                .map_err(AppError::from)?;
        }

        let mut remote = self
            .sftp
            .create(remote_path)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;

        let mut transferred = resume_from;
        let mut buf = vec![0u8; CHUNK_SIZE];

        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }

            let n = file.read(&mut buf).await.map_err(AppError::from)?;
            if n == 0 {
                break;
            }

            remote
                .write_all(&buf[..n])
                .await
                .map_err(|e| AppError::Connection(e.to_string()))?;
            transferred += n as u64;
            let _ = progress.unbounded_send(transferred);
            throttle::sleep_for_bytes(n as u64, limit_kbps).await;
        }

        remote
            .shutdown()
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;
        debug!(remote_path, bytes = transferred, "sftp upload complete");
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn download(
        &mut self,
        remote_path: &str,
        local_dir: &Path,
        filename: &str,
        resume_from: u64,
        cancel: &Arc<AtomicBool>,
        progress: &UnboundedSender<u64>,
        limit_kbps: Option<u32>,
    ) -> AppResult<()> {
        tokio::fs::create_dir_all(local_dir)
            .await
            .map_err(AppError::from)?;
        let local_path = local_dir.join(filename);
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(resume_from == 0)
            .open(&local_path)
            .await
            .map_err(AppError::from)?;
        if resume_from > 0 {
            file.seek(std::io::SeekFrom::Start(resume_from))
                .await
                .map_err(AppError::from)?;
        }

        let mut remote = self
            .sftp
            .open(remote_path)
            .await
            .map_err(|e| AppError::Connection(e.to_string()))?;
        if resume_from > 0 {
            remote
                .seek(std::io::SeekFrom::Start(resume_from))
                .await
                .map_err(|e| AppError::Connection(e.to_string()))?;
        }

        let mut transferred = resume_from;
        let mut buf = vec![0u8; CHUNK_SIZE];

        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(AppError::Cancelled);
            }

            let n = remote
                .read(&mut buf)
                .await
                .map_err(|e| AppError::Connection(e.to_string()))?;
            if n == 0 {
                break;
            }

            file.write_all(&buf[..n]).await.map_err(AppError::from)?;
            transferred += n as u64;
            let _ = progress.unbounded_send(transferred);
            throttle::sleep_for_bytes(n as u64, limit_kbps).await;
        }

        debug!(filename, bytes = transferred, "sftp download complete");
        Ok(())
    }
}

pub fn join_remote(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}

fn format_system_time(time: SystemTime) -> String {
    chrono::DateTime::<chrono::Utc>::from(time)
        .format("%Y-%m-%d %H:%M")
        .to_string()
}

pub fn parse_modified(entry: &FtpEntry) -> Option<SystemTime> {
    entry.modified.as_ref().and_then(|s| {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M")
            .ok()
            .map(|dt| UNIX_EPOCH + Duration::from_secs(dt.and_utc().timestamp() as u64))
    })
}
