use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::models::settings::AppSettings;

fn config_dir() -> AppResult<PathBuf> {
    let dir = dirs::config_dir()
        .ok_or_else(|| AppError::Config("no se encontró el directorio de configuración".into()))?
        .join("miel-ftp");
    Ok(dir)
}

fn settings_path() -> AppResult<PathBuf> {
    Ok(config_dir()?.join("settings.json"))
}

pub fn load_settings() -> AppResult<AppSettings> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let data = std::fs::read_to_string(&path)?;
    serde_json::from_str(&data).map_err(AppError::from)
}

pub fn save_settings(settings: &AppSettings) -> AppResult<()> {
    let dir = config_dir()?;
    std::fs::create_dir_all(&dir)?;

    let data = serde_json::to_string_pretty(settings)?;
    let path = settings_path()?;
    let tmp = dir.join("settings.json.tmp");
    std::fs::write(&tmp, data)?;
    std::fs::rename(tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_json() {
        let settings = AppSettings {
            hide_system_files: false,
            show_dotfiles: true,
            custom_hidden: vec!["foo.bar".into()],
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: AppSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(settings, parsed);
    }

    #[test]
    fn settings_default_deserializes_missing_fields() {
        let json = r#"{"show_dotfiles": true}"#;
        let parsed: AppSettings = serde_json::from_str(json).unwrap();
        assert!(parsed.hide_system_files);
        assert!(parsed.show_dotfiles);
        assert!(parsed.custom_hidden.is_empty());
    }
}
