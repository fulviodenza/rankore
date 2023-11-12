use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_voice_multiplier(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let multiplier = args
        .clone()
        .single_quoted::<String>()
        .unwrap_or_default()
        .parse::<i64>()
        .unwrap();
    let data_read = ctx.data.read().await;
    let mut outgoing_msg: String =
        "You don't have permissions to change voice multiplier!".to_string();

    if let Some(global_state) = data_read.get::<GlobalState>() {
        let guild_state = global_state.guilds.lock().await;

        if guild_state
            .set_voice_multiplier(msg.guild_id.unwrap().0 as i64, multiplier)
            .await?
        {
            outgoing_msg = "Voice multiplier set".to_string();
        }
    }
    send_message(ctx, msg, outgoing_msg).await;
    Ok(())
}
