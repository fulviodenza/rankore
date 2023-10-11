use std::{env, sync::Mutex};

use crate::commands::set_prefix::SET_PREFIX_COMMAND;
use async_trait::async_trait;
use db::guild::{Guild, GuildRepo};
use serenity::{
    framework::{standard::macros::group, StandardFramework},
    model::prelude::Ready,
    prelude::{Context, EventHandler, GatewayIntents, TypeMapKey},
    Client,
};

mod commands;
mod db;

#[group]
#[commands(set_prefix)]
pub struct Bot;

pub struct GlobalStateInner {
    guild: Mutex<Guild>,
}

pub struct GlobalState {}

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
        .configure(|c| {
            c.dynamic_prefix(|_, _msg| Box::pin(async move { Some({ "!" }.to_string()) }))
        })
        .group(&BOT_GROUP);
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .framework(framework)
        .type_map_insert::<GlobalState>(GlobalStateInner {
            guild: Mutex::new(Guild::new()),
        })
        .await
        .expect("Error creating client");
    if let Err(why) = client.start().await {
        println!("Client error: {:?}", why)
    }
}
