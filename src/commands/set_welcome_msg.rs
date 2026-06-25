use crate::{db::guilds::GuildRepo, Context, Error};

#[poise::command(
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_welcome_msg(
    ctx: Context<'_>,
    #[description = "Welcome message template (use the user mention will be appended)"]
    #[rest]
    welcome_msg: String,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .guilds
        .set_welcome_msg(guild_id, welcome_msg.trim())
        .await
    {
        Ok(()) => "welcome message set",
        Err(e) => {
            eprintln!("[set_welcome_msg] db error: {e}");
            "failed to set welcome message"
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
