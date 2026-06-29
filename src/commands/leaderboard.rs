use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{db::users::UsersRepo, Context, Error};

const LEADERBOARD_LIMIT: i64 = 100;

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let users = match ctx
        .data()
        .users
        .get_leaderboard(guild_id, LEADERBOARD_LIMIT)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[leaderboard] db error: {e}");
            return Ok(());
        }
    };
    let body = users
        .into_iter()
        .map(|u| format!("{}: {}\n", u.nick, u.score))
        .collect::<String>();
    let embed = CreateEmbed::new()
        .title("leaderboard")
        .description(if body.is_empty() { "(empty)".to_string() } else { body })
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
