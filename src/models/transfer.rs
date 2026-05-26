use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferKind {
    Upload,
    Download,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransferStatus {
    Queued,
    Active,
    Done,
    Failed(String),
    Cancelled,
}

impl TransferStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TransferStatus::Done | TransferStatus::Failed(_) | TransferStatus::Cancelled
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(self, TransferStatus::Active)
    }
}

#[derive(Debug, Clone)]
pub struct TransferEntry {
    pub id: Uuid,
    pub connection_id: Uuid,
    pub kind: TransferKind,
    pub filename: String,
    pub local_path: PathBuf,
    pub remote_path: String,
    pub total_bytes: Option<u64>,
    pub transferred_bytes: u64,
    pub resume_from: u64,
    pub status: TransferStatus,
    pub cancel: Arc<AtomicBool>,
}

impl TransferEntry {
    pub fn percent(&self) -> Option<f32> {
        let total = self.total_bytes?;
        if total == 0 {
            return Some(100.0);
        }
        Some((self.transferred_bytes as f32 / total as f32) * 100.0)
    }

    pub fn status_label(&self) -> &'static str {
        match self.status {
            TransferStatus::Queued => "en cola",
            TransferStatus::Active => "activa",
            TransferStatus::Done => "completada",
            TransferStatus::Failed(_) => "fallida",
            TransferStatus::Cancelled => "cancelada",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    fn sample_entry(total: Option<u64>, transferred: u64) -> TransferEntry {
        TransferEntry {
            id: Uuid::new_v4(),
            connection_id: Uuid::new_v4(),
            kind: TransferKind::Upload,
            filename: "test.bin".into(),
            local_path: PathBuf::from("/tmp/test.bin"),
            remote_path: "/".into(),
            total_bytes: total,
            transferred_bytes: transferred,
            resume_from: 0,
            status: TransferStatus::Active,
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    #[test]
    fn percent_none_when_total_unknown() {
        let e = sample_entry(None, 100);
        assert!(e.percent().is_none());
    }

    #[test]
    fn percent_full_when_zero_total() {
        let e = sample_entry(Some(0), 0);
        assert_eq!(e.percent(), Some(100.0));
    }

    #[test]
    fn percent_half() {
        let e = sample_entry(Some(200), 100);
        assert_eq!(e.percent(), Some(50.0));
    }

    #[test]
    fn terminal_statuses() {
        assert!(!TransferStatus::Queued.is_terminal());
        assert!(!TransferStatus::Active.is_terminal());
        assert!(TransferStatus::Done.is_terminal());
        assert!(TransferStatus::Failed("x".into()).is_terminal());
        assert!(TransferStatus::Cancelled.is_terminal());
    }

    #[test]
    fn active_status() {
        assert!(TransferStatus::Active.is_active());
        assert!(!TransferStatus::Queued.is_active());
    }

    #[test]
    fn cancel_flag_defaults_false() {
        let e = sample_entry(Some(10), 0);
        assert!(!e.cancel.load(Ordering::Relaxed));
    }
}
