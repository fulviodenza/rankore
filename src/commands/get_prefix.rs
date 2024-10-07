use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;

#[command]
async fn get_prefix(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.guilds.lock().await;
        let prefix = global_state
            .get_prefix(if let Some(guild_id) = msg.guild_id {
                guild_id.0 as i64
            } else {
                0
            })
            .await;
        send_message(ctx, msg, format!("Current prefix: {}", prefix)).await;
    }
    Ok(())
}
