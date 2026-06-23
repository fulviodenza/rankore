use std::fs;
use std::path::Path;

use crate::commands::send_titled_files;
use crate::db::users::{User, UsersRepo};
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use xlsxwriter::*;

const LEADERBOARD_LIMIT: i64 = 10_000;

#[command]
async fn download_leaderboard(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let Some(guild_id) = msg.guild_id else {
        return Ok(());
    };
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return Ok(());
    };

    let users = match global_state
        .users
        .get_leaderboard(guild_id.0 as i64, LEADERBOARD_LIMIT)
        .await
    {
        Ok(u) => u,
        Err(e) => {
            eprintln!("[download_leaderboard] db error: {e}");
            return Ok(());
        }
    };

    let tmp_dir = std::env::temp_dir().join("rankore");
    if let Err(e) = fs::create_dir_all(&tmp_dir) {
        eprintln!("[download_leaderboard] failed to create tmp dir: {e}");
        return Ok(());
    }

    let file_path = tmp_dir.join(format!(
        "leaderboard-{}-{}.xlsx",
        guild_id.0,
        msg.id.0,
    ));

    if let Err(e) = create_xls_file(&file_path, &users) {
        eprintln!("[download_leaderboard] xlsx write failed: {e}");
        return Ok(());
    }

    send_titled_files(ctx, msg, file_path.to_string_lossy().into_owned()).await;
    let _ = fs::remove_file(&file_path);

    Ok(())
}

fn create_xls_file(file_path: &Path, users: &[User]) -> Result<(), XlsxError> {
    let path_str = file_path.to_string_lossy();
    let workbook = Workbook::new(&path_str)?;
    let mut sheet = workbook.add_worksheet(None)?;

    sheet.write_string(0, 0, "Username", None)?;
    sheet.write_string(0, 1, "Score", None)?;

    let mut row: u32 = 1;
    for user in users {
        sheet.write_string(row, 0, &user.nick, None)?;
        sheet.write_number(row, 1, user.score as f64, None)?;
        row += 1;
    }

    workbook.close()?;
    Ok(())
}
