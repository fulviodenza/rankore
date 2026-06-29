use crate::{db::guilds::GuildRepo, Context, Error};

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn get_prefix(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let prefix = ctx.data().guilds.get_prefix(guild_id).await;
    ctx.say(format!("Current prefix: {}", prefix)).await?;
    Ok(())
}
