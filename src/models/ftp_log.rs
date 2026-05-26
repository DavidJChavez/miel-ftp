const MAX_LINES: usize = 500;

#[derive(Debug, Clone, Default)]
pub struct FtpLog {
    pub visible: bool,
    lines: Vec<String>,
}

impl FtpLog {
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn toggle_visible(&mut self) {
        self.visible = !self.visible;
    }

    pub fn push(&mut self, line: impl Into<String>) {
        self.lines.push(line.into());
        if self.lines.len() > MAX_LINES {
            let excess = self.lines.len() - MAX_LINES;
            self.lines.drain(0..excess);
        }
    }

    pub fn clear(&mut self) {
        self.lines.clear();
    }
}
