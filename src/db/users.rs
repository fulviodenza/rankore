use async_trait::async_trait;
use sqlx::{Error, Pool, Postgres};
use std::{collections::HashMap, sync::Arc};
use tokio::{
    select,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot, Mutex,
    },
};

use super::events::{Observer, UserEvents};

pub struct Users {
    pub pool: Pool<Postgres>,
    pub tx: UnboundedSender<UserEvents>,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct User {
    pub id: i64,
    pub score: i64,
    pub nick: String,
    pub is_bot: bool,
    pub guild_id: i64,
    pub has_left: bool,
}

#[async_trait]
pub trait UsersRepo {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self>;
    async fn increment_score(
        pool: &Pool<Postgres>,
        id: i64,
        guild_id: i64,
        nick: &str,
        is_bot: bool,
        delta: i64,
    ) -> Result<(), Error>;
    async fn mark_left(pool: &Pool<Postgres>, id: i64, guild_id: i64) -> Result<(), Error>;
    async fn get_leaderboard(&self, guild_id: i64, limit: i64) -> Result<Vec<User>, Error>;
    async fn reset_scores(&self, guild_id: i64) -> Result<(), Error>;
}

#[async_trait]
impl UsersRepo for Users {
    async fn new(pool: &Pool<Postgres>) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<UserEvents>();
        let users = Arc::new(Users {
            tx,
            pool: pool.clone(),
        });
        let users_clone = Arc::clone(&users);
        tokio::spawn(async move {
            users_clone.notify(rx).await;
        });
        users
    }

    async fn increment_score(
        pool: &Pool<Postgres>,
        id: i64,
        guild_id: i64,
        nick: &str,
        is_bot: bool,
        delta: i64,
    ) -> Result<(), Error> {
        sqlx::query!(
            "INSERT INTO users (id, score, nick, is_bot, guild_id, hasleft) \
             VALUES ($1, $2, $3, $4, $5, false) \
             ON CONFLICT (id, guild_id) DO UPDATE \
             SET score = users.score + EXCLUDED.score, \
                 nick = EXCLUDED.nick, \
                 is_bot = EXCLUDED.is_bot, \
                 hasleft = false",
            id,
            delta,
            nick,
            is_bot,
            guild_id,
        )
        .execute(pool)
        .await
        .map(|_| ())
    }

    async fn mark_left(pool: &Pool<Postgres>, id: i64, guild_id: i64) -> Result<(), Error> {
        sqlx::query!(
            "UPDATE users SET hasleft = true WHERE id = $1 AND guild_id = $2",
            id,
            guild_id,
        )
        .execute(pool)
        .await
        .map(|_| ())
    }

    async fn get_leaderboard(&self, guild_id: i64, limit: i64) -> Result<Vec<User>, Error> {
        let rows = sqlx::query!(
            "SELECT id, score, nick, is_bot, guild_id, hasleft \
             FROM users \
             WHERE guild_id = $1 AND is_bot = false \
             ORDER BY score DESC \
             LIMIT $2",
            guild_id,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| User {
                id: row.id,
                score: row.score,
                nick: row.nick,
                is_bot: row.is_bot,
                guild_id: row.guild_id,
                has_left: row.hasleft.unwrap_or(false),
            })
            .collect())
    }

    async fn reset_scores(&self, guild_id: i64) -> Result<(), Error> {
        sqlx::query!("UPDATE users SET score = 0 WHERE guild_id = $1", guild_id,)
            .execute(&self.pool)
            .await
            .map(|_| ())
    }
}

#[async_trait]
impl Observer for Users {
    async fn notify(&self, mut rx: UnboundedReceiver<UserEvents>) {
        // (user_id, guild_id) -> cancel sender for the per-user voice tick task
        let tickers: Arc<Mutex<HashMap<(i64, i64), oneshot::Sender<()>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        while let Some(event) = rx.recv().await {
            match event {
                UserEvents::Joined(user_id, nick, is_bot, guild_id, interval_secs) => {
                    let interval = interval_secs.max(1) as u64;
                    let (cancel_tx, mut cancel_rx) = oneshot::channel::<()>();

                    // If a previous ticker exists for this (user, guild), cancel it.
                    let prev = tickers.lock().await.insert((user_id, guild_id), cancel_tx);
                    if let Some(prev_cancel) = prev {
                        let _ = prev_cancel.send(());
                    }

                    let pool = self.pool.clone();
                    let tickers = tickers.clone();
                    tokio::spawn(async move {
                        loop {
                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {
                                    if let Err(e) = Users::increment_score(
                                        &pool, user_id, guild_id, &nick, is_bot, 1,
                                    ).await {
                                        eprintln!("[voice tick] increment_score failed: {e}");
                                    }
                                }
                                _ = &mut cancel_rx => break,
                            }
                        }
                        // Best-effort cleanup; only remove if our entry is still current.
                        let mut map = tickers.lock().await;
                        if let Some(entry) = map.get(&(user_id, guild_id)) {
                            if entry.is_closed() {
                                map.remove(&(user_id, guild_id));
                            }
                        }
                    });
                }
                UserEvents::Left(user_id, guild_id) => {
                    if let Some(cancel) = tickers.lock().await.remove(&(user_id, guild_id)) {
                        let _ = cancel.send(());
                    }
                    if let Err(e) = Users::mark_left(&self.pool, user_id, guild_id).await {
                        eprintln!("[Left] mark_left failed: {e}");
                    }
                }
                UserEvents::SentText(user_id, nick, is_bot, guild_id, delta) => {
                    if let Err(e) =
                        Users::increment_score(&self.pool, user_id, guild_id, &nick, is_bot, delta)
                            .await
                    {
                        eprintln!("[SentText] increment_score failed: {e}");
                    }
                }
            }
        }
    }
}
