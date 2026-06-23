use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_prefix(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let new_prefix = args.clone().single_quoted::<String>().unwrap_or_default();
    if new_prefix.is_empty() {
        send_message(ctx, msg, "prefix cannot be empty".to_string()).await;
        return Ok(());
    }

    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };

    let reply = match global_state.guilds.set_prefix(guild_id.0 as i64, &new_prefix).await {
        Ok(()) => "prefix updated!".to_string(),
        Err(e) => {
            eprintln!("[set_prefix] db error: {e}");
            "failed to update prefix".to_string()
        }
    };
    send_message(ctx, msg, reply).await;
    Ok(())
}
