use sea_orm::EntityTrait;
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::game::{self, Game, ScrimOrMatch},
    error::BotError,
    utils::success_embed,
};

#[derive(Clone, Debug, SubCommand)]
pub struct DeleteCommand {
    /// The game to cancel.
    #[command(autocomplete)]
    game: OffsetDateTime,
}

impl DeleteCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let mut res = game::Entity::delete_by_id((guild.id, self.game))
            .exec_with_returning(&tx)
            .await?;

        let Some(game) = res.pop() else {
            return Err(BotError::GameNotFound);
        };

        let embed = Game::try_from(game)?
            .embed(guild.serveme_api_key.as_ref(), guild.rgl_team_id)
            .await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Game cancelled."), embed]),
            )
            .await?;

        Ok(())
    }
}

impl DeleteCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let Self::Game { game } = self;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        guild
            .autocomplete_games::<ScrimOrMatch>(ctx, interaction, tx, &game)
            .await
    }
}
