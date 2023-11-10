use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::db::users::UsersRepo;
use crate::GlobalState;

#[command]
async fn reset_scores(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    let member = msg.member(&ctx.http).await.unwrap();
    println!("{:?}", member);
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.users.lock().await;
        let guild_id = msg.guild_id.unwrap().0;
        global_state.reset_scores(guild_id as i64).await;
        let _ = msg
            .channel_id
            .send_message(&ctx.http, |m| {
                m.allowed_mentions(|am| am.replied_user(true));
                m.reference_message(msg);
                m.content("Scores resetted");
                m
            })
            .await;
    }

    Ok(())
}
