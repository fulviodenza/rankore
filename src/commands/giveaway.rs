use std::time::Duration;

use redis::Commands;
use serenity::{
    client::Context,
    framework::standard::{macros::command, Args, CommandResult},
    model::channel::Message,
};
use sqlx::types::chrono::Local;

use crate::{commands::send_message, GlobalState};

#[command]
async fn giveaway(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut cloned_args = args.clone();
    let message_content = cloned_args.quoted().message();
    let vec_args = message_content.split_whitespace().collect::<Vec<&str>>();
    let (number_str, unit) = vec_args.split_at(1);

    let number: u64 = number_str[0].parse::<u64>().unwrap();
    let now = Local::now();

    let duration: u64 = match unit[0] {
        "day" | "days" => Duration::as_secs(&Duration::new(number * 24 * 60 * 60, 0)),
        "week" | "weeks" => Duration::as_secs(&Duration::new(number * 7 * 24 * 60 * 60, 0)),
        _ => 0,
    };

    let schedule = now + Duration::new(duration, 0);
    let data_read: tokio::sync::RwLockReadGuard<'_, serenity::prelude::TypeMap> =
        ctx.data.read().await;
    let timestamp_seconds =
        schedule.timestamp() + (schedule.timestamp_subsec_nanos() as f64 / 1_000_000_000.0) as i64;

    if let Some(global_state) = data_read.get::<GlobalState>() {
        let mut redis_state = global_state.redis.lock().await;
        let key = msg.guild_id.unwrap().0.to_string();
        let _ = redis_state
            .set::<String, String, String>(key.clone(), "OK".to_string())
            .unwrap();
        {
            let redis_res: redis::RedisResult<i32> =
                redis_state.expire_at(key, timestamp_seconds as usize);
            match redis_res {
                Ok(_) => println!("Key set and expired successfully"),
                Err(e) => eprintln!("Error expiring key: {}", e),
            }
        }
    }
    let outgoing_msg = format!(
        "event created with following schedule: {:?}",
        schedule.to_string(),
    );
    send_message(ctx, msg, outgoing_msg).await;

    Ok(())
}
