use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_welcome_msg(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };
    let reply = match global_state
        .guilds
        .set_welcome_msg(guild_id.0 as i64, args.message())
        .await
    {
        Ok(()) => "welcome message set".to_string(),
        Err(e) => {
            eprintln!("[set_welcome_msg] db error: {e}");
            "failed to set welcome message".to_string()
        }
    };
    send_message(ctx, msg, reply).await;
    Ok(())
}
