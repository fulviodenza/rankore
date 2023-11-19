create table users(
    id bigint,
    score bigint not null,
    nick varchar not null,
    is_bot boolean not null,
    guild_id bigint,
    primary key (id, guild_id)
);

create table guilds(
    id bigint primary key,
    prefix varchar not null,
    welcome_msg varchar,
    voice_multiplier bigint not null default 1,
    text_multiplier bigint not null default 1
)
