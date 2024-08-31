CREATE TABLE scrims(
    guild_id bigint NOT NULL REFERENCES guilds,
    format smallint NOT NULL,
    timestamp timestamp with time zone NOT NULL,
    hosted boolean NOT NULL,
    map_1 text NOT NULL,
    map_2 text NOT NULL,
    opponent text NOT NULL,
    registration int,
    event_id bigint,
    CONSTRAINT scrims_pkey PRIMARY KEY (guild_id, timestamp)
);

