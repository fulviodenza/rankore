CREATE TABLE score_events (
    id BIGSERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    delta BIGINT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX score_events_guild_time_idx
    ON score_events (guild_id, occurred_at DESC);
CREATE INDEX score_events_guild_user_time_idx
    ON score_events (guild_id, user_id, occurred_at DESC);
