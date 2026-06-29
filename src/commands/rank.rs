use poise::CreateReply;

use crate::{db::users::UsersRepo, Context, Error};

/// Quick self-check: your current rank and score, replied ephemerally.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn rank(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let user_id = ctx.author().id.get() as i64;
    let body = match ctx.data().users.get_user_stats(user_id, guild_id).await {
        Ok(Some(s)) => format!("you are rank #{} with {} points", s.rank, s.score),
        Ok(None) => "you have no activity recorded yet.".to_string(),
        Err(e) => {
            eprintln!("[rank] db error: {e}");
            "failed to read rank".to_string()
        }
    };
    ctx.send(CreateReply::default().content(body).ephemeral(true)).await?;
    Ok(())
}
