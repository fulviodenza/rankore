use crate::commands::send_titled_message;
use crate::db::users::UsersRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

const LEADERBOARD_LIMIT: i64 = 100;

#[command]
async fn leaderboard(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };

    let users = match global_state
        .users
        .get_leaderboard(guild_id.0 as i64, LEADERBOARD_LIMIT)
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

    send_titled_message(ctx, msg, "leaderboard".to_string(), body).await;
    Ok(())
}
