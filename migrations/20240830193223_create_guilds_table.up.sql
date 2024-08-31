CREATE TABLE guilds(
    id bigint NOT NULL PRIMARY KEY,
    voice_channel bigint,
    schedule_channel bigint,
    logs_channel bigint,
    serveme_api_key varchar(32)
);

