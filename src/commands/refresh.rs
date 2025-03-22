use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::Command;

use crate::{Bot, BotResult, utils::success_embed};

#[derive(Clone, Debug, Command)]
pub struct RefreshCommand;

impl RefreshCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().embed(success_embed("Schedule refreshed.")),
            )
            .await?;

        Ok(())
    }
}
