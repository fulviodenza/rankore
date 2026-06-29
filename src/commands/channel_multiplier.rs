use poise::CreateReply;
use serenity::all::{Channel, CreateEmbed};

use crate::{db::channels::ChannelsRepo, Context, Error};

/// Per-channel overrides for text and voice scoring multipliers.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only,
    subcommands("set", "clear", "list")
)]
pub async fn channel_multiplier(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Override text and/or voice multiplier for a channel.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Channel to override"] channel: Channel,
    #[description = "Text multiplier (omit to leave unchanged)"] text: Option<i64>,
    #[description = "Voice multiplier (omit to leave unchanged)"] voice: Option<i64>,
) -> Result<(), Error> {
    if text.is_none() && voice.is_none() {
        ctx.say("provide at least one of `text:` or `voice:`").await?;
        return Ok(());
    }
    if let Some(t) = text {
        if t <= 0 {
            ctx.say("text multiplier must be positive").await?;
            return Ok(());
        }
    }
    if let Some(v) = voice {
        if v <= 0 {
            ctx.say("voice multiplier must be positive").await?;
            return Ok(());
        }
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.id().get() as i64;
    let reply = match ctx
        .data()
        .channels
        .set_multiplier(guild_id, channel_id, text, voice)
        .await
    {
        Ok(()) => format!(
            "override set for <#{}>: text={:?} voice={:?}",
            channel_id, text, voice
        ),
        Err(e) => {
            eprintln!("[channel_multiplier set] db error: {e}");
            "failed to set override".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// Remove the override for a channel (revert to guild default).
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn clear(
    ctx: Context<'_>,
    #[description = "Channel"] channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.id().get() as i64;
    let reply = match ctx.data().channels.clear_multiplier(guild_id, channel_id).await {
        Ok(()) => format!("override cleared for <#{}>", channel_id),
        Err(e) => {
            eprintln!("[channel_multiplier clear] db error: {e}");
            "failed to clear override".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// List channel-level overrides for this guild.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let rows = match ctx.data().channels.list(guild_id).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[channel_multiplier list] db error: {e}");
            ctx.say("failed to list overrides").await?;
            return Ok(());
        }
    };
    let body = if rows.is_empty() {
        "(none configured)".to_string()
    } else {
        rows.into_iter()
            .map(|r| {
                format!(
                    "- <#{}> text={} voice={}\n",
                    r.channel_id,
                    r.text_multiplier.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string()),
                    r.voice_multiplier.map(|v| v.to_string()).unwrap_or_else(|| "—".to_string()),
                )
            })
            .collect()
    };
    let embed = CreateEmbed::new()
        .title("channel multipliers")
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
