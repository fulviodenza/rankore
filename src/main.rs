use std::{
    collections::{HashMap, HashSet},
    env,
    path::PathBuf,
    sync::Arc,
};

use db::{
    guilds::{GuildRepo, Guilds},
    users::{Users, UsersRepo},
};
use serenity::{
    all::{ChannelType, FullEvent, GatewayIntents, GuildId},
    Client,
};
use songbird::SerenityInit;
use sqlx::postgres::PgPoolOptions;
use tokio::sync::Mutex;

mod commands;
mod db;
mod services;
mod voice;

use crate::voice::session::TranscriptSession;

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Clone)]
pub struct Data {
    pub guilds: Arc<Guilds>,
    pub users: Arc<Users>,
    pub active_users: Arc<Mutex<HashSet<(i64, i64)>>>,
    pub sessions: Arc<Mutex<HashMap<GuildId, TranscriptSession>>>,
    pub whisper_url: String,
    pub transcripts_dir: PathBuf,
    pub http: reqwest::Client,
}

#[tokio::main]
async fn main() {
    // Initialize tracing so RUST_LOG controls log output from serenity, poise,
    // songbird, sqlx, reqwest, and our own tracing::* calls. Without this,
    // those crates emit nothing.
    //
    // Sensible defaults if RUST_LOG isn't set:
    //   info from this crate + warn from everything else.
    // To debug voice:
    //   RUST_LOG=warn,rankore=debug,songbird=debug,songbird::driver=trace
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,rankore=info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_ansi(false)
        .init();

    tracing::info!("starting rankore");

    let db_url = env::var("DATABASE_URL")
        .expect("Expected a token for database in the environment");

    let pool = {
        let mut attempt = 0;
        loop {
            match PgPoolOptions::new().max_connections(50).connect(&db_url).await {
                Ok(p) => break p,
                Err(e) if attempt < 10 => {
                    eprintln!("[db] connect attempt {attempt} failed: {e}; retrying...");
                    attempt += 1;
                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                }
                Err(e) => panic!("failed to connect to database: {e}"),
            }
        }
    };

    if let Err(e) = sqlx::migrate!("./migrations").run(&pool).await {
        panic!("failed to run migrations: {e}");
    }

    let token = env::var("DISCORD_TOKEN")
        .expect("Expected a token for discord in the environment");
    let whisper_url =
        env::var("WHISPER_URL").unwrap_or_else(|_| "http://whisper:8000".to_string());
    let transcripts_dir = PathBuf::from(
        env::var("TRANSCRIPTS_DIR").unwrap_or_else(|_| "/tmp/rankore-transcripts".to_string()),
    );
    if let Err(e) = std::fs::create_dir_all(&transcripts_dir) {
        eprintln!("[startup] failed to create transcripts dir: {e}");
    }

    let intents = GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_VOICE_STATES
        | GatewayIntents::GUILD_MEMBERS;

    let guilds = Guilds::new(&pool).await;
    let users = Users::new(&pool).await;
    let active_users = Arc::new(Mutex::new(HashSet::new()));
    let sessions = Arc::new(Mutex::new(HashMap::new()));
    let http = reqwest::Client::new();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::set_prefix::set_prefix(),
                commands::get_prefix::get_prefix(),
                commands::leaderboard::leaderboard(),
                commands::set_welcome_msg::set_welcome_msg(),
                commands::help::help(),
                commands::reset_scores::reset_scores(),
                commands::set_voice_multiplier::set_voice_multiplier(),
                commands::set_text_multiplier::set_text_multiplier(),
                commands::multipliers::multipliers(),
                commands::download_leaderboard::download_leaderboard(),
                commands::transcribe::transcribe_join(),
                commands::transcribe::transcribe_leave(),
                commands::transcribe::transcribe_status(),
            ],
            prefix_options: poise::PrefixFrameworkOptions {
                dynamic_prefix: Some(|ctx| {
                    Box::pin(async move {
                        let Some(guild_id) = ctx.guild_id else {
                            return Ok(None);
                        };
                        Ok(Some(ctx.data.guilds.get_prefix(guild_id.get() as i64).await))
                    })
                }),
                ..Default::default()
            },
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(move |_ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    guilds,
                    users,
                    active_users,
                    sessions,
                    whisper_url,
                    transcripts_dir,
                    http,
                })
            })
        })
        .build();

    let mut client = Client::builder(token, intents)
        .framework(framework)
        .register_songbird()
        .await
        .expect("Error creating client");

    if let Err(why) = client.start().await {
        eprintln!("Client error: {:?}", why);
    }
}

async fn event_handler(
    ctx: &serenity::all::Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        FullEvent::Message { new_message } => {
            if new_message.author.bot {
                return Ok(());
            }
            let Some(guild_id) = new_message.guild_id else {
                return Ok(());
            };
            crate::services::message::increase_score(
                data,
                new_message.author.id.get() as i64,
                new_message.author.name.clone(),
                new_message.author.bot,
                guild_id.get() as i64,
            )
            .await;
        }
        FullEvent::GuildMemberAddition { new_member } => {
            let Some(channel_id) =
                find_welcome_channel(ctx, new_member.guild_id.get()).await
            else {
                return Ok(());
            };
            let template = data
                .guilds
                .get_welcome_msg(new_member.guild_id.get() as i64)
                .await
                .filter(|s| !s.is_empty());
            let welcome_message = match template {
                Some(m) => format!("{}, <@{}>!", m, new_member.user.id),
                None => format!("Welcome, <@{}>!", new_member.user.id),
            };
            if let Err(e) = channel_id.say(&ctx.http, welcome_message).await {
                eprintln!("[guild_member_addition] failed to send welcome: {e}");
            }
        }
        FullEvent::VoiceStateUpdate { old: _, new } => {
            crate::services::message::handle_voice(ctx.clone(), data, new.clone()).await;
        }
        FullEvent::Ready { data_about_bot } => {
            println!("{} is connected!", data_about_bot.user.name);
            init_voice_state(ctx, data).await;
        }
        _ => {}
    }
    Ok(())
}

async fn find_welcome_channel(
    ctx: &serenity::all::Context,
    guild_id: u64,
) -> Option<serenity::all::ChannelId> {
    let guild_id = GuildId::new(guild_id);
    let channels = guild_id.channels(&ctx.http).await.ok()?;
    channels
        .into_iter()
        .find(|(_, c)| c.name == "welcome")
        .map(|(id, _)| id)
}

async fn init_voice_state(ctx: &serenity::all::Context, data: &Data) {
    use crate::services::message::{init_active_users, VoiceStateReady};

    let guilds = match data.guilds.guilds().await {
        Ok(g) => g,
        Err(e) => {
            eprintln!("[ready] failed to list guilds: {e}");
            return;
        }
    };
    let guild_ids: Vec<i64> = guilds.into_iter().filter_map(|g| g.id).collect();
    for id in guild_ids {
        let guild_id = GuildId::new(id as u64);
        let Ok(channels) = guild_id.channels(&ctx.http).await else {
            continue;
        };
        for (channel_id, channel) in channels {
            if channel.kind != ChannelType::Voice {
                continue;
            }
            let Ok(members) = channel.members(&ctx.cache) else {
                continue;
            };
            for m in members {
                init_active_users(
                    ctx.clone(),
                    data,
                    VoiceStateReady {
                        member: m.clone(),
                        user_id: m.user.id,
                        _channel_id: channel_id,
                        guild_id: channel.guild_id,
                    },
                )
                .await;
            }
        }
    }
}
