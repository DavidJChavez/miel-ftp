#[derive(Debug, Clone)]
pub struct FtpEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,        // None si es directorio
    pub modified: Option<String>, // Simplificado por ahora
}
