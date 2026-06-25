use std::fs;
use std::path::{Path, PathBuf};

use poise::CreateReply;
use serenity::all::CreateAttachment;
use xlsxwriter::*;

use crate::{
    db::users::{User, UsersRepo},
    Context, Error,
};

const LEADERBOARD_LIMIT: i64 = 10_000;

#[poise::command(prefix_command, guild_only)]
pub async fn download_leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().unwrap().get() as i64;
    let users = match ctx
        .data()
        .users
        .get_leaderboard(guild_id, LEADERBOARD_LIMIT)
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
    let file_path = tmp_dir.join(format!("leaderboard-{}-{}.xlsx", guild_id, ctx.id()));

    if let Err(e) = create_xls_file(&file_path, &users) {
        eprintln!("[download_leaderboard] xlsx write failed: {e}");
        return Ok(());
    }

    let attachment = match CreateAttachment::path(&file_path).await {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[download_leaderboard] attachment: {e}");
            let _ = fs::remove_file(&file_path);
            return Ok(());
        }
    };
    ctx.send(CreateReply::default().attachment(attachment).reply(true))
        .await?;
    let _ = fs::remove_file(&file_path);

    Ok(())
}

fn create_xls_file(file_path: &Path, users: &[User]) -> Result<(), XlsxError> {
    let path: PathBuf = file_path.to_path_buf();
    let path_str = path.to_string_lossy();
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
