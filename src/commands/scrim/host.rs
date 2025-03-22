use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        game::{self, Maps, ReservationId},
        team_guild::GameFormat,
    },
    error::BotError,
    utils::success_embed,
};

#[derive(Clone, Debug, SubCommand)]
pub struct HostCommand {
    /// The date/time to schedule the scrim for.
    #[command(autocomplete)]
    date_time: OffsetDateTime,

    /// Opposing team's contacted team member. Enter their user ID if they are
    /// not in the server.
    opponent: UserId,

    /// Comma-separated list of maps to be played.
    #[command(autocomplete)]
    maps: Option<Maps>,

    /// The game format of the scrim. Defaults to the guild's default game
    /// format.
    game_format: Option<GameFormat>,

    /// An existing reservation to set up and modify. If not provided, a new
    /// reservation will be created.
    #[command(autocomplete)]
    reservation_id: Option<ReservationId>,
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

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        guild.ensure_time_open(&tx, self.date_time).await?;

        let mut game = game::Model {
            guild_id: guild.id,
            timestamp: self.date_time,
            game_format: self
                .game_format
                .or(guild.game_format)
                .ok_or(BotError::NoGameFormat)?,
            opponent_user_id: self.opponent.into(),
            reservation_id: self.reservation_id,
            server_ip_and_port: None,
            server_password: None,
            maps: Some(self.maps.unwrap_or_default()),
            rgl_match_id: None,
        };

        let serveme_api_key = guild
            .serveme_api_key
            .as_ref()
            .ok_or(BotError::NoServemeApiKey)?;

        if self.reservation_id.is_some() {
            game.edit_reservation(serveme_api_key).await?;
        } else {
            let reservation = game.create_reservation(serveme_api_key).await?;

            game.reservation_id = Some(reservation.id);
        }

        let game = game.into_active_model().reset_all().insert(&tx).await?;

        let embed = game.embed(Some(serveme_api_key)).await?;

        guild.refresh_schedule(ctx, &tx).await?;

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

impl HostCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::DateTime { date_time, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_times(ctx, interaction, tx, &date_time)
                    .await
            }
            Self::Maps {
                maps, game_format, ..
            } => {
                let guild = bot.get_guild(interaction.guild_id).await?;

                guild
                    .autocomplete_maps(ctx, interaction, game_format.flatten().into_value(), &maps)
                    .await
            }
            Self::ReservationId { reservation_id, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_reservations(ctx, interaction, tx, &reservation_id)
                    .await
            }
        }
    }
}
