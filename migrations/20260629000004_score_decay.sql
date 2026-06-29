ALTER TABLE guilds
    ADD COLUMN decay_per_day_pct INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN last_decay_day DATE;
