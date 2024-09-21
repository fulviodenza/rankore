use std::fs;

use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

use crate::commands::send_titled_message;

#[command]
async fn help(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let readme_file = fs::read_to_string("../assets/help.md").expect("Unable to read file");
    send_titled_message(ctx, msg, "Rankore /help".to_string(), readme_file).await;

    Ok(())
}
