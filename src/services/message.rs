use std::sync::Arc;

use serenity::{model::voice::VoiceState, prelude::Context};

use crate::{db::guild::GuildRepo, GlobalState};

pub async fn increase_score(
    ctx: Arc<Context>,
    user_id: i64,
    nick: String,
    is_bot: bool,
    guild_id: i64,
) {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let global_state_users = global_state.users.lock().await.clone();
        let multiplier_result = global_state
            .guild
            .lock()
            .await
            .get_text_multiplier(guild_id)
            .await;

        let mut multiplier = 1;
        if let Ok(mult) = multiplier_result {
            multiplier = mult
        }
        global_state_users
            .tx
            .send(crate::db::events::UserEvents::SentText(
                user_id, nick, is_bot, guild_id, multiplier,
            ))
            .unwrap();
        println!("user: {:?} sent message", user_id);
    }
}

pub async fn handle_voice(ctx: Context, voice: VoiceState) {
    let is_bot = voice.member.clone().unwrap().user.bot;
    let user_id = voice.user_id.0 as i64;
    if let Some(global_state) = ctx.data.read().await.get::<GlobalState>() {
        let mut active_users = global_state.active_users.lock().await;
        let guilds = global_state.guild.lock().await;
        if active_users.contains(&user_id) && voice.channel_id.is_none() {
            // the user left the channel
            let global_state_users = global_state.users.lock().await.clone();
            global_state_users
                .tx
                .send(crate::db::events::UserEvents::Left(user_id))
                .unwrap();
            active_users.remove(&user_id);
            println!("Bye!");
        } else if !active_users.contains(&user_id) {
            // The user didn't leave the channel
            if voice.channel_id.is_some() {
                println!("{:?}", voice.channel_id);
                let global_state_users = global_state.users.lock().await.clone();
                let mut nick: String = "".to_string();
                let mut multiplier = 1;
                if let Some(guild_id) = voice.guild_id {
                    // assign to nick the nick in the guild
                    // if any or assign the display name
                    nick = guild_id
                        .member(&ctx.http, voice.user_id.0)
                        .await
                        .ok()
                        .unwrap()
                        .nick
                        .unwrap_or_else(|| voice.member.unwrap().display_name().to_string());
                    let multiplier_result = guilds.get_voice_multiplier(guild_id.0 as i64).await;
                    if let Ok(r) = multiplier_result {
                        multiplier = r
                    }
                }

                match active_users.contains(&user_id) {
                    true => {}
                    false => {
                        global_state_users
                            .tx
                            .send(crate::db::events::UserEvents::Joined(
                                user_id,
                                nick,
                                is_bot,
                                voice.guild_id.unwrap().0 as i64,
                                multiplier,
                            ))
                            .unwrap();
                        active_users.insert(user_id);
                    }
                }
            }
        } else {
            println!("Nothing to do!");
        }
    }
}
