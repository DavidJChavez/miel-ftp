use std::path::PathBuf;
use suppaftp::{AsyncFtpStream, FtpError};
use tokio::io::AsyncReadExt;

use crate::app::{Connection, FtpEntry};

pub async fn upload(
    conn: Connection,
    local_path: PathBuf,
    remote_path: String,
) -> Result<(), String> {
    let addr = format!("{}:{}", conn.host, conn.port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| e.to_string())?;

    ftp.login(&conn.username, &conn.password)
        .await
        .map_err(|e| e.to_string())?;

    ftp.cwd(&remote_path).await.map_err(|e| e.to_string())?;

    let file_bytes = tokio::fs::read(&local_path)
        .await
        .map_err(|e| e.to_string())?;

    let filename = local_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archivo")
        .to_string();

    let mut cursor = futures::io::Cursor::new(file_bytes);

    ftp.put_file(&filename, &mut cursor)
        .await
        .map_err(|e| e.to_string())?;

    let _ = ftp.quit().await;

    Ok(())
}

pub async fn download(
    conn: Connection,
    remote_path: String,
    filename: String,
    local_dir: PathBuf,
) -> Result<(), String> {
    let addr = format!("{}:{}", conn.host, conn.port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| e.to_string())?;

    ftp.login(&conn.username, &conn.password)
        .await
        .map_err(|e| e.to_string())?;

    ftp.cwd(&remote_path).await.map_err(|e| e.to_string())?;

    let mut stream = ftp
        .retr_as_stream(&filename)
        .await
        .map_err(|e| e.to_string())?;

    let mut data = Vec::new();
    futures::io::AsyncReadExt::read_to_end(&mut stream, &mut data)
        .await
        .map_err(|e| e.to_string())?;

    ftp.finalize_retr_stream(stream)
        .await
        .map_err(|e| e.to_string())?;

    let dest = local_dir.join(&filename);

    tokio::fs::write(&dest, data)
        .await
        .map_err(|e| e.to_string())?;

    let _ = ftp.quit().await;

    Ok(())
}

pub async fn connect(conn: Connection) -> Result<(), String> {
    let addr = format!("{}:{}", conn.host, conn.port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| e.to_string())?;

    ftp.login(&conn.username, &conn.password)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn list_dir(conn: Connection, path: String) -> Result<(String, Vec<FtpEntry>), String> {
    let addr = format!("{}:{}", conn.host, conn.port);

    let mut ftp = AsyncFtpStream::connect(&addr)
        .await
        .map_err(|e| e.to_string())?;

    ftp.login(&conn.username, &conn.password)
        .await
        .map_err(|e| e.to_string())?;

    ftp.cwd(&path).await.map_err(|e| e.to_string())?;

    let current = ftp.pwd().await.map_err(|e| e.to_string())?;

    let list = ftp.list(None).await.map_err(|e| e.to_string())?;

    let entries = list
        .iter()
        .filter_map(|line| parse_list_line(line))
        .collect();

    let _ = ftp.quit().await;

    Ok((current, entries))
}

// parses lines of LIST unix-style:
// drwxr-xr-x 2 user group 4096 Apr 10 12:00 folder
// -rw-r--r-- 1 user group 1234 Apr 10 12:00 file.txt
fn parse_list_line(line: &str) -> Option<FtpEntry> {
    let parts: Vec<&str> = line.split_whitespace().collect();

    // Format: perms links owner group size month day hour/year name
    if parts.len() < 9 {
        return None;
    }

    let is_dir = line.starts_with('d');
    let size = if is_dir {
        None
    } else {
        parts[4].parse::<u64>().ok()
    };
    let modified = format!("{} {} {}", parts[5], parts[6], parts[7]);

    // Find the 9th token position at original line
    // then take what's left
    let name = extract_name(line, 8)?;

    // ignorar . y ..
    if name == "." || name == ".." {
        return None;
    }

    Some(FtpEntry {
        name,
        is_dir,
        size,
        modified: Some(modified),
    })
}

fn extract_name(line: &str, skip_tokens: usize) -> Option<String> {
    let mut tokens_seen = 0;
    let mut in_token = false;
    let mut name_start = 0;

    for (i, ch) in line.char_indices() {
        let is_space = ch == ' ';

        if !is_space && !in_token {
            in_token = true;
            if tokens_seen == skip_tokens {
                name_start = i;
                return Some(line[name_start..].to_string());
            }
        } else if is_space && in_token {
            in_token = false;
            tokens_seen += 1;
        }
    }

    None
}
