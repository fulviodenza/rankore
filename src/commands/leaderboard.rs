use chrono::{Duration, Utc};
use poise::CreateReply;
use serenity::all::CreateEmbed;

use crate::{db::users::UsersRepo, Context, Error};

const LEADERBOARD_LIMIT: i64 = 100;

#[derive(Debug, poise::ChoiceParameter)]
pub enum Period {
    #[name = "all-time"]
    All,
    #[name = "week"]
    Week,
    #[name = "month"]
    Month,
}

#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn leaderboard(
    ctx: Context<'_>,
    #[description = "Time window (default: all-time)"] period: Option<Period>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let period = period.unwrap_or(Period::All);
    let users = match &period {
        Period::All => ctx.data().users.get_leaderboard(guild_id, LEADERBOARD_LIMIT).await,
        Period::Week => {
            let since = Utc::now() - Duration::days(7);
            ctx.data()
                .users
                .get_period_leaderboard(guild_id, since, LEADERBOARD_LIMIT)
                .await
        }
        Period::Month => {
            let since = Utc::now() - Duration::days(30);
            ctx.data()
                .users
                .get_period_leaderboard(guild_id, since, LEADERBOARD_LIMIT)
                .await
        }
    };
    let users = match users {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[leaderboard] db error: {e}");
            return Ok(());
        }
    };
    let title = match period {
        Period::All => "leaderboard (all-time)",
        Period::Week => "leaderboard (last 7 days)",
        Period::Month => "leaderboard (last 30 days)",
    };
    let body = users
        .into_iter()
        .map(|u| format!("{}: {}\n", u.nick, u.score))
        .collect::<String>();
    let embed = CreateEmbed::new()
        .title(title)
        .description(if body.is_empty() { "(empty)".to_string() } else { body })
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
