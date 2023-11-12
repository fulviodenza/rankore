use crate::db::guild::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_text_multiplier(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let multiplier = args
        .clone()
        .single_quoted::<String>()
        .unwrap_or_default()
        .parse::<i64>()
        .unwrap();
    let data_read = ctx.data.read().await;
    let mut outgoing_msg: String =
        "You don't have permissions to change text multiplier!".to_string();

    if let Some(global_state) = data_read.get::<GlobalState>() {
        let guild_state = global_state.guild.lock().await;

        match guild_state
            .set_text_multiplier(msg.guild_id.unwrap().0 as i64, multiplier)
            .await
        {
            Ok(true) => {
                outgoing_msg = "Text multiplier set".to_string();
            }
            Ok(false) => {}
            Err(_) => {}
        }
    }
    let _ = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.allowed_mentions(|am| am.replied_user(true));
            m.add_embed(|embed| embed.description(outgoing_msg).colour((58, 8, 9)));
            m
        })
        .await;
    Ok(())
}
