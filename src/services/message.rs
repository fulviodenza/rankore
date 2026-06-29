use serenity::all::{ChannelId, Context, GuildId, Member, UserId, VoiceState};

use crate::{
    db::{channels::ChannelsRepo, guilds::GuildRepo},
    Data,
};

pub async fn increase_score(
    data: &Data,
    user_id: i64,
    nick: String,
    is_bot: bool,
    guild_id: i64,
    channel_id: i64,
) {
    let multiplier = match data.channels.get_text(guild_id, channel_id).await {
        Some(m) => m,
        None => data.guilds.get_text_multiplier(guild_id).await,
    };
    if let Err(e) = data.users.tx.send(crate::db::events::UserEvents::SentText(
        user_id, nick, is_bot, guild_id, multiplier,
    )) {
        eprintln!("[increase_score] event channel closed: {e}");
    }
}

pub async fn handle_voice(ctx: Context, data: &Data, voice: VoiceState) {
    let Some(guild_id) = voice.guild_id else {
        return;
    };
    let guild_id_i64 = guild_id.get() as i64;
    let user_id = voice.user_id.get() as i64;

    let key = (guild_id_i64, user_id);
    let mut active_users = data.active_users.lock().await;
    let is_active = active_users.contains(&key);

    if is_active && voice.channel_id.is_none() {
        active_users.remove(&key);
        drop(active_users);
        if let Err(e) = data
            .users
            .tx
            .send(crate::db::events::UserEvents::Left(user_id, guild_id_i64))
        {
            eprintln!("[handle_voice/Left] event channel closed: {e}");
        }
    } else if !is_active && voice.channel_id.is_some() {
        let is_bot = voice.member.as_ref().map(|m| m.user.bot).unwrap_or(false);
        let nick = match guild_id.member(&ctx.http, voice.user_id).await {
            Ok(m) => m
                .nick
                .clone()
                .unwrap_or_else(|| m.display_name().to_string()),
            Err(_) => voice
                .member
                .as_ref()
                .map(|m| m.display_name().to_string())
                .unwrap_or_else(|| voice.user_id.get().to_string()),
        };
        let voice_channel_id = voice.channel_id.unwrap().get() as i64;
        let multiplier = match data.channels.get_voice(guild_id_i64, voice_channel_id).await {
            Some(m) => m,
            None => data.guilds.get_voice_multiplier(guild_id_i64).await,
        };

        active_users.insert(key);
        drop(active_users);

        if let Err(e) = data.users.tx.send(crate::db::events::UserEvents::Joined(
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

pub async fn init_active_users(_ctx: Context, data: &Data, voice: VoiceStateReady) {
    let guild_id_i64 = voice.guild_id.get() as i64;
    let user_id = voice.user_id.get() as i64;
    let channel_id = voice._channel_id.get() as i64;
    let key = (guild_id_i64, user_id);

    let mut active_users = data.active_users.lock().await;
    if active_users.contains(&key) {
        return;
    }
    active_users.insert(key);
    drop(active_users);

    let multiplier = match data.channels.get_voice(guild_id_i64, channel_id).await {
        Some(m) => m,
        None => data.guilds.get_voice_multiplier(guild_id_i64).await,
    };
    let nick = voice
        .member
        .nick
        .clone()
        .unwrap_or_else(|| voice.member.display_name().to_string());

    if let Err(e) = data.users.tx.send(crate::db::events::UserEvents::Joined(
        user_id,
        nick,
        voice.member.user.bot,
        guild_id_i64,
        multiplier,
    )) {
        eprintln!("[init_active_users] event channel closed: {e}");
    }
}
