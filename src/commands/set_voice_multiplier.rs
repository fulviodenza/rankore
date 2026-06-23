use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_voice_multiplier(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let parsed = args
        .clone()
        .single_quoted::<String>()
        .unwrap_or_default()
        .parse::<i64>();
    let Ok(multiplier) = parsed else {
        send_message(
            ctx,
            msg,
            "you need to insert a positive integer (e.g. 1, 2, 3)".to_string(),
        )
        .await;
        return Ok(());
    };

    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };

    let reply = match global_state
        .guilds
        .set_voice_multiplier(guild_id.0 as i64, multiplier)
        .await
    {
        Ok(()) => "Voice multiplier set".to_string(),
        Err(e) => {
            eprintln!("[set_voice_multiplier] db error: {e}");
            "failed to set voice multiplier".to_string()
        }
    };
    send_message(ctx, msg, reply).await;
    Ok(())
}
