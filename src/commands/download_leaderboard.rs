use crate::commands::send_titled_files;
use crate::db::users::UsersRepo;
use crate::GlobalState;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use xlsxwriter::*;

#[command]
async fn download_leaderboard(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state = global_state.users.lock().await;

        let mut users_vec = global_state.get_users(msg.guild_id.unwrap().0 as i64).await;
        users_vec.sort_by(|a: &crate::db::users::User, b| b.score.partial_cmp(&a.score).unwrap());

        let file_path = format!("./tmp/leaderboard-{}.xlsx", msg.guild_id.unwrap().0);

        println!("[download_leaderboard] creating file {}", file_path);
        let _ = create_xls_file(&file_path, users_vec.into_iter());

        let path = file_path.clone();
        send_titled_files(ctx, msg, path).await;
    }

    Ok(())
}

fn create_xls_file(file_path: &str, users_vec: std::vec::IntoIter<crate::db::users::User>) {
    let workbook = Workbook::new(&file_path).unwrap();
    let mut sheet = workbook.add_worksheet(None).unwrap();

    sheet.write_string(0, 0, "Username", None).unwrap();
    sheet.write_string(0, 1, "Score", None).unwrap();

    for (i, user) in users_vec.enumerate() {
        if !user.is_bot {
            sheet
                .write_string(i as u32 + 1, 0, &user.nick, None)
                .unwrap();
            sheet
                .write_number(i as u32 + 1, 1, user.score as f64, None)
                .unwrap();
        }
    }

    workbook.close().unwrap();
}
