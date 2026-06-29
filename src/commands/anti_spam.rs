use crate::{db::guilds::GuildRepo, Context, Error};

/// Ignore messages shorter than this many characters when scoring.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_min_msg_length(
    ctx: Context<'_>,
    #[description = "Minimum characters (0 disables)"] chars: i64,
) -> Result<(), Error> {
    if chars < 0 {
        ctx.say("min length cannot be negative").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .guilds
        .set_min_msg_length(guild_id, chars as i32)
        .await
    {
        Ok(()) if chars == 0 => "min message length disabled".to_string(),
        Ok(()) => format!("messages shorter than {} chars won't score", chars),
        Err(e) => {
            eprintln!("[set_min_msg_length] db error: {e}");
            "failed to set min length".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// Require N seconds between scoring messages from the same user.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_msg_cooldown(
    ctx: Context<'_>,
    #[description = "Cooldown in seconds (0 disables)"] secs: i64,
) -> Result<(), Error> {
    if secs < 0 {
        ctx.say("cooldown cannot be negative").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .guilds
        .set_msg_cooldown(guild_id, secs as i32)
        .await
    {
        Ok(()) if secs == 0 => "message cooldown disabled".to_string(),
        Ok(()) => format!("cooldown set to {}s between scoring messages", secs),
        Err(e) => {
            eprintln!("[set_msg_cooldown] db error: {e}");
            "failed to set cooldown".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
