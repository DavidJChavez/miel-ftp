use std::time::Instant;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Success,
    Error,
}

pub struct Toast {
    pub id: Uuid,
    pub message: String,
    pub kind: ToastKind,
    pub created_at: Instant,
}

impl Toast {
    pub fn new(message: impl Into<String>, kind: ToastKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            message: message.into(),
            kind,
            created_at: Instant::now(),
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs() >= 4
    }
}
