use crate::commands::send_titled_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
async fn multipliers(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };
    let text = global_state.guilds.get_text_multiplier(guild_id.0 as i64).await;
    let voice = global_state.guilds.get_voice_multiplier(guild_id.0 as i64).await;
    let body = format!("text multiplier: {}\nvoice multiplier: {}", text, voice);
    send_titled_message(ctx, msg, "multipliers".to_string(), body).await;
    Ok(())
}
