use std::fs;

use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
async fn help(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let readme_file = fs::read_to_string("./README.md").expect("Unable to read file");
    let _ = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.allowed_mentions(|am| am.replied_user(true));
            m.add_embed(|embed| {
                embed
                    .title("Rankore /help")
                    .description(readme_file)
                    .colour((58, 8, 9))
            })
            .reference_message(msg);
            m
        })
        .await;

    Ok(())
}
