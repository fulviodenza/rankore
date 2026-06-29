use sqlx::{Pool, Postgres};
use tokio::time::{sleep, Duration};

const TICK: Duration = Duration::from_secs(60 * 60); // hourly

/// Spawn a background task that applies score decay for any guild whose
/// `decay_per_day_pct` is greater than zero. Decay is compounded per day —
/// missed days are caught up at the next tick.
pub fn spawn(pool: Pool<Postgres>) {
    tokio::spawn(async move {
        loop {
            if let Err(e) = run_once(&pool).await {
                eprintln!("[decay] tick failed: {e}");
            }
            sleep(TICK).await;
        }
    });
}

async fn run_once(pool: &Pool<Postgres>) -> Result<(), sqlx::Error> {
    let rows = sqlx::query!(
        "SELECT id, decay_per_day_pct, last_decay_day FROM guilds \
         WHERE decay_per_day_pct > 0"
    )
    .fetch_all(pool)
    .await?;
    let today = chrono::Utc::now().date_naive();
    for row in rows {
        let days = match row.last_decay_day {
            Some(d) => (today - d).num_days() as i32,
            None => 1,
        };
        if days <= 0 {
            continue;
        }
        let pct = row.decay_per_day_pct;
        sqlx::query!(
            "UPDATE users \
             SET score = GREATEST(0, FLOOR( \
                 score::double precision * power(1.0 - $1::double precision / 100.0, $2::int) \
             )::bigint) \
             WHERE guild_id = $3",
            pct as f64,
            days,
            row.id,
        )
        .execute(pool)
        .await?;
        sqlx::query!(
            "UPDATE guilds SET last_decay_day = $1 WHERE id = $2",
            today,
            row.id,
        )
        .execute(pool)
        .await?;
        tracing::info!(guild_id = row.id, pct, days, "applied score decay");
    }
    Ok(())
}
