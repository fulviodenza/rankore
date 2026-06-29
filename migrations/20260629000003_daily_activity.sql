CREATE TABLE daily_activity (
    user_id BIGINT NOT NULL,
    guild_id BIGINT NOT NULL,
    day DATE NOT NULL,
    PRIMARY KEY (user_id, guild_id, day)
);
CREATE INDEX daily_activity_guild_day_idx
    ON daily_activity (guild_id, day DESC);
