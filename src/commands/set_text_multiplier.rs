use crate::commands::send_message;
use crate::db::guilds::GuildRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::{model::prelude::Message, prelude::Context};

#[command]
#[required_permissions(ADMINISTRATOR)]
async fn set_text_multiplier(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let multiplier_res = args
        .clone()
        .single_quoted::<String>()
        .unwrap_or_default()
        .parse::<i64>();
    let mut outgoing_msg = "you need to insert an integer number! If math is a problem, integer numbers are the numbers like 1, 2, 3".to_string();
    match multiplier_res {
        Ok(m) => {
            let data_read = ctx.data.read().await;
            outgoing_msg = "You don't have permissions to change text multiplier!".to_string();

            if let Some(global_state) = data_read.get::<GlobalState>() {
                let guild_state = global_state.guilds.lock().await;

                match guild_state
                    .set_text_multiplier(msg.guild_id.unwrap().0 as i64, m)
                    .await
                {
                    Ok(true) => {
                        outgoing_msg = "Text multiplier set".to_string();
                    }
                    Ok(false) => {}
                    Err(_) => {}
                }
            }
            send_message(ctx, msg, outgoing_msg).await;
            Ok(())
        }
        Err(_) => {
            send_message(ctx, msg, outgoing_msg).await;
            Ok(())
        }
    }
}
