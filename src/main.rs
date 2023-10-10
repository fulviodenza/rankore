use std::env;

use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use async_trait::async_trait;
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::prelude::Ready,
    prelude::{Context, EventHandler, GatewayIntents, TypeMapKey},
    Client,
};

mod commands;

#[group]
#[commands(set_prefix)]
pub struct Bot;

pub struct GlobalStateInner {
    pub prefix: String,
}

pub struct GlobalState {}

impl GlobalStateInner {
    fn set_prefix(&mut self, prefix: String) {
        self.prefix = prefix
    }
}

impl TypeMapKey for GlobalState {
    type Value = GlobalStateInner;
}
struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, _: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
    }
}

#[tokio::main]
async fn main() {
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let framework = StandardFramework::new()
        .configure(|c| c.prefix("!"))
        .group(&BOT_GROUP);
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<GlobalState>(GlobalStateInner {
            prefix: "".to_string(),
        })
        .await
        .expect("Error creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why)
    }
}
