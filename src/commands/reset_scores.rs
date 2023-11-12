use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::users::UsersRepo;
use crate::GlobalState;

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn reset_scores(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.users.lock().await;
        let guild_id = msg.guild_id.unwrap().0;
        global_state.reset_scores(guild_id as i64).await;
        send_message(ctx, msg, "Scores resetted".to_string()).await;
    }

    Ok(())
}
