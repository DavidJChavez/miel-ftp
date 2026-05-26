use std::path::{Path, PathBuf};

use futures::AsyncReadExt;
use suppaftp::AsyncFtpStream;
use tracing::{debug, error, instrument};

use crate::error::{AppError, AppResult};
use crate::models::{connection::Connection, ftp_entry::FtpEntry};

pub struct FtpResponse<T> {
    pub result: AppResult<T>,
    pub log: Vec<String>,
}

fn cmd(line: impl AsRef<str>) -> String {
    format!("> {}", line.as_ref())
}

#[instrument(skip(conn), fields(host = %conn.host, port = conn.port))]
pub async fn upload(conn: Connection, local_path: PathBuf, remote_path: String) -> FtpResponse<()> {
    let addr = format!("{}:{}", conn.host, conn.port);
    let mut log = vec![
        cmd(format!("CONNECT {addr}")),
        cmd(format!("USER {}", conn.username)),
        cmd("PASS ***"),
    ];

    let result = run_upload(&conn, &addr, &remote_path, &local_path, &mut log).await;

    if let Err(e) = &result {
        log.push(format!("< STOR ERROR: {e}"));
    }

    FtpResponse { result, log }
}

async fn run_upload(
    conn: &Connection,
    addr: &str,
    remote_path: &str,
    local_path: &PathBuf,
    log: &mut Vec<String>,
) -> AppResult<()> {
    let mut ftp = AsyncFtpStream::connect(addr).await?;
    ftp.login(conn.username.as_str(), conn.password()).await?;
    log.push(cmd(format!("CWD {remote_path}")));
    ftp.cwd(remote_path).await?;

    let file_bytes = tokio::fs::read(local_path).await?;

    let filename = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archivo")
        .to_string();

    log.push(cmd(format!("STOR {filename}")));
    let mut cursor = futures::io::Cursor::new(file_bytes);
    ftp.put_file(&filename, &mut cursor).await?;

    log.push(cmd("QUIT"));
    let _ = ftp.quit().await;
    debug!(%filename, "upload complete");
    Ok(())
}

#[instrument(skip(conn), fields(host = %conn.host, port = conn.port))]
pub async fn download(
    conn: Connection,
    remote_path: String,
    filename: String,
    local_dir: PathBuf,
) -> FtpResponse<()> {
    let addr = format!("{}:{}", conn.host, conn.port);
    let mut log = vec![
        cmd(format!("CONNECT {addr}")),
        cmd(format!("USER {}", conn.username)),
        cmd("PASS ***"),
    ];

    let result = run_download(&conn, &addr, &remote_path, &filename, &local_dir, &mut log).await;

    if let Err(e) = &result {
        log.push(format!("< RETR ERROR: {e}"));
    }

    FtpResponse { result, log }
}

async fn run_download(
    conn: &Connection,
    addr: &str,
    remote_path: &str,
    filename: &str,
    local_dir: &Path,
    log: &mut Vec<String>,
) -> AppResult<()> {
    let mut ftp = AsyncFtpStream::connect(addr).await?;
    ftp.login(conn.username.as_str(), conn.password()).await?;
    log.push(cmd(format!("CWD {remote_path}")));
    ftp.cwd(remote_path).await?;

    log.push(cmd(format!("RETR {filename}")));
    let mut stream = ftp.retr_as_stream(filename).await?;

    let mut data = Vec::new();
    stream.read_to_end(&mut data).await?;

    ftp.finalize_retr_stream(stream).await?;

    let dest = local_dir.join(filename);
    tokio::fs::write(&dest, data).await?;

    log.push(cmd("QUIT"));
    let _ = ftp.quit().await;
    debug!(%filename, "download complete");
    Ok(())
}

#[instrument(skip(conn), fields(host = %conn.host, port = conn.port))]
pub async fn connect(conn: Connection) -> FtpResponse<()> {
    let addr = format!("{}:{}", conn.host, conn.port);
    let mut log = vec![
        cmd(format!("CONNECT {addr}")),
        cmd(format!("USER {}", conn.username)),
        cmd("PASS ***"),
    ];

    let result = run_connect(&conn, &addr, &mut log).await;

    if let Err(e) = &result {
        log.push(format!("< LOGIN ERROR: {e}"));
    }

    FtpResponse { result, log }
}

async fn run_connect(conn: &Connection, addr: &str, log: &mut Vec<String>) -> AppResult<()> {
    let mut ftp = AsyncFtpStream::connect(addr).await?;
    ftp.login(conn.username.as_str(), conn.password()).await?;
    log.push(cmd("QUIT"));
    let _ = ftp.quit().await;
    Ok(())
}

#[instrument(skip(conn), fields(host = %conn.host, port = conn.port, %path))]
pub async fn list_dir(conn: Connection, path: String) -> FtpResponse<(String, Vec<FtpEntry>)> {
    let addr = format!("{}:{}", conn.host, conn.port);
    let mut log = vec![
        cmd(format!("CONNECT {addr}")),
        cmd(format!("USER {}", conn.username)),
        cmd("PASS ***"),
    ];

    let result = run_list_dir(&conn, &addr, &path, &mut log).await;

    if let Err(e) = &result {
        log.push(format!("< LIST ERROR: {e}"));
    }

    FtpResponse { result, log }
}

async fn run_list_dir(
    conn: &Connection,
    addr: &str,
    path: &str,
    log: &mut Vec<String>,
) -> AppResult<(String, Vec<FtpEntry>)> {
    let mut ftp = AsyncFtpStream::connect(addr).await?;
    ftp.login(conn.username.as_str(), conn.password()).await?;
    log.push(cmd(format!("CWD {path}")));
    ftp.cwd(path).await?;

    let current = ftp.pwd().await?;
    log.push(cmd("LIST"));
    let list = ftp.list(None).await?;

    let entries = list
        .iter()
        .filter_map(|line| FtpEntry::from_list_line(line))
        .collect();

    log.push(cmd(format!("PWD → {current}")));
    log.push(cmd("QUIT"));
    let _ = ftp.quit().await;

    Ok((current, entries))
}

pub fn log_ftp_error(context: &str, err: &AppError) {
    error!(%context, error = %err, "FTP operation failed");
}
