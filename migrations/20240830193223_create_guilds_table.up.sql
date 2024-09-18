CREATE TABLE guilds(
    id bigint NOT NULL PRIMARY KEY,
    rgl_team_id int UNIQUE,
    game_format smallint CHECK (game_format = 6 OR game_format = 9),
    games_channel_id bigint,
    serveme_api_key varchar(32)
);

