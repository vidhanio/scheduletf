use serenity::all::{
    CommandInteraction, Context, EditInteractionResponse, ScheduledEventId, UserId,
};
use serenity_commands::SubCommand;
use sqlx::query_as;

use crate::{
    error::BotError,
    models::{DbScrim, GameFormat, Map, NextDay, Scrim, ServerInfo, Status, Time},
    utils::success_embed,
    Bot, BotResult,
};

#[derive(Clone, Debug, SubCommand)]
pub struct HostCommand {
    /// The next day of the week the scrim is scheduled for.
    day: NextDay,

    /// The time the scrim is scheduled for.
    time: Time,

    /// Opposing team's contact. Enter their user ID if they are not in the
    /// server.
    opponent: UserId,

    /// The game format of the scrim.
    game_format: GameFormat,

    /// The first map to be played.
    map_1: Option<Map>,

    /// The second map to be played.
    map_2: Option<Map>,

    /// An existing reservation to set up and modify. If not provided, a new
    /// reservation will be created.
    reservation_id: Option<u32>,
}

impl HostCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let timestamp = self.day.to_datetime(self.time);

        if let Some(scrim) = query_as!(
            DbScrim,
            "SELECT * FROM scrims
            WHERE guild_id = $1 AND timestamp = $2",
            i64::from(guild.id),
            timestamp,
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(Scrim::from)
        {
            return Err(BotError::ScrimAlreadyScheduled(scrim.timestamp));
        }

        let mut scrim = Scrim {
            guild_id: guild.id,
            timestamp,
            opponent_user_id: self.opponent,
            game_format: self.game_format,
            map_1: self.map_1,
            map_2: self.map_2,
            server_info: None,
            event_id: ScheduledEventId::default(),
            message_id: None,
            status: Status::Waiting,
        };

        if let Some(serveme_api_key) = guild.serveme_api_key {
            let reservation = if let Some(reservation_id) = self.reservation_id {
                bot.edit_serveme_reservation(&serveme_api_key, &scrim, reservation_id)
                    .await?
            } else {
                bot.new_serveme_reservation(&serveme_api_key, &scrim)
                    .await?
            };

            let connect_info = reservation.connect_info();

            scrim.server_info = Some(ServerInfo::Serveme {
                reservation_id: reservation.id,
                rcon_password: reservation.rcon,
                connect_info,
            });
        } else {
            return Err(BotError::NoServemeApiKey);
        }

        scrim.event_id = guild
            .id
            .create_scheduled_event(ctx, scrim.create_event())
            .await?
            .id;

        if let Some(games_channel) = guild.games_channel_id {
            let message_id = games_channel.send_message(ctx, scrim.message()).await?.id;

            scrim.message_id = Some(message_id);
        }

        let embed = scrim.embed();

        DbScrim::from(scrim).insert(&mut *tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Scrim scheduled."), embed]),
            )
            .await?;

        Ok(())
    }
}
