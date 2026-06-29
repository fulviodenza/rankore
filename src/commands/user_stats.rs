use poise::CreateReply;
use serenity::all::{CreateEmbed, User};

use crate::{db::users::UsersRepo, Context, Error};

/// Show a user's score, rank, streak, and most recent activity.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn user_stats(
    ctx: Context<'_>,
    #[description = "User to inspect (defaults to you)"] user: Option<User>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let target = user.unwrap_or_else(|| ctx.author().clone());
    let target_id = target.id.get() as i64;

    let stats = match ctx.data().users.get_user_stats(target_id, guild_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            ctx.say(format!("{} has no activity recorded yet.", target.name))
                .await?;
            return Ok(());
        }
        Err(e) => {
            eprintln!("[user_stats] db error: {e}");
            ctx.say("failed to read stats").await?;
            return Ok(());
        }
    };
    let streak = ctx
        .data()
        .users
        .get_streak(target_id, guild_id)
        .await
        .unwrap_or(0);

    let last_active = stats
        .last_active
        .map(|t| t.format("%Y-%m-%d %H:%M UTC").to_string())
        .unwrap_or_else(|| "—".to_string());

    let body = format!(
        "score: {}\nrank: #{}\nstreak: {} days\nscoring events: {}\nlast scored: {}",
        stats.score, stats.rank, streak, stats.event_count, last_active,
    );
    let embed = CreateEmbed::new()
        .title(format!("stats — {}", target.name))
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
