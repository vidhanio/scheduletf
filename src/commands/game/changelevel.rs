use sea_orm::{ColumnTrait, ModelTrait, QueryFilter, QueryOrder};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        Map,
        game::{self, GameDetails, ScrimOrMatch},
    },
    error::BotError,
    serveme::EditReservationRequest,
    utils::{OffsetDateTimeEtExt, success_embed},
};

#[derive(Clone, Debug, SubCommand)]
pub struct ChangelevelCommand {
    /// The map to go to.
    #[command(autocomplete)]
    map: Map,

    /// The game to change the map of. If not provided, the most recent game
    /// will be used.
    #[command(autocomplete)]
    game: Option<OffsetDateTime>,
}

impl ChangelevelCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let game = if let Some(game) = self.game {
            guild.get_game::<ScrimOrMatch>(&tx, game).await?
        } else {
            guild
                .find_related(game::Entity)
                .filter(game::Column::Timestamp.lte(OffsetDateTime::now_et()))
                .order_by_desc(game::Column::Timestamp)
                .into_partial_model()
                .one(&tx)
                .await?
                .ok_or(BotError::GameNotFound)?
        };

        let reservation_id = game.server.reservation_id()?;

        let server_config_id = self
            .map
            .server_config(game.details.kind(), game.details.game_format().await?)
            .map(|c| c.id);

        EditReservationRequest {
            first_map: Some(self.map),
            server_config_id,
            ..Default::default()
        }
        .send(guild.serveme_api_key()?, reservation_id)
        .await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().embed(success_embed("Map changed.")),
            )
            .await?;

        Ok(())
    }
}

impl ChangelevelCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Map { map, game } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                let game = if let Some(game) = game.into_value().flatten() {
                    guild.get_game::<ScrimOrMatch>(&tx, game).await?
                } else {
                    guild
                        .find_related(game::Entity)
                        .filter(game::Column::Timestamp.lte(OffsetDateTime::now_et()))
                        .order_by_desc(game::Column::Timestamp)
                        .into_partial_model()
                        .one(&tx)
                        .await?
                        .ok_or(BotError::GameNotFound)?
                };

                game.autocomplete_maps(ctx, interaction, guild.serveme_api_key()?, &map)
                    .await
            }
            Self::Game { game, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_games::<ScrimOrMatch>(ctx, interaction, tx, &game)
                    .await
            }
        }
    }
}
