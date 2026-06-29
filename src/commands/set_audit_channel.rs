use serenity::all::Channel;

use crate::{db::guilds::GuildRepo, Context, Error};

/// Send a log line to this channel whenever an admin command runs. Omit the
/// channel to disable.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_audit_channel(
    ctx: Context<'_>,
    #[description = "Channel to log to (omit to disable)"] channel: Option<Channel>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let channel_id = channel.as_ref().map(|c| c.id().get() as i64);
    let reply = match ctx.data().guilds.set_audit_channel(guild_id, channel_id).await {
        Ok(()) => match channel_id {
            Some(id) => format!("audit log channel set to <#{}>", id),
            None => "audit logging disabled".to_string(),
        },
        Err(e) => {
            eprintln!("[set_audit_channel] db error: {e}");
            "failed to set audit channel".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
