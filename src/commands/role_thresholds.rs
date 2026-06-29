use poise::CreateReply;
use serenity::all::{CreateEmbed, Role};

use crate::{
    db::roles::RolesRepo,
    Context, Error,
};

/// Configure roles that are granted automatically when users hit score thresholds.
#[poise::command(
    prefix_command,
    slash_command,
    required_permissions = "ADMINISTRATOR",
    guild_only,
    subcommands("set", "remove", "list")
)]
pub async fn role_thresholds(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Grant a role automatically when a user reaches `score` points.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Role to grant"] role: Role,
    #[description = "Score threshold (positive integer)"] score: i64,
) -> Result<(), Error> {
    if score < 0 {
        ctx.say("score threshold must be non-negative").await?;
        return Ok(());
    }
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .roles
        .set_threshold(guild_id, role.id.get() as i64, score)
        .await
    {
        Ok(()) => format!("set threshold: {} at {} points", role.name, score),
        Err(e) => {
            eprintln!("[role_thresholds set] db error: {e}");
            "failed to set threshold".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// Remove the threshold for a role.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn remove(
    ctx: Context<'_>,
    #[description = "Role to stop auto-granting"] role: Role,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let reply = match ctx
        .data()
        .roles
        .remove_threshold(guild_id, role.id.get() as i64)
        .await
    {
        Ok(()) => format!("removed threshold for {}", role.name),
        Err(e) => {
            eprintln!("[role_thresholds remove] db error: {e}");
            "failed to remove threshold".to_string()
        }
    };
    ctx.say(reply).await?;
    Ok(())
}

/// List configured role thresholds for this guild.
#[poise::command(prefix_command, slash_command, guild_only)]
pub async fn list(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let rows = match ctx.data().roles.list_thresholds(guild_id).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[role_thresholds list] db error: {e}");
            ctx.say("failed to list thresholds").await?;
            return Ok(());
        }
    };
    let body = if rows.is_empty() {
        "(none configured)".to_string()
    } else {
        rows.into_iter()
            .map(|t| format!("- <@&{}> at {} points\n", t.role_id, t.score))
            .collect()
    };
    let embed = CreateEmbed::new()
        .title("role thresholds")
        .description(body)
        .colour((58, 8, 9));
    ctx.send(CreateReply::default().embed(embed).reply(true)).await?;
    Ok(())
}
