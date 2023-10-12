use std::borrow::BorrowMut;

use serenity::{model::prelude::Message, prelude::Context};

use crate::{
    db::users::{User, UsersRepo},
    GlobalState,
};

pub async fn handle_message(ctx: Context, msg: Message) {
    let user_id = msg.author.id.0;
    {
        let data_read = ctx.data.read().await;
        if let Some(global_state) = data_read.get::<GlobalState>() {
            let mut global_state = global_state.users.lock().expect("Failed to aquire mutex");
            let mut user = &global_state.get_user(user_id);
            let user_mut = user.borrow_mut().clone();
            global_state.update_user(User {
                id: user_id,
                score: user_mut.score + 1,
            });
            println!("{:?}", user_mut.score + 1);
            println!("user updated {:?}", global_state.get_user(user_id));
        }
    }
}
