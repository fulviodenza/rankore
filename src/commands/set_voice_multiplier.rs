use crate::{db::guilds::GuildRepo, Context, Error};

#[poise::command(
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_voice_multiplier(
    ctx: Context<'_>,
    #[description = "Voice multiplier (positive integer)"] multiplier: i64,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .guilds
        .set_voice_multiplier(guild_id, multiplier)
        .await
    {
        Ok(()) => "Voice multiplier set".to_string(),
        Err(e) => {
            eprintln!("[set_voice_multiplier] db error: {e}");
            "failed to set voice multiplier".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
