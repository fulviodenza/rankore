use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
};

/// Appends transcript lines to a per-session text file.
pub struct TranscriptWriter {
    file: File,
}

impl TranscriptWriter {
    pub fn create(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file })
    }

    pub fn append(
        &mut self,
        timestamp: chrono::DateTime<chrono::Utc>,
        nick: &str,
        text: &str,
    ) -> std::io::Result<()> {
        let ts = timestamp.format("%H:%M:%S");
        writeln!(self.file, "[{ts}] @{nick}: {text}")?;
        self.file.flush()
    }
}
