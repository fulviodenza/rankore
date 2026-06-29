use serenity::all::ChannelId;

use crate::{db::guilds::GuildRepo, Context};

/// Post a one-line audit log entry for the given command invocation, if the
/// guild has configured an audit channel and the command requires
/// ADMINISTRATOR. Silent no-op otherwise.
pub async fn log_command(ctx: Context<'_>) {
    let Some(guild_id) = ctx.guild_id() else {
        return;
    };
    let cmd = ctx.command();
    let requires_admin = cmd
        .required_permissions
        .contains(serenity::all::Permissions::ADMINISTRATOR);
    if !requires_admin {
        return;
    }
    let Some(channel_id) = ctx.data().guilds.get_audit_channel(guild_id.get() as i64).await
    else {
        return;
    };
    let invoked_with = ctx.invocation_string();
    let msg = format!("[audit] {} ran `{}`", ctx.author().tag(), invoked_with);
    let channel = ChannelId::new(channel_id as u64);
    if let Err(e) = channel.say(ctx.http(), msg).await {
        eprintln!("[audit] failed to write audit log: {e}");
    }
}
