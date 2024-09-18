CREATE TABLE games(
    guild_id bigint NOT NULL REFERENCES guilds,
    timestamp timestamp with time zone NOT NULL,
    event_id bigint NOT NULL UNIQUE,
    message_id bigint UNIQUE,
    opponent_user_id bigint NOT NULL,
    reservation_id int,
    server_ip_and_port text,
    server_password text,
    map_1 text,
    map_2 text,
    rgl_match_id int,
    PRIMARY KEY (guild_id, timestamp),
    CHECK ((reservation_id IS NOT NULL AND server_ip_and_port IS NULL AND server_password IS NULL) -- hosted
    OR (reservation_id IS NULL AND server_ip_and_port IS NOT NULL AND server_password IS NOT NULL) -- joined
    OR (reservation_id IS NULL AND server_ip_and_port IS NULL AND server_password IS NULL)), -- undecided
    CHECK ((map_1 IS NULL AND map_2 IS NULL AND rgl_match_id IS NOT NULL) -- official
    OR (rgl_match_id IS NULL)) -- scrim
);

