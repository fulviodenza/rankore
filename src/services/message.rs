use serenity::{
    model::{
        prelude::{ChannelId, GuildId, Member, UserId},
        voice::VoiceState,
    },
    prelude::Context,
};

use crate::{db::guilds::GuildRepo, GlobalState};

pub async fn increase_score(
    ctx: &Context,
    user_id: i64,
    nick: String,
    is_bot: bool,
    guild_id: i64,
) {
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return;
    };

    let multiplier = global_state.guilds.get_text_multiplier(guild_id).await;
    if let Err(e) = global_state.users.tx.send(crate::db::events::UserEvents::SentText(
        user_id, nick, is_bot, guild_id, multiplier,
    )) {
        eprintln!("[increase_score] event channel closed: {e}");
    }
}

pub async fn handle_voice(ctx: Context, voice: VoiceState) {
    let Some(guild_id) = voice.guild_id else {
        return;
    };
    let guild_id_i64 = guild_id.0 as i64;
    let user_id = voice.user_id.0 as i64;

    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return;
    };

    let key = (guild_id_i64, user_id);
    let mut active_users = global_state.active_users.lock().await;
    let is_active = active_users.contains(&key);

    if is_active && voice.channel_id.is_none() {
        // User left voice in this guild.
        active_users.remove(&key);
        drop(active_users);
        if let Err(e) = global_state
            .users
            .tx
            .send(crate::db::events::UserEvents::Left(user_id, guild_id_i64))
        {
            eprintln!("[handle_voice/Left] event channel closed: {e}");
        }
    } else if !is_active && voice.channel_id.is_some() {
        // User joined voice in this guild.
        let is_bot = voice.member.as_ref().map(|m| m.user.bot).unwrap_or(false);
        let nick = match guild_id.member(&ctx.http, voice.user_id.0).await {
            Ok(m) => m
                .nick
                .clone()
                .unwrap_or_else(|| m.display_name().to_string()),
            Err(_) => voice
                .member
                .as_ref()
                .map(|m| m.display_name().to_string())
                .unwrap_or_else(|| voice.user_id.0.to_string()),
        };
        let multiplier = global_state.guilds.get_voice_multiplier(guild_id_i64).await;

        active_users.insert(key);
        drop(active_users);

        if let Err(e) = global_state.users.tx.send(crate::db::events::UserEvents::Joined(
            user_id,
            nick,
            is_bot,
            guild_id_i64,
            multiplier,
        )) {
            eprintln!("[handle_voice/Joined] event channel closed: {e}");
        }
    }
}

pub struct VoiceStateReady {
    pub member: Member,
    pub user_id: UserId,
    pub _channel_id: ChannelId,
    pub guild_id: GuildId,
}

pub async fn init_active_users(ctx: Context, voice: VoiceStateReady) {
    let data_read = ctx.data.read().await;
    let Some(global_state) = data_read.get::<GlobalState>() else {
        return;
    };

    let guild_id_i64 = voice.guild_id.0 as i64;
    let user_id = voice.user_id.0 as i64;
    let key = (guild_id_i64, user_id);

    let mut active_users = global_state.active_users.lock().await;
    if active_users.contains(&key) {
        return;
    }
    active_users.insert(key);
    drop(active_users);

    let multiplier = global_state.guilds.get_voice_multiplier(guild_id_i64).await;
    let nick = voice
        .member
        .nick
        .clone()
        .unwrap_or_else(|| voice.member.display_name().to_string());

    if let Err(e) = global_state.users.tx.send(crate::db::events::UserEvents::Joined(
        user_id,
        nick,
        voice.member.user.bot,
        guild_id_i64,
        multiplier,
    )) {
        eprintln!("[init_active_users] event channel closed: {e}");
    }
}
