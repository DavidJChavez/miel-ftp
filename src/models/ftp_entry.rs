use std::time::SystemTime;

use chrono::{DateTime, Utc};
use suppaftp::list::File as ListFile;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FtpEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: Option<String>,
}

impl FtpEntry {
    pub fn from_list_line(line: &str) -> Option<Self> {
        let file = ListFile::try_from(line).ok()?;
        let name = file.name().to_string();

        if name == "." || name == ".." {
            return None;
        }

        let is_dir = file.is_directory();
        let size = if is_dir {
            None
        } else {
            Some(file.size() as u64)
        };
        let modified = Some(format_system_time(file.modified()));

        Some(Self {
            name,
            is_dir,
            size,
            modified,
        })
    }
}

fn format_system_time(time: SystemTime) -> String {
    let datetime: DateTime<Utc> = time.into();
    datetime.format("%Y-%m-%d %H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unix_list_line() {
        let line = "-rw-r--r-- 1 user group 1234 Apr 10 12:00 file.txt";
        let entry = FtpEntry::from_list_line(line).expect("should parse");
        assert_eq!(entry.name, "file.txt");
        assert!(!entry.is_dir);
        assert_eq!(entry.size, Some(1234));
        assert!(entry.modified.is_some());
    }

    #[test]
    fn parses_directory_list_line() {
        let line = "drwxr-xr-x 2 user group 4096 Apr 10 12:00 folder";
        let entry = FtpEntry::from_list_line(line).expect("should parse");
        assert_eq!(entry.name, "folder");
        assert!(entry.is_dir);
        assert_eq!(entry.size, None);
    }

    #[test]
    fn ignores_dot_entries() {
        assert!(FtpEntry::from_list_line("drwxr-xr-x 2 u g 4096 Apr 10 12:00 .").is_none());
        assert!(FtpEntry::from_list_line("drwxr-xr-x 2 u g 4096 Apr 10 12:00 ..").is_none());
    }
}
