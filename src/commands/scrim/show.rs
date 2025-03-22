use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{Bot, BotResult};

#[derive(Clone, Debug, SubCommand)]
pub struct ShowCommand {
    /// The scrim to get details of.
    #[command(autocomplete)]
    scrim: OffsetDateTime,
}

impl ShowCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let embed = guild
            .get_game(&tx, self.scrim)
            .await?
            .embed(guild.serveme_api_key.as_ref())
            .await?;

        tx.commit().await?;

        interaction
            .edit_response(&ctx, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}

impl ShowCommandAutocomplete {
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
