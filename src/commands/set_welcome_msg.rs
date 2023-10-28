use serenity::framework::standard::macros::command;

use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::db::guild::GuildRepo;
use crate::GlobalState;

#[command]
async fn set_welcome_msg(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut cloned_args = args.clone();
    let welcome_msg = cloned_args.single_quoted::<String>().unwrap_or_default();

    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.guild.lock().await;
        global_state
            .set_welcome_msg(
                if let Some(guild_id) = msg.guild_id {
                    guild_id.0 as i64
                } else {
                    0
                },
                &welcome_msg,
            )
            .await;
    }
    Ok(())
}
