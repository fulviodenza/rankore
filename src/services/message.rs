use std::borrow::BorrowMut;

use serenity::{
    model::{prelude::Message, voice::VoiceState},
    prelude::Context,
};

use crate::{
    db::users::{User, UsersRepo},
    GlobalState,
};

pub async fn handle_message(ctx: Context, msg: Message) {
    let user_id = msg.author.id.0;

    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let mut global_state = global_state.users.lock().await;
        let mut user = global_state.get_user(user_id).await;
        let user_mut = user.borrow_mut().clone();
        global_state
            .update_user(User {
                id: user_id,
                score: user_mut.score + 1,
            })
            .await;
        println!(
            "user updated {:?} with score {:?}",
            global_state.get_user(user_id).await,
            user_mut.score + 1
        );
    }
}

pub async fn handle_voice(ctx: Context, voice: VoiceState) {
    let user_id = voice.user_id.0;
    println!("{:?}", voice);
    // The user didn't left the channel
    if voice.channel_id.is_some() {
        increase_score(ctx, user_id).await;
    } else {
        println!("Bye!")
    }
}

async fn increase_score(ctx: Context, user_id: u64) {
    let data_read = ctx.data.read().await;
    if let Some(global_state) = data_read.get::<GlobalState>() {
        let mut global_state = global_state.users.lock().await;
        let mut user = global_state.get_user(user_id).await;
        let user_mut = user.borrow_mut().clone();
        global_state
            .update_user(User {
                id: user_id,
                score: user_mut.score + 1,
            })
            .await;
        println!(
            "user updated {:?} with score {:?}",
            global_state.get_user(user_id).await,
            user_mut.score + 1
        );
    }
}
