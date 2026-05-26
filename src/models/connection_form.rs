use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct ConnectionForm {
    pub editing_id: Option<Uuid>,
    pub name: String,
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub error: Option<String>,
}

impl ConnectionForm {
    pub fn new() -> Self {
        Self {
            port: String::from("21"),
            ..Default::default()
        }
    }

    pub fn from_connection(
        id: Uuid,
        name: &str,
        host: &str,
        port: u16,
        username: &str,
        password: &str,
    ) -> Self {
        Self {
            editing_id: Some(id),
            name: name.to_string(),
            host: host.to_string(),
            port: port.to_string(),
            username: username.to_string(),
            password: password.to_string(),
            error: None,
        }
    }
}
