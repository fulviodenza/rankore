use serenity::{model::prelude::Message, prelude::Context};

pub mod download_leaderboard;
pub mod help;
pub mod leaderboard;
pub mod multipliers;
pub mod reset_scores;
pub mod set_prefix;
pub mod set_text_multiplier;
pub mod set_voice_multiplier;
pub mod set_welcome_msg;
use std::path::Path;

pub async fn send_message(ctx: &Context, msg: &Message, content: String) {
    let _ = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.allowed_mentions(|am| am.replied_user(true));
            m.reference_message(msg);
            m.content(content);
            m
        })
        .await;
}

pub async fn send_titled_message(ctx: &Context, msg: &Message, title: String, content: String) {
    let _ = msg
        .channel_id
        .send_message(&ctx.http, |m| {
            m.allowed_mentions(|am| am.replied_user(true));
            m.add_embed(|embed| embed.title(title).description(content).colour((58, 8, 9)))
                .reference_message(msg);
            m
        })
        .await;
}

pub async fn send_titled_files(ctx: &Context, msg: &Message, file_path: String) {
    let paths = vec![Path::new(&file_path)];

    let _ = msg
        .channel_id
        .send_files(&ctx.http, paths, |m| {
            m.allowed_mentions(|am| am.replied_user(true))
                .reference_message(msg)
        })
        .await;
}
