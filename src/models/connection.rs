use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Connection {
    pub id: Uuid,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String, // TODO: encryptar con keyring en v2
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}
