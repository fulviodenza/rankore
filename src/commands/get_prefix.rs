use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;

#[command]
async fn get_prefix(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };
    let prefix = global_state.guilds.get_prefix(guild_id.0 as i64).await;
    send_message(ctx, msg, format!("Current prefix: {}", prefix)).await;
    Ok(())
}
