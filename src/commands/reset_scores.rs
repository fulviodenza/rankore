use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::users::UsersRepo;
use crate::GlobalState;

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn reset_scores(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };
    let reply = match global_state.users.reset_scores(guild_id.0 as i64).await {
        Ok(()) => "Scores reset".to_string(),
        Err(e) => {
            eprintln!("[reset_scores] db error: {e}");
            "failed to reset scores".to_string()
        }
    };
    send_message(ctx, msg, reply).await;
    Ok(())
}
