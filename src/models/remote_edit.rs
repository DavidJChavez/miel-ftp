use std::path::PathBuf;

use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct RemoteEdit {
    pub temp_path: PathBuf,
    pub remote_dir: String,
    pub filename: String,
    pub connection_id: Uuid,
}
