CREATE TABLE IF NOT EXISTS users(
    id bigint PRIMARY KEY NOT NULL, 
    score bigint NOT NULL, 
    nick varchar(20) NOT NULL
);
CREATE TABLE IF NOT EXISTS guilds(
    id bigint PRIMARY KEY NOT NULL,
    prefix varchar(1) NOT NULL,
    welcome_msg varchar(50) NOT NULL
);
