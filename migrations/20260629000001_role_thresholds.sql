CREATE TABLE role_thresholds (
    guild_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    score BIGINT NOT NULL,
    PRIMARY KEY (guild_id, role_id)
);
CREATE INDEX role_thresholds_guild_score_idx ON role_thresholds (guild_id, score);
