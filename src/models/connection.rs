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

/// Protocolo de conexión.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    #[default]
    Ftp,
    FtpsExplicit,
    Sftp,
}

impl Protocol {
    pub fn label(self) -> &'static str {
        match self {
            Protocol::Ftp => "FTP",
            Protocol::FtpsExplicit => "FTPS",
            Protocol::Sftp => "SFTP",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            Protocol::Ftp | Protocol::FtpsExplicit => 21,
            Protocol::Sftp => 22,
        }
    }

    pub fn uses_ftp_mode(self) -> bool {
        !matches!(self, Protocol::Sftp)
    }

    pub fn uses_tls_options(self) -> bool {
        matches!(self, Protocol::FtpsExplicit)
    }
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Seguridad de transporte FTP (compatibilidad JSON legacy).
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
    pub protocol: Protocol,
    #[serde(default)]
    pub accept_invalid_certs: bool,
    #[serde(default)]
    pub bookmarks: Vec<String>,
}

impl ConnectionRecord {
    pub fn effective_protocol(&self) -> Protocol {
        match (self.protocol, self.security) {
            (Protocol::Ftp, FtpSecurity::Explicit) => Protocol::FtpsExplicit,
            (p, _) => p,
        }
    }
}

#[derive(Clone)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub mode: FtpMode,
    pub protocol: Protocol,
    pub security: FtpSecurity,
    pub accept_invalid_certs: bool,
    pub bookmarks: Vec<String>,
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
            .field("protocol", &self.protocol)
            .field("security", &self.security)
            .field("accept_invalid_certs", &self.accept_invalid_certs)
            .field("bookmarks", &self.bookmarks)
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
        protocol: Protocol,
        security: FtpSecurity,
        accept_invalid_certs: bool,
        bookmarks: Vec<String>,
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
            protocol,
            security,
            accept_invalid_certs,
            bookmarks,
            password: password.into(),
        })
    }

    pub fn effective_protocol(&self) -> Protocol {
        match (self.protocol, self.security) {
            (Protocol::Ftp, FtpSecurity::Explicit) => Protocol::FtpsExplicit,
            (p, _) => p,
        }
    }

    pub fn add_bookmark(&mut self, path: String) {
        if !self.bookmarks.contains(&path) {
            self.bookmarks.push(path);
        }
    }

    pub fn remove_bookmark(&mut self, path: &str) {
        self.bookmarks.retain(|b| b != path);
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
            protocol: self.protocol,
            accept_invalid_certs: self.accept_invalid_certs,
            bookmarks: self.bookmarks.clone(),
        }
    }

    pub fn from_record(record: ConnectionRecord, password: SecretString) -> Self {
        let protocol = record.effective_protocol();
        Self {
            id: record.id,
            name: record.name,
            host: record.host,
            port: record.port,
            username: record.username,
            mode: record.mode,
            protocol,
            security: record.security,
            accept_invalid_certs: record.accept_invalid_certs,
            bookmarks: record.bookmarks,
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
    fn connection_record_effective_protocol_from_legacy_security() {
        let record = ConnectionRecord {
            id: Uuid::nil(),
            name: "s".into(),
            host: "h".into(),
            port: 21,
            username: "u".into(),
            mode: FtpMode::Passive,
            security: FtpSecurity::Explicit,
            protocol: Protocol::Ftp,
            accept_invalid_certs: false,
            bookmarks: vec![],
        };
        assert_eq!(record.effective_protocol(), Protocol::FtpsExplicit);
    }

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
            protocol: Protocol::Ftp,
            accept_invalid_certs: false,
            bookmarks: vec![],
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
            protocol: Protocol::FtpsExplicit,
            accept_invalid_certs: true,
            bookmarks: vec![],
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
            Protocol::FtpsExplicit,
            FtpSecurity::Explicit,
            true,
            vec![],
        )
        .unwrap();
        assert_eq!(conn.mode, FtpMode::Active);
        assert_eq!(conn.protocol, Protocol::FtpsExplicit);
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
