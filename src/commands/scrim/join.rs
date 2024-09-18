use serenity::all::{
    CommandInteraction, Context, EditInteractionResponse, ScheduledEventId, UserId,
};
use serenity_commands::SubCommand;
use sqlx::query_as;

use crate::{
    error::BotError,
    models::{DbScrim, GameFormat, Map, NextDay, Scrim, ServerInfo, Status, Time},
    serveme::ConnectInfo,
    utils::success_embed,
    Bot, BotResult,
};

#[derive(Clone, Debug, SubCommand)]
pub struct JoinCommand {
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

    /// The connect info for the other team's server.
    connect_info: Option<ConnectInfo>,
}

impl JoinCommand {
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
            server_info: self.connect_info.map(ServerInfo::ExternalServer),
            event_id: ScheduledEventId::default(),
            message_id: None,
            status: Status::Waiting,
        };

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
