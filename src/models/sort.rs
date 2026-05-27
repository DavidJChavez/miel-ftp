use crate::models::ftp_entry::FtpEntry;
use crate::models::settings::{AppSettings, is_hidden};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    #[default]
    Asc,
    Desc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SortSpec {
    pub key: SortKey,
    pub order: SortOrder,
}

impl SortSpec {
    pub fn toggle_key(self, key: SortKey) -> Self {
        if self.key == key {
            Self {
                key,
                order: match self.order {
                    SortOrder::Asc => SortOrder::Desc,
                    SortOrder::Desc => SortOrder::Asc,
                },
            }
        } else {
            Self {
                key,
                order: SortOrder::Asc,
            }
        }
    }
}

pub fn apply_view<'a>(
    entries: &'a [FtpEntry],
    sort: SortSpec,
    filter: &str,
    settings: &AppSettings,
) -> Vec<&'a FtpEntry> {
    let filter_lower = filter.trim().to_lowercase();
    let mut visible: Vec<&FtpEntry> = entries
        .iter()
        .filter(|e| !is_hidden(&e.name, settings))
        .filter(|e| filter_lower.is_empty() || e.name.to_lowercase().contains(&filter_lower))
        .collect();

    visible.sort_by(|a, b| {
        let dir_cmp = b.is_dir.cmp(&a.is_dir);
        if dir_cmp != std::cmp::Ordering::Equal {
            return dir_cmp;
        }

        let cmp = match sort.key {
            SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            SortKey::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
            SortKey::Modified => a
                .modified
                .as_deref()
                .unwrap_or("")
                .cmp(b.modified.as_deref().unwrap_or("")),
        };

        match sort.order {
            SortOrder::Asc => cmp,
            SortOrder::Desc => cmp.reverse(),
        }
    });

    visible
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool, size: Option<u64>) -> FtpEntry {
        FtpEntry {
            name: name.into(),
            is_dir,
            size,
            modified: None,
        }
    }

    #[test]
    fn dirs_always_first() {
        let entries = vec![
            entry("file.txt", false, Some(10)),
            entry("folder", true, None),
        ];
        let result = apply_view(&entries, SortSpec::default(), "", &AppSettings::default());
        assert!(result[0].is_dir);
        assert!(!result[1].is_dir);
    }

    #[test]
    fn filter_case_insensitive() {
        let entries = vec![
            entry("Alpha.txt", false, Some(1)),
            entry("beta.txt", false, Some(2)),
        ];
        let result = apply_view(
            &entries,
            SortSpec::default(),
            "ALPHA",
            &AppSettings::default(),
        );
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "Alpha.txt");
    }

    #[test]
    fn sort_by_size_desc() {
        let entries = vec![
            entry("a.txt", false, Some(100)),
            entry("b.txt", false, Some(500)),
            entry("c.txt", false, Some(200)),
        ];
        let result = apply_view(
            &entries,
            SortSpec {
                key: SortKey::Size,
                order: SortOrder::Desc,
            },
            "",
            &AppSettings::default(),
        );
        assert_eq!(result[0].name, "b.txt");
        assert_eq!(result[1].name, "c.txt");
        assert_eq!(result[2].name, "a.txt");
    }

    #[test]
    fn hides_system_files_when_enabled() {
        let entries = vec![
            entry(".DS_Store", false, Some(1)),
            entry("visible.txt", false, Some(2)),
        ];
        let settings = AppSettings::default();
        let result = apply_view(&entries, SortSpec::default(), "", &settings);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "visible.txt");
    }
}
