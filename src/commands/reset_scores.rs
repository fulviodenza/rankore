use crate::{db::users::UsersRepo, Context, Error};

#[poise::command(
    prefix_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn reset_scores(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx.data().users.reset_scores(guild_id).await {
        Ok(()) => "Scores reset",
        Err(e) => {
            eprintln!("[reset_scores] db error: {e}");
            "failed to reset scores"
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
