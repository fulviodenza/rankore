CREATE TABLE channel_multipliers (
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    text_multiplier BIGINT,
    voice_multiplier BIGINT,
    PRIMARY KEY (guild_id, channel_id)
);
