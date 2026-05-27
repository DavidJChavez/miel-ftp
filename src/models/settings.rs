use serde::{Deserialize, Serialize};

/// Nombres ocultos por defecto cuando `hide_system_files` está activo.
pub const SYSTEM_DEFAULTS: &[&str] = &[
    ".DS_Store",
    "Thumbs.db",
    "desktop.ini",
    ".git",
    "node_modules",
    ".idea",
    ".vscode",
    "__pycache__",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
    #[serde(default = "default_hide_system_files")]
    pub hide_system_files: bool,
    #[serde(default)]
    pub show_dotfiles: bool,
    #[serde(default)]
    pub custom_hidden: Vec<String>,
}

fn default_hide_system_files() -> bool {
    true
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hide_system_files: true,
            show_dotfiles: false,
            custom_hidden: Vec::new(),
        }
    }
}

/// Indica si una entrada debe ocultarse según las preferencias globales.
pub fn is_hidden(name: &str, settings: &AppSettings) -> bool {
    if settings.hide_system_files && SYSTEM_DEFAULTS.contains(&name) {
        return true;
    }

    if !settings.show_dotfiles && name.starts_with('.') {
        return true;
    }

    settings.custom_hidden.iter().any(|h| h == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_hide_system_files() {
        let s = AppSettings::default();
        assert!(s.hide_system_files);
        assert!(!s.show_dotfiles);
        assert!(is_hidden(".DS_Store", &s));
    }

    #[test]
    fn dotfile_hidden_when_show_dotfiles_off() {
        let s = AppSettings {
            hide_system_files: false,
            show_dotfiles: false,
            custom_hidden: vec![],
        };
        assert!(is_hidden(".env", &s));
        assert!(!is_hidden("file.txt", &s));
    }

    #[test]
    fn dotfile_visible_when_show_dotfiles_on() {
        let s = AppSettings {
            hide_system_files: false,
            show_dotfiles: true,
            custom_hidden: vec![],
        };
        assert!(!is_hidden(".env", &s));
    }

    #[test]
    fn system_file_hidden_even_with_dotfiles_shown() {
        let s = AppSettings {
            hide_system_files: true,
            show_dotfiles: true,
            custom_hidden: vec![],
        };
        assert!(is_hidden(".DS_Store", &s));
        assert!(!is_hidden(".env", &s));
    }

    #[test]
    fn custom_hidden_exact_match() {
        let s = AppSettings {
            hide_system_files: false,
            show_dotfiles: true,
            custom_hidden: vec!["backup.tmp".into()],
        };
        assert!(is_hidden("backup.tmp", &s));
        assert!(!is_hidden("other.tmp", &s));
    }
}
