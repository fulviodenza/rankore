use std::sync::Arc;

use serenity::{model::voice::VoiceState, prelude::Context};

use crate::GlobalState;

pub async fn increase_score(ctx: Arc<Context>, user_id: u64) {
    let data_read = ctx.data.read().await;
    if let Some(_global_state) = data_read.get::<GlobalState>() {
        println!("user updated {:?}", user_id,);
    }
}

pub async fn handle_voice(ctx: Context, voice: VoiceState) {
    let user_id = voice.user_id.0;
    if let Some(global_state) = ctx.data.read().await.get::<GlobalState>() {
        let mut active_users = global_state.active_users.lock().await;
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
                match active_users.contains(&user_id) {
                    true => {}
                    false => {
                        global_state_users
                            .tx
                            .send(crate::db::events::UserEvents::Joined(user_id))
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
