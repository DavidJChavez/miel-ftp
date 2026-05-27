use uuid::Uuid;

use crate::models::connection::{FtpMode, FtpSecurity};

#[derive(Debug, Clone, Default)]
pub struct ConnectionForm {
    pub editing_id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub active_mode: bool,
    pub use_ftps: bool,
    pub accept_invalid_certs: bool,
    pub error: Option<String>,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            port: String::from("21"),
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_connection(
        id: Uuid,
        name: &str,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
        mode: FtpMode,
        security: FtpSecurity,
        accept_invalid_certs: bool,
    ) -> Self {
        Self {
            editing_id: Some(id),
            name: name.to_string(),
            host: host.to_string(),
            port: port.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            active_mode: mode == FtpMode::Active,
            use_ftps: security == FtpSecurity::Explicit,
            accept_invalid_certs,
            error: None,
        }
    }

    pub fn ftp_mode(&self) -> FtpMode {
        if self.active_mode {
            FtpMode::Active
        } else {
            FtpMode::Passive
        }
    }

    pub fn ftp_security(&self) -> FtpSecurity {
        if self.use_ftps {
            FtpSecurity::Explicit
        } else {
            FtpSecurity::Plain
        }
    }
}
