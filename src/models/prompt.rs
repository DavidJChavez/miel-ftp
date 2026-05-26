use crate::models::panel::PanelKind;

#[derive(Debug, Clone)]
pub enum PromptDialog {
    Mkdir {
        panel: PanelKind,
        value: String,
    },
    Rename {
        panel: PanelKind,
        old: String,
        new: String,
    },
    ConfirmDelete {
        panel: PanelKind,
        names: Vec<String>,
    },
}

impl PromptDialog {
    pub fn mkdir(panel: PanelKind) -> Self {
        Self::Mkdir {
            panel,
            value: String::new(),
        }
    }

    pub fn rename(panel: PanelKind, old: String) -> Self {
        Self::Rename {
            panel,
            old: old.clone(),
            new: old,
        }
    }

    pub fn confirm_delete(panel: PanelKind, names: Vec<String>) -> Self {
        Self::ConfirmDelete { panel, names }
    }
}

pub fn validate_entry_name(name: &str) -> Result<String, String> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err("el nombre no puede estar vacío".into());
    }
    if trimmed == "." || trimmed == ".." {
        return Err("nombre inválido".into());
    }
    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err("el nombre no puede contener / o \\".into());
    }
    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_name() {
        assert!(validate_entry_name("  ").is_err());
    }

    #[test]
    fn rejects_dot_entries() {
        assert!(validate_entry_name(".").is_err());
        assert!(validate_entry_name("..").is_err());
    }

    #[test]
    fn rejects_slashes() {
        assert!(validate_entry_name("foo/bar").is_err());
        assert!(validate_entry_name("foo\\bar").is_err());
    }

    #[test]
    fn accepts_valid_name() {
        assert_eq!(
            validate_entry_name("  my file.txt  ").unwrap(),
            "my file.txt"
        );
    }
}
