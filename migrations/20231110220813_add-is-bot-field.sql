create table users(
    id bigint primary key,
    score bigint not null,
    nick varchar not null,
    is_bot boolean not null,
    guild_id bigint not null
);

create table guilds(
    id bigint primary key,
    prefix varchar not null,
    welcome_msg varchar
)
