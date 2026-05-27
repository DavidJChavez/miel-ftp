#[derive(Debug, Clone, Default)]
pub struct QuickconnectForm {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
    pub error: Option<String>,
}

impl QuickconnectForm {
    pub fn new() -> Self {
        Self {
            port: String::from("21"),
            ..Default::default()
        }
    }
}
