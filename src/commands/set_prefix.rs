use serenity::framework::standard::macros::command;

use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::db::guild::GuildRepo;
use crate::GlobalState;

#[command]
async fn set_prefix(ctx: &Context, _msg: &Message, args: Args) -> CommandResult {
    let mut cloned_args = args.clone();
    let new_prefix = cloned_args.single_quoted::<String>().unwrap_or_default();

    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let mut global_state = global_state.guild.lock().expect("Failed to aquire mutex");
        global_state.set_prefix(new_prefix);
    }
    Ok(())
}
