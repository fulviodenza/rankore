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
use crate::services::roles::RoleSyncer;

pub struct Users {
    pub pool: Pool<Postgres>,
    pub tx: UnboundedSender<UserEvents>,
    pub role_syncer: Arc<RoleSyncer>,
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
    async fn new(pool: &Pool<Postgres>, role_syncer: Arc<RoleSyncer>) -> Arc<Self>;
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
    async fn get_period_leaderboard(
        &self,
        guild_id: i64,
        since: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<User>, Error>;
    async fn reset_scores(&self, guild_id: i64) -> Result<(), Error>;
    async fn get_streak(&self, user_id: i64, guild_id: i64) -> Result<i64, Error>;
    async fn get_user_stats(
        &self,
        user_id: i64,
        guild_id: i64,
    ) -> Result<Option<UserStats>, Error>;
}

#[derive(Debug)]
pub struct UserStats {
    pub score: i64,
    pub rank: i64,
    pub last_active: Option<chrono::DateTime<chrono::Utc>>,
    pub event_count: i64,
}

#[async_trait]
impl UsersRepo for Users {
    async fn new(pool: &Pool<Postgres>, role_syncer: Arc<RoleSyncer>) -> Arc<Self> {
        let (tx, rx) = mpsc::unbounded_channel::<UserEvents>();
        let users = Arc::new(Users {
            tx,
            pool: pool.clone(),
            role_syncer,
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
        let mut tx = pool.begin().await?;
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
        .execute(&mut *tx)
        .await?;
        if !is_bot && delta > 0 {
            sqlx::query!(
                "INSERT INTO score_events (user_id, guild_id, delta) \
                 VALUES ($1, $2, $3)",
                id,
                guild_id,
                delta,
            )
            .execute(&mut *tx)
            .await?;
            sqlx::query!(
                "INSERT INTO daily_activity (user_id, guild_id, day) \
                 VALUES ($1, $2, (NOW() AT TIME ZONE 'UTC')::date) \
                 ON CONFLICT DO NOTHING",
                id,
                guild_id,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await
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

    async fn get_period_leaderboard(
        &self,
        guild_id: i64,
        since: chrono::DateTime<chrono::Utc>,
        limit: i64,
    ) -> Result<Vec<User>, Error> {
        let rows = sqlx::query!(
            "SELECT u.id, u.nick, u.is_bot, u.guild_id, u.hasleft, \
                    COALESCE(SUM(e.delta), 0)::BIGINT AS \"period_score!\" \
             FROM users u \
             JOIN score_events e \
               ON e.user_id = u.id AND e.guild_id = u.guild_id \
             WHERE u.guild_id = $1 AND u.is_bot = false AND e.occurred_at >= $2 \
             GROUP BY u.id, u.guild_id, u.nick, u.is_bot, u.hasleft \
             ORDER BY \"period_score!\" DESC \
             LIMIT $3",
            guild_id,
            since,
            limit,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| User {
                id: row.id,
                score: row.period_score,
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

    async fn get_user_stats(
        &self,
        user_id: i64,
        guild_id: i64,
    ) -> Result<Option<UserStats>, Error> {
        let row = sqlx::query!(
            "WITH ranked AS ( \
               SELECT id, score, \
                      ROW_NUMBER() OVER (ORDER BY score DESC, id ASC) AS rnk \
               FROM users \
               WHERE guild_id = $1 AND is_bot = false \
             ), \
             events AS ( \
               SELECT MAX(occurred_at) AS last_active, COUNT(*) AS event_count \
               FROM score_events WHERE guild_id = $1 AND user_id = $2 \
             ) \
             SELECT r.score, r.rnk AS \"rnk!\", \
                    (SELECT last_active FROM events) AS last_active, \
                    (SELECT event_count FROM events) AS event_count \
             FROM ranked r \
             WHERE r.id = $2",
            guild_id,
            user_id,
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| UserStats {
            score: r.score,
            rank: r.rnk,
            last_active: r.last_active,
            event_count: r.event_count.unwrap_or(0),
        }))
    }

    async fn get_streak(&self, user_id: i64, guild_id: i64) -> Result<i64, Error> {
        let days: Vec<chrono::NaiveDate> = sqlx::query_scalar!(
            "SELECT day FROM daily_activity \
             WHERE user_id = $1 AND guild_id = $2 \
             ORDER BY day DESC \
             LIMIT 365",
            user_id,
            guild_id,
        )
        .fetch_all(&self.pool)
        .await?;
        if days.is_empty() {
            return Ok(0);
        }
        let today = chrono::Utc::now().date_naive();
        let yesterday = today - chrono::Duration::days(1);
        let mut expected = if days[0] == today {
            today
        } else if days[0] == yesterday {
            yesterday
        } else {
            return Ok(0);
        };
        let mut streak: i64 = 0;
        for d in days {
            if d == expected {
                streak += 1;
                expected -= chrono::Duration::days(1);
            } else if d < expected {
                break;
            }
        }
        Ok(streak)
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
                    let role_syncer = self.role_syncer.clone();
                    tokio::spawn(async move {
                        loop {
                            select! {
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(interval)) => {
                                    match Users::increment_score(
                                        &pool, user_id, guild_id, &nick, is_bot, 1,
                                    ).await {
                                        Ok(()) => role_syncer.sync(user_id, guild_id).await,
                                        Err(e) => eprintln!("[voice tick] increment_score failed: {e}"),
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
                    match Users::increment_score(
                        &self.pool, user_id, guild_id, &nick, is_bot, delta,
                    )
                    .await
                    {
                        Ok(()) => self.role_syncer.sync(user_id, guild_id).await,
                        Err(e) => eprintln!("[SentText] increment_score failed: {e}"),
                    }
                }
            }
        }
    }
}
