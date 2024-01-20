use crate::commands::send_titled_message;
use crate::db::users::UsersRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
async fn leaderboard(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.users.lock().await;

        let mut users_vec = global_state.get_users(msg.guild_id.unwrap().0 as i64).await;
        users_vec.sort_by(|a: &crate::db::users::User, b| b.score.partial_cmp(&a.score).unwrap());

        let mut msg_str = "".to_string();
        for user in users_vec.into_iter() {
            if !user.is_bot {
                msg_str.push_str(&format!("{}: {}\n", user.nick, user.score));
            }
        }
        send_titled_message(ctx, msg, "leaderboard".to_string(), msg_str).await;
    }
    Ok(())
}
