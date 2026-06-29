use crate::{db::guilds::GuildRepo, Context, Error};

/// Set the percent of points to remove from every user's score each day. 0 disables decay.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn set_decay_rate(
    ctx: Context<'_>,
    #[description = "Percent per day (0-100). 0 disables decay."] pct: i64,
) -> Result<(), Error> {
    if !(0..=100).contains(&pct) {
        ctx.say("decay rate must be between 0 and 100").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx.data().guilds.set_decay_rate(guild_id, pct as i32).await {
        Ok(()) if pct == 0 => "score decay disabled".to_string(),
        Ok(()) => format!("score decay set to {}%/day", pct),
        Err(e) => {
            eprintln!("[set_decay_rate] db error: {e}");
            "failed to set decay rate".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}
