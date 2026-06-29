use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{db::guilds::GuildRepo, Context, Error};

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn multipliers(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let text = ctx.data().guilds.get_text_multiplier(guild_id).await;
    let voice = ctx.data().guilds.get_voice_multiplier(guild_id).await;
    let body = format!("text multiplier: {}\nvoice multiplier: {}", text, voice);
    let embed = CreateEmbed::new()
        .title("multipliers")
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
