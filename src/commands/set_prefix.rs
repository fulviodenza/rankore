use crate::{db::guilds::GuildRepo, Context, Error};

#[poise::command(
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_prefix(
    ctx: Context<'_>,
    #[description = "New prefix"]
    #[rest]
    new_prefix: String,
) -> Result<(), Error> {
    let new_prefix = new_prefix.trim().to_string();
    if new_prefix.is_empty() {
        ctx.say("prefix cannot be empty").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx.data().guilds.set_prefix(guild_id, &new_prefix).await {
        Ok(()) => "prefix updated!",
        Err(e) => {
            eprintln!("[set_prefix] db error: {e}");
            "failed to update prefix"
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
