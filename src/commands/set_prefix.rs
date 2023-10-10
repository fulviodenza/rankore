use serenity::framework::standard::macros::command;

use serenity::framework::standard::CommandResult;
use serenity::{model::prelude::Message, prelude::Context};

use crate::GlobalState;

#[command]
async fn set_prefix(ctx: &Context, msg: &Message) -> CommandResult {
    let mut x = ctx.data.write().await;
    let x = x.get_mut::<GlobalState>().unwrap();
    let new_prefix = msg.content.split(' ').collect::<Vec<_>>();
    let new_prefix = new_prefix.get(1).unwrap();

    x.set_prefix(new_prefix.to_string());
    let outgoing_msg = format!("New message set: {:?}", new_prefix);
    if let Err(why) = msg.channel_id.say(&ctx, outgoing_msg).await {
        println!("Error sending message :{:?}", why)
    }

    Ok(())
}
