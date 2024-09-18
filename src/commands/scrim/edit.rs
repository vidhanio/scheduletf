use serenity::all::{CommandInteraction, Context, CreateInteractionResponse, UserId};
use serenity_commands::SubCommandGroup;
use sqlx::{query, query_as};
use time::OffsetDateTime;

use crate::{
    error::BotError,
    models::{DbScrim, GameFormat, Map, NextDay, Scrim, ServerInfo, Time},
    serveme::ConnectInfo,
    utils::success_message,
    Bot, BotResult,
};

#[derive(Clone, Debug, SubCommandGroup)]
pub enum EditCommand {
    /// Edit the date and time of the scrim.
    DateTime {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// The new day the scrim is scheduled for.
        new_day: NextDay,

        /// The new time the scrim is scheduled for.
        new_time: Time,
    },

    /// Edit the opposing team's contact.
    Opponent {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// The opposing team's contact. Enter their user ID if they are not in
        /// the server.
        opponent: UserId,
    },

    /// Edit the game format of the scrim.
    GameFormat {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// The game format of the scrim.
        game_format: GameFormat,
    },

    /// Edit the first map to be played.
    #[command(name = "map-1")]
    Map1 {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// The first map to be played.
        map: Option<Map>,
    },

    /// Edit the second map to be played.
    #[command(name = "map-2")]
    Map2 {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// The second map to be played.
        map: Option<Map>,
    },

    /// Edit the existing reservation to set up and modify.
    ReservationId {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// An existing reservation to set up and modify. If not provided, a new
        /// reservation will be created.
        reservation_id: Option<u32>,
    },

    /// Edit the external connect info, if they host.
    ConnectInfo {
        /// The day of the week the scrim is scheduled for.
        day: NextDay,

        /// The time the scrim is scheduled for.
        time: Time,

        /// External connect info, if they host. If originally hosted, this will
        /// cancel the reservation.
        connect_info: Option<ConnectInfo>,
    },
}

impl EditCommand {
    fn timestamp(&self) -> OffsetDateTime {
        let (day, time) = match self {
            Self::DateTime { day, time, .. }
            | Self::Opponent { day, time, .. }
            | Self::GameFormat { day, time, .. }
            | Self::Map1 { day, time, .. }
            | Self::Map2 { day, time, .. }
            | Self::ReservationId { day, time, .. }
            | Self::ConnectInfo { day, time, .. } => (day, time),
        };

        day.to_datetime(*time)
    }
}

impl EditCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let timestamp = self.timestamp();

        let mut edited_reservation = false;

        let scrim = match self {
            Self::DateTime {
                new_day, new_time, ..
            } => {
                edited_reservation = true;

                let new_timestamp = new_day.to_datetime(new_time);
                query_as!(
                    DbScrim,
                    "UPDATE scrims SET timestamp = $1
                    WHERE guild_id = $2 AND timestamp = $3
                    RETURNING *",
                    new_timestamp,
                    i64::from(guild.id),
                    timestamp,
                )
                .fetch_one(&mut *tx)
                .await?
                .into()
            }

            Self::Opponent { opponent, .. } => query_as!(
                DbScrim,
                "UPDATE scrims SET opponent_user_id = $1
                    WHERE guild_id = $2 AND timestamp = $3
                    RETURNING *",
                i64::from(opponent),
                i64::from(guild.id),
                timestamp,
            )
            .fetch_one(&mut *tx)
            .await?
            .into(),

            Self::GameFormat { game_format, .. } => {
                edited_reservation = true;

                query_as!(
                    DbScrim,
                    "UPDATE scrims SET game_format = $1
                    WHERE guild_id = $2 AND timestamp = $3
                    RETURNING *",
                    i16::from(game_format),
                    i64::from(guild.id),
                    timestamp,
                )
                .fetch_one(&mut *tx)
                .await?
                .into()
            }

            Self::Map1 { map, .. } => {
                edited_reservation = true;

                query_as!(
                    DbScrim,
                    "UPDATE scrims SET map_1 = $1
                    WHERE guild_id = $2 AND timestamp = $3
                    RETURNING *",
                    map.as_ref().map(Map::as_str),
                    i64::from(guild.id),
                    timestamp,
                )
                .fetch_one(&mut *tx)
                .await?
                .into()
            }
            Self::Map2 { map, .. } => query_as!(
                DbScrim,
                "UPDATE scrims SET map_2 = $1
                    WHERE guild_id = $2 AND timestamp = $3
                    RETURNING *",
                map.as_ref().map(Map::as_str),
                i64::from(guild.id),
                timestamp,
            )
            .fetch_one(&mut *tx)
            .await?
            .into(),
            Self::ReservationId { reservation_id, .. } => {
                let mut scrim = Scrim::from(
                    query_as!(
                        DbScrim,
                        "SELECT * FROM scrims
                        WHERE guild_id = $1 AND timestamp = $2",
                        i64::from(guild.id),
                        timestamp,
                    )
                    .fetch_one(&mut *tx)
                    .await?,
                );

                if let Some(serveme_api_key) = &guild.serveme_api_key {
                    let reservation = if let Some(reservation_id) = reservation_id {
                        bot.edit_serveme_reservation(serveme_api_key, &scrim, reservation_id)
                            .await?
                    } else {
                        bot.new_serveme_reservation(serveme_api_key, &scrim).await?
                    };

                    let connect_info = reservation.connect_info();

                    query!(
                        "UPDATE scrims SET reservation_id = $1, serveme_rcon = $2, ip_and_port = $3, password = $4
                        WHERE guild_id = $5 AND timestamp = $6",
                        reservation.id as i32,
                        &reservation.rcon,
                        &connect_info.ip_and_port,
                        &connect_info.password,
                        i64::from(guild.id),
                        timestamp,
                    )
                    .execute(&mut *tx)
                    .await?;

                    scrim.server_info = Some(ServerInfo::Serveme {
                        reservation_id: reservation.id,
                        rcon_password: reservation.rcon,
                        connect_info,
                    });

                    scrim
                } else {
                    return Err(BotError::NoServemeApiKey);
                }
            }
            Self::ConnectInfo { connect_info, .. } => {
                let scrim = Scrim::from(
                    query_as!(
                        DbScrim,
                        "UPDATE scrims SET reservation_id = NULL, serveme_rcon = NULL, ip_and_port = $1, password = $2
                        WHERE guild_id = $3 AND timestamp = $4
                        RETURNING *",
                        connect_info.as_ref().map(|info| &info.ip_and_port),
                        connect_info.as_ref().map(|info| &info.password),
                        i64::from(guild.id),
                        timestamp,
                    )
                    .fetch_one(&mut *tx)
                    .await?
                );

                if let Some((serveme_api_key, reservation_id)) =
                    guild.serveme_api_key.as_ref().zip(scrim.reservation_id())
                {
                    bot.delete_serveme_reservation(serveme_api_key, reservation_id)
                        .await?;
                }

                scrim
            }
        };

        if edited_reservation {
            if let Some((serveme_api_key, reservation_id)) =
                guild.serveme_api_key.as_ref().zip(scrim.reservation_id())
            {
                bot.edit_serveme_reservation(serveme_api_key, &scrim, reservation_id)
                    .await?;
            }
        }

        guild
            .id
            .edit_scheduled_event(ctx, scrim.event_id, scrim.edit_event())
            .await?;

        if let Some((games_channel, message_id)) = guild.games_channel_id.zip(scrim.message_id) {
            games_channel
                .edit_message(&ctx, message_id, scrim.edit_message())
                .await?;
        }

        tx.commit().await?;

        interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(
                    success_message("Scrim edited.").add_embed(scrim.embed()),
                ),
            )
            .await?;

        Ok(())
    }
}
