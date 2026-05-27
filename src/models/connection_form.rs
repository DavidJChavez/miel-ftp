use uuid::Uuid;

use crate::models::connection::{FtpMode, FtpSecurity, Protocol};

#[derive(Debug, Clone, Default)]
pub struct ConnectionForm {
    pub editing_id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub active_mode: bool,
    pub protocol: Protocol,
    pub accept_invalid_certs: bool,
    pub error: Option<String>,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            port: String::from("21"),
            protocol: Protocol::Ftp,
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
        protocol: Protocol,
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
            protocol,
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
        match self.protocol {
            Protocol::FtpsExplicit => FtpSecurity::Explicit,
            _ => FtpSecurity::Plain,
        }
    }

    pub fn set_protocol(&mut self, protocol: Protocol) {
        self.protocol = protocol;
        if protocol == Protocol::Sftp && self.port == "21" {
            self.port = Protocol::Sftp.default_port().to_string();
        } else if protocol != Protocol::Sftp && self.port == "22" {
            self.port = Protocol::Ftp.default_port().to_string();
        }
        if !protocol.uses_tls_options() {
            self.accept_invalid_certs = false;
        }
    }
}
