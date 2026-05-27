use std::path::PathBuf;

use secrecy::SecretString;

use crate::error::{AppError, AppResult};
use crate::models::connection::{Connection, ConnectionRecord};

fn config_dir() -> AppResult<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("no se encontró el directorio de configuración".into()))?
        .join("miel-ftp");
    Ok(dir)
}

fn sites_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join("sites.json"))
}

pub fn load_connections() -> AppResult<Vec<Connection>> {
    let path = sites_path()?;
    if !path.exists() {
        return Ok(Vec::new());
    }

    let data = std::fs::read_to_string(&path)?;
    let records: Vec<ConnectionRecord> = serde_json::from_str(&data)?;

    records
        .into_iter()
        .map(|record| {
            let password = Connection::load_password(record.id)
                .unwrap_or_else(|_| SecretString::from(String::new()));
            Ok(Connection::from_record(record, password))
        })
        .collect()
}

pub fn save_connections(connections: &[Connection]) -> AppResult<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)?;

    let records: Vec<ConnectionRecord> = connections.iter().map(|c| c.to_record()).collect();
    let data = serde_json::to_string_pretty(&records)?;

    let path = sites_path()?;
    let tmp = dir.join("sites.json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, path)?;

    for conn in connections {
        conn.store_password()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;
    use uuid::Uuid;

    use super::*;
    use crate::models::connection::{Connection, FtpMode, FtpSecurity};

    #[test]
    fn connection_record_round_trip_json() {
        let record = ConnectionRecord {
            id: Uuid::nil(),
            name: "Test".into(),
            host: "127.0.0.1".into(),
            port: 2121,
            username: "user".into(),
            mode: FtpMode::Passive,
            security: FtpSecurity::Plain,
            accept_invalid_certs: false,
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: ConnectionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn connection_try_new_validates_empty_host() {
        let result = Connection::try_new(
            None,
            "Site".into(),
            "  ".into(),
            21,
            "user".into(),
            SecretString::from("pass".to_string()),
            FtpMode::Passive,
            FtpSecurity::Plain,
            false,
        );
        assert!(matches!(result, Err(AppError::Validation(_))));
    }
}
