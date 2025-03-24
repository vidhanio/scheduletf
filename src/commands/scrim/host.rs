use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        GameFormat, MapList, ReservationId,
        game::{Game, GameServer, Scrim},
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

    /// Space-separated list of maps to be played.
    #[command(autocomplete)]
    maps: Option<MapList>,

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

        let mut game = Game {
            guild_id: guild.id,
            timestamp: self.date_time,
            server: self
                .reservation_id
                .map(GameServer::Hosted)
                .unwrap_or_default(),
            details: Scrim {
                opponent_user_id: self.opponent.into(),
                game_format: self
                    .game_format
                    .or(guild.game_format)
                    .ok_or(BotError::NoGameFormat)?,
                maps: self.maps.unwrap_or_default(),
            },
        };

        let serveme_api_key = guild.serveme_api_key()?;

        if game.server.is_hosted() {
            game.edit_reservation(serveme_api_key).await?;
        } else {
            game.create_reservation(serveme_api_key).await?;
        }

        let game = Game::try_from(game.into_active_model().insert(&tx).await?)?;

        let embed = game.embed(&guild).await?;

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
