use std::{
    collections::HashSet,
    path::PathBuf,
    sync::Arc,
};

use serenity::all::{ChannelId, GuildId, UserId};
use tokio::sync::Mutex;

use super::transcript::TranscriptWriter;

/// One active transcription session per guild.
pub struct TranscriptSession {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub language: Option<String>, // None = auto-detect
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub writer: Arc<Mutex<TranscriptWriter>>,
    pub opted_out: Arc<Mutex<HashSet<UserId>>>,
    pub file_path: PathBuf,
}

impl TranscriptSession {
    pub fn pretty_started(&self) -> String {
        self.started_at.format("%Y-%m-%d %H:%M:%S UTC").to_string()
    }
}
