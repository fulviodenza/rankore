use poise::CreateReply;
use serenity::all::CreateEmbed;
use std::fs;

use crate::{Context, Error};

#[poise::command(prefix_command)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    let body = fs::read_to_string("./assets/help.md")
        .unwrap_or_else(|_| "Help file not available.".to_string());
    let embed = CreateEmbed::new()
        .title("Rankore /help")
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
