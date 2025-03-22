use sea_orm::EntityTrait;
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{Bot, BotResult, entities::game, error::BotError, utils::success_embed};

#[derive(Clone, Debug, SubCommand)]
pub struct CancelCommand {
    /// The scrim to cancel.
    #[command(autocomplete)]
    scrim: OffsetDateTime,
}

impl CancelCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let mut res = game::Entity::delete_by_id((guild.id, self.scrim))
            .exec_with_returning(&tx)
            .await?;

        let Some(game) = res.pop() else {
            return Err(BotError::GameNotFound);
        };

        let embed = game.embed(guild.serveme_api_key.as_ref()).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Scrim cancelled."), embed]),
            )
            .await?;

        Ok(())
    }
}

impl CancelCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let Self::Scrim { scrim } = self;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        guild.autocomplete_games(ctx, interaction, tx, &scrim).await
    }
}
