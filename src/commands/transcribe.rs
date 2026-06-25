use std::{collections::HashSet, sync::Arc};

use poise::CreateReply;
use serenity::all::{CreateAttachment, GuildChannel};
use tokio::sync::Mutex;

use crate::{
    voice::{
        receiver::{voice_events, VoiceReceiver},
        session::TranscriptSession,
        transcript::TranscriptWriter,
    },
    Context, Error,
};

const ALLOWED_LANGS: &[&str] = &["en", "it", "es", "fr"];

/// Join the caller's voice channel and start transcribing.
///
/// Usage:
///   !transcribe_join          (auto-detect language)
///   !transcribe_join lang=it  (pin to one of: en | it | es | fr)
#[poise::command(prefix_command, guild_only)]
pub async fn transcribe_join(
    ctx: Context<'_>,
    #[description = "Optional language pin (lang=en|it|es|fr)"]
    #[rest]
    args: Option<String>,
) -> Result<(), Error> {
    let language = parse_language(args.as_deref())?;
    let guild_id = ctx.guild_id().unwrap();

    let channel_id = match caller_voice_channel(&ctx).await? {
        Some(id) => id,
        None => {
            ctx.say("Join a voice channel first, then run this command again.")
                .await?;
            return Ok(());
        }
    };

    {
        let sessions = ctx.data().sessions.lock().await;
        if sessions.contains_key(&guild_id) {
            ctx.say("A transcription session is already active in this server. Run `transcribe_leave` first.")
                .await?;
            return Ok(());
        }
    }

    let serenity_ctx = ctx.serenity_context().clone();
    let manager = songbird::get(&serenity_ctx)
        .await
        .ok_or("Songbird voice manager not initialized")?
        .clone();

    tracing::info!(guild_id = %guild_id.get(), channel_id = %channel_id.get(), "transcribe_join: requesting songbird join");
    let handler_lock = match manager.join(guild_id, channel_id).await {
        Ok(h) => {
            tracing::info!(guild_id = %guild_id.get(), "transcribe_join: songbird join OK");
            h
        }
        Err(e) => {
            tracing::error!(error = ?e, guild_id = %guild_id.get(), channel_id = %channel_id.get(), "transcribe_join: songbird join failed");
            ctx.say(format!("Failed to join voice channel: {e}")).await?;
            return Ok(());
        }
    };

    // Set up the per-session transcript file.
    let started_at = chrono::Utc::now();
    let file_name = format!(
        "transcript-{}-{}-{}.txt",
        guild_id.get(),
        channel_id.get(),
        started_at.format("%Y%m%dT%H%M%S")
    );
    let file_path = ctx.data().transcripts_dir.join(file_name);
    let writer = match TranscriptWriter::create(&file_path) {
        Ok(w) => Arc::new(Mutex::new(w)),
        Err(e) => {
            ctx.say(format!("Failed to open transcript file: {e}")).await?;
            let _ = manager.remove(guild_id).await;
            return Ok(());
        }
    };

    let session = TranscriptSession {
        guild_id,
        channel_id,
        language: language.clone(),
        started_at,
        writer,
        opted_out: Arc::new(Mutex::new(HashSet::new())),
        file_path: file_path.clone(),
    };
    ctx.data().sessions.lock().await.insert(guild_id, session);

    // Wire songbird events.
    let data_arc = Arc::new(ctx.data().clone());
    let receiver = VoiceReceiver::new(guild_id, data_arc, serenity_ctx.clone());
    let receiver_arc = Arc::new(receiver);
    {
        let mut handler = handler_lock.lock().await;
        for ev in voice_events() {
            handler.add_global_event(ev, ReceiverEvent(receiver_arc.clone()));
        }
    }

    let lang_label = language.as_deref().unwrap_or("auto-detect");
    ctx.say(format!(
        "Recording started in <#{}>. Language: **{}**. Transcripts will appear in `{}`. \
         Run `transcribe_leave` to stop.",
        channel_id.get(),
        lang_label,
        file_path.display()
    ))
    .await?;
    Ok(())
}

/// Stop transcribing and send the transcript file.
#[poise::command(prefix_command, guild_only)]
pub async fn transcribe_leave(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();

    let session = ctx.data().sessions.lock().await.remove(&guild_id);
    let Some(session) = session else {
        ctx.say("No transcription session active in this server.").await?;
        return Ok(());
    };

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or("Songbird voice manager not initialized")?
        .clone();
    let _ = manager.remove(guild_id).await;

    let attachment = match CreateAttachment::path(&session.file_path).await {
        Ok(a) => Some(a),
        Err(e) => {
            eprintln!("[transcribe_leave] attachment: {e}");
            None
        }
    };
    let summary = format!(
        "Recording stopped. Session started {}, duration {}.",
        session.pretty_started(),
        format_duration(chrono::Utc::now() - session.started_at),
    );
    let mut reply = CreateReply::default().content(summary).reply(true);
    if let Some(a) = attachment {
        reply = reply.attachment(a);
    }
    ctx.send(reply).await?;
    Ok(())
}

/// Show current transcription session info.
#[poise::command(prefix_command, guild_only)]
pub async fn transcribe_status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap();
    let sessions = ctx.data().sessions.lock().await;
    let Some(session) = sessions.get(&guild_id) else {
        ctx.say("No transcription session active in this server.").await?;
        return Ok(());
    };
    let opted_out_count = session.opted_out.lock().await.len();
    let lang = session.language.as_deref().unwrap_or("auto-detect");
    ctx.say(format!(
        "Active session in <#{}>. Started {}. Language: **{}**. Opted-out users: {}.",
        session.channel_id.get(),
        session.pretty_started(),
        lang,
        opted_out_count,
    ))
    .await?;
    Ok(())
}

async fn caller_voice_channel(
    ctx: &Context<'_>,
) -> Result<Option<serenity::all::ChannelId>, Error> {
    let guild_id = ctx.guild_id().unwrap();
    let author_id = ctx.author().id;
    let guild = guild_id.to_partial_guild(&ctx.serenity_context().http).await?;
    let channels = guild.channels(&ctx.serenity_context().http).await?;
    for (id, ch) in channels {
        if !matches!(ch.kind, serenity::all::ChannelType::Voice) {
            continue;
        }
        if user_in_voice_channel(ctx, &ch, author_id).await {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

async fn user_in_voice_channel(
    ctx: &Context<'_>,
    channel: &GuildChannel,
    user_id: serenity::all::UserId,
) -> bool {
    match channel.members(&ctx.serenity_context().cache) {
        Ok(members) => members.iter().any(|m| m.user.id == user_id),
        Err(_) => false,
    }
}

fn parse_language(args: Option<&str>) -> Result<Option<String>, Error> {
    let Some(raw) = args else {
        return Ok(None);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(None);
    }
    let lang = raw
        .strip_prefix("lang=")
        .or_else(|| raw.strip_prefix("language="))
        .unwrap_or(raw)
        .trim()
        .to_lowercase();
    if !ALLOWED_LANGS.contains(&lang.as_str()) {
        return Err(format!(
            "language must be one of {ALLOWED_LANGS:?} (got {lang:?})"
        )
        .into());
    }
    Ok(Some(lang))
}

fn format_duration(d: chrono::Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h{m:02}m{s:02}s")
    } else {
        format!("{m}m{s:02}s")
    }
}

// Adapter so we can store a Send + Sync handler with cheap clone.
struct ReceiverEvent(Arc<VoiceReceiver>);

#[async_trait::async_trait]
impl songbird::EventHandler for ReceiverEvent {
    async fn act(&self, ctx: &songbird::EventContext<'_>) -> Option<songbird::Event> {
        songbird::EventHandler::act(&*self.0, ctx).await
    }
}
