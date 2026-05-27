use std::fmt;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "miel-ftp";

/// Modo de canal de datos FTP por conexión.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FtpMode {
    #[default]
    Passive,
    Active,
}

impl FtpMode {
    pub fn label(self) -> &'static str {
        match self {
            FtpMode::Passive => "passive",
            FtpMode::Active => "active",
        }
    }
}

/// Seguridad de transporte FTP.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum FtpSecurity {
    #[default]
    Plain,
    Explicit,
}

/// Registro persistido en JSON (sin contraseña; va en keyring).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionRecord {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub mode: FtpMode,
    #[serde(default)]
    pub security: FtpSecurity,
    #[serde(default)]
    pub accept_invalid_certs: bool,
}

#[derive(Clone)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub mode: FtpMode,
    pub security: FtpSecurity,
    pub accept_invalid_certs: bool,
    password: SecretString,
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Connection")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("mode", &self.mode)
            .field("security", &self.security)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl Connection {
    #[allow(clippy::too_many_arguments)]
    pub fn try_new(
        id: Option<Uuid>,
        name: String,
        host: String,
        port: u16,
        username: String,
        password: impl Into<SecretString>,
        mode: FtpMode,
        security: FtpSecurity,
        accept_invalid_certs: bool,
    ) -> AppResult<Self> {
        if name.trim().is_empty() {
            return Err(AppError::Validation(
                "el nombre no puede estar vacío".into(),
            ));
        }
        if host.trim().is_empty() {
            return Err(AppError::Validation("el host no puede estar vacío".into()));
        }
        if port == 0 {
            return Err(AppError::Validation(
                "el puerto debe ser mayor que 0".into(),
            ));
        }
        if username.trim().is_empty() {
            return Err(AppError::Validation(
                "el usuario no puede estar vacío".into(),
            ));
        }

        Ok(Self {
            id: id.unwrap_or_else(Uuid::new_v4),
            name: name.trim().to_string(),
            host: host.trim().to_string(),
            port,
            username: username.trim().to_string(),
            mode,
            security,
            accept_invalid_certs,
            password: password.into(),
        })
    }

    pub fn password(&self) -> &str {
        self.password.expose_secret()
    }

    pub fn to_record(&self) -> ConnectionRecord {
        ConnectionRecord {
            id: self.id,
            name: self.name.clone(),
            host: self.host.clone(),
            port: self.port,
            username: self.username.clone(),
            mode: self.mode,
            security: self.security,
            accept_invalid_certs: self.accept_invalid_certs,
        }
    }

    pub fn from_record(record: ConnectionRecord, password: SecretString) -> Self {
        Self {
            id: record.id,
            name: record.name,
            host: record.host,
            port: record.port,
            username: record.username,
            mode: record.mode,
            security: record.security,
            accept_invalid_certs: record.accept_invalid_certs,
            password,
        }
    }

    pub fn store_password(&self) -> AppResult<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &self.id.to_string())
            .map_err(|e| AppError::Keyring(e.to_string()))?;
        entry
            .set_password(self.password.expose_secret())
            .map_err(|e| AppError::Keyring(e.to_string()))
    }

    pub fn load_password(id: Uuid) -> AppResult<SecretString> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &id.to_string())
            .map_err(|e| AppError::Keyring(e.to_string()))?;
        let password = entry
            .get_password()
            .map_err(|e| AppError::Keyring(e.to_string()))?;
        Ok(SecretString::from(password))
    }

    pub fn delete_password(id: Uuid) -> AppResult<()> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, &id.to_string())
            .map_err(|e| AppError::Keyring(e.to_string()))?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Keyring(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use secrecy::SecretString;

    use super::*;

    #[test]
    fn connection_record_defaults_mode_passive() {
        let json = r#"{"id":"00000000-0000-0000-0000-000000000000","name":"s","host":"h","port":21,"username":"u"}"#;
        let record: ConnectionRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.mode, FtpMode::Passive);
        assert_eq!(record.security, FtpSecurity::Plain);
        assert!(!record.accept_invalid_certs);
    }

    #[test]
    fn connection_record_round_trip_active_mode() {
        let record = ConnectionRecord {
            id: Uuid::nil(),
            name: "Test".into(),
            host: "127.0.0.1".into(),
            port: 21,
            username: "user".into(),
            mode: FtpMode::Active,
            security: FtpSecurity::Plain,
            accept_invalid_certs: false,
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: ConnectionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn connection_record_round_trip_explicit_ftps() {
        let record = ConnectionRecord {
            id: Uuid::nil(),
            name: "Secure".into(),
            host: "ftp.example.com".into(),
            port: 21,
            username: "user".into(),
            mode: FtpMode::Passive,
            security: FtpSecurity::Explicit,
            accept_invalid_certs: true,
        };

        let json = serde_json::to_string(&record).unwrap();
        let parsed: ConnectionRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record, parsed);
    }

    #[test]
    fn try_new_preserves_mode() {
        let conn = Connection::try_new(
            None,
            "Site".into(),
            "host".into(),
            21,
            "user".into(),
            SecretString::from("pass"),
            FtpMode::Active,
            FtpSecurity::Explicit,
            true,
        )
        .unwrap();
        assert_eq!(conn.mode, FtpMode::Active);
        assert_eq!(conn.security, FtpSecurity::Explicit);
        assert!(conn.accept_invalid_certs);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}
