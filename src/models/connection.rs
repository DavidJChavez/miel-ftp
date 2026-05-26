use std::fmt;

use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "miel-ftp";

/// Registro persistido en JSON (sin contraseña; va en keyring).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConnectionRecord {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
}

#[derive(Clone)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
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
            .field("password", &"[REDACTED]")
            .finish()
    }
}

impl Connection {
    pub fn try_new(
        id: Option<Uuid>,
        name: String,
        host: String,
        port: u16,
        username: String,
        password: impl Into<SecretString>,
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
        }
    }

    pub fn from_record(record: ConnectionRecord, password: SecretString) -> Self {
        Self {
            id: record.id,
            name: record.name,
            host: record.host,
            port: record.port,
            username: record.username,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}
