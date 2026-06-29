use poise::CreateReply;
use serenity::all::{Channel, CreateEmbed};

use crate::{db::channels::ChannelsRepo, Context, Error};

/// Manage channels where activity does NOT award points.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only,
    subcommands("add", "remove", "list")
)]
pub async fn excluded_channels(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Stop awarding points for activity in this channel.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn add(
    ctx: Context<'_>,
    #[description = "Channel to exclude"] channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.id().get() as i64;
    let reply = match ctx.data().channels.exclude(guild_id, channel_id).await {
        Ok(()) => format!("excluded <#{}> from scoring", channel_id),
        Err(e) => {
            eprintln!("[excluded_channels add] db error: {e}");
            "failed to exclude channel".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// Re-enable scoring for this channel.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Channel"] channel: Channel,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.id().get() as i64;
    let reply = match ctx.data().channels.include(guild_id, channel_id).await {
        Ok(()) => format!("re-enabled scoring for <#{}>", channel_id),
        Err(e) => {
            eprintln!("[excluded_channels remove] db error: {e}");
            "failed to re-enable scoring".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// List excluded channels.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let ids = match ctx.data().channels.list_excluded(guild_id).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[excluded_channels list] db error: {e}");
            ctx.say("failed to list").await?;
            return Ok(());
        }
    };
    let body = if ids.is_empty() {
        "(none)".to_string()
    } else {
        ids.into_iter().map(|id| format!("- <#{}>\n", id)).collect()
    };
    let embed = CreateEmbed::new()
        .title("excluded channels")
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
