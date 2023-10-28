CREATE TABLE IF NOT EXISTS users(
    id bigint PRIMARY KEY NOT NULL, 
    score bigint NOT NULL, 
    nick varchar(20) NOT NULL
);
