use crate::commands::send_titled_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
async fn multipliers(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.guild.lock().await;
        let text_multiplier_result = global_state
            .get_text_multiplier(msg.guild_id.unwrap().0 as i64)
            .await;
        let mut text_multiplier = 1;
        if let Ok(m) = text_multiplier_result {
            text_multiplier = m
        }
        let voice_multiplier_result = global_state
            .get_voice_multiplier(msg.guild_id.unwrap().0 as i64)
            .await;
        let mut voice_multiplier = 1;
        if let Ok(m) = voice_multiplier_result {
            voice_multiplier = m
        }
        let msg_str = "text multiplier: ".to_string()
            + &text_multiplier.to_string()
            + "\nvoice multiplier: "
            + &voice_multiplier.to_string();

        send_titled_message(ctx, msg, "multipliers".to_string(), msg_str).await;
    }
    Ok(())
}
