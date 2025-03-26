use serenity::all::{
    ButtonStyle, ComponentInteraction, ComponentInteractionData, Context, CreateButton,
    EditInteractionResponse,
};

use crate::{Bot, BotResult, error::BotError, utils::success_embed};

#[derive(Debug, Clone)]
pub enum AllComponents {
    Refresh(RefreshButton),
}

impl AllComponents {
    pub fn from_component_data(data: &ComponentInteractionData) -> BotResult<Self> {
        match data.custom_id.as_str() {
            RefreshButton::CUSTOM_ID => Ok(Self::Refresh(RefreshButton)),
            _ => Err(BotError::InvalidComponentInteraction),
        }
    }

    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &ComponentInteraction,
    ) -> BotResult {
        match self {
            Self::Refresh(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshButton;

impl RefreshButton {
    const CUSTOM_ID: &'static str = "refresh";

    pub fn create() -> CreateButton {
        CreateButton::new(Self::CUSTOM_ID)
            .label("Refresh")
            .style(ButtonStyle::Secondary)
    }

    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &ComponentInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        interaction
            .edit_response(
                ctx,
                EditInteractionResponse::new().embed(success_embed("Schedule refreshed.")),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
