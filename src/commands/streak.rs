use serenity::all::User;

use crate::{db::users::UsersRepo, Context, Error};

/// Show a user's current daily-activity streak.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn streak(
    ctx: Context<'_>,
    #[description = "User to check (defaults to you)"] user: Option<User>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let target = user.unwrap_or_else(|| ctx.author().clone());
    let target_id = target.id.get() as i64;
    let streak = match ctx.data().users.get_streak(target_id, guild_id).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[streak] db error: {e}");
            ctx.say("failed to read streak").await?;
            return Ok(());
        }
    };
    let body = match streak {
        0 => format!("{} has no active streak.", target.name),
        1 => format!("{} is on a 1-day streak.", target.name),
        n => format!("{} is on a {}-day streak.", target.name, n),
    };
    ctx.say(body).await?;
    Ok(())
}
