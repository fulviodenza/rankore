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

        let mut users_vec = global_state.get_users().await;
        users_vec.sort_by(|a: &crate::db::users::User, b| b.score.partial_cmp(&a.score).unwrap());

        let mut msg_str = "".to_string();
        for (_, user) in users_vec.into_iter().enumerate() {
            if !user.nick.is_empty() {
                msg_str.push_str(&format!("{}: {}\n", user.nick, user.score));
            }
        }

        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.reference_message(msg);
                m.allowed_mentions(|am| am.replied_user(true));
                m.content(msg_str);
                m
            })
            .await;
    }
    Ok(())
}
