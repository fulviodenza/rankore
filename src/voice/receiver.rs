//! Songbird voice event handler.
//!
//! Hooks `VoiceTick` (per-speaker decoded PCM) and `SpeakingStateUpdate`
//! (SSRC ↔ user_id mapping). For each speaker we maintain a [`Segmenter`];
//! completed utterances are resampled to 16 kHz mono and shipped to whisper
//! on a background task, then appended to the session's transcript file.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use serenity::all::{Context as SerenityContext, GuildId, UserId};
use songbird::{
    events::context_data::VoiceTick,
    model::payload::Speaking,
    CoreEvent, Event, EventContext, EventHandler as VoiceEventHandler,
};
use tokio::sync::Mutex;

use super::{resampler, segmenter::Segmenter, stt::WhisperClient};
use crate::Data;

/// Shared per-session state owned by the songbird driver.
pub struct VoiceReceiver {
    guild_id: GuildId,
    data: Arc<Data>,
    serenity_ctx: SerenityContext,
    /// SSRC -> Discord UserId (populated from SpeakingStateUpdate).
    ssrc_map: Arc<Mutex<HashMap<u32, UserId>>>,
    /// SSRC -> per-user segmenter.
    segmenters: Arc<Mutex<HashMap<u32, Segmenter>>>,
}

impl VoiceReceiver {
    pub fn new(guild_id: GuildId, data: Arc<Data>, serenity_ctx: SerenityContext) -> Self {
        Self {
            guild_id,
            data,
            serenity_ctx,
            ssrc_map: Arc::new(Mutex::new(HashMap::new())),
            segmenters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn on_tick(&self, tick: &VoiceTick) {
        for (&ssrc, voice_data) in &tick.speaking {
            let Some(decoded) = voice_data.decoded_voice.as_ref() else {
                continue;
            };
            self.handle_audio(ssrc, decoded).await;
        }
    }

    async fn handle_audio(&self, ssrc: u32, pcm48k_stereo: &[i16]) {
        let user_id = self.ssrc_map.lock().await.get(&ssrc).copied();

        // Drop audio for opted-out users before it ever hits a buffer or disk.
        if let Some(uid) = user_id {
            let sessions = self.data.sessions.lock().await;
            if let Some(session) = sessions.get(&self.guild_id) {
                if session.opted_out.lock().await.contains(&uid) {
                    return;
                }
            }
        }

        let mut segs = self.segmenters.lock().await;
        let seg = segs.entry(ssrc).or_default();
        let pcm16k_mono = resampler::stereo48k_to_mono16k(pcm48k_stereo);
        let utterances = seg.push(&pcm16k_mono);
        drop(segs);

        for utterance in utterances {
            self.dispatch_utterance(user_id, utterance);
        }
    }

    fn dispatch_utterance(&self, user_id: Option<UserId>, utterance: Vec<i16>) {
        let data = self.data.clone();
        let serenity_ctx = self.serenity_ctx.clone();
        let guild_id = self.guild_id;
        tokio::spawn(async move {
            let (file_writer, language) = {
                let sessions = data.sessions.lock().await;
                let Some(session) = sessions.get(&guild_id) else {
                    return;
                };
                (session.writer.clone(), session.language.clone())
            };

            let client = WhisperClient::new(data.http.clone(), data.whisper_url.clone());
            let text = match client.transcribe(&utterance, language.as_deref()).await {
                Ok(t) if !t.is_empty() => t,
                Ok(_) => return,
                Err(e) => {
                    eprintln!("[voice/stt] transcribe failed: {e}");
                    return;
                }
            };

            let nick = resolve_nick(&serenity_ctx, guild_id, user_id).await;
            let now = chrono::Utc::now();
            let mut w = file_writer.lock().await;
            if let Err(e) = w.append(now, &nick, &text) {
                eprintln!("[voice/transcript] write failed: {e}");
            }
        });
    }

    async fn on_speaking_state(&self, speaking: &Speaking) {
        if let Some(uid) = speaking.user_id {
            self.ssrc_map
                .lock()
                .await
                .insert(speaking.ssrc, UserId::new(uid.0));
        }
    }
}

async fn resolve_nick(
    ctx: &SerenityContext,
    guild_id: GuildId,
    user_id: Option<UserId>,
) -> String {
    let Some(uid) = user_id else {
        return "unknown".to_string();
    };
    match guild_id.member(&ctx.http, uid).await {
        Ok(m) => m
            .nick
            .clone()
            .unwrap_or_else(|| m.display_name().to_string()),
        Err(_) => uid.get().to_string(),
    }
}

#[async_trait]
impl VoiceEventHandler for VoiceReceiver {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        match ctx {
            EventContext::VoiceTick(tick) => self.on_tick(tick).await,
            EventContext::SpeakingStateUpdate(speaking) => {
                self.on_speaking_state(speaking).await
            }
            _ => {}
        }
        None
    }
}

/// Helper to register both events the receiver cares about.
pub fn voice_events() -> [Event; 2] {
    [
        Event::Core(CoreEvent::VoiceTick),
        Event::Core(CoreEvent::SpeakingStateUpdate),
    ]
}

