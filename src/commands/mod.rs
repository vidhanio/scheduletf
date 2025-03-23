mod config;
mod game;
mod r#match;
mod refresh;
mod scrim;

use serenity::all::{
    CommandInteraction, Context, InstallationContext, InteractionContext, Permissions,
    ResolvedTarget,
};
use serenity_commands::Commands;
use tracing::instrument;

use self::{
    config::ConfigCommand, game::GameCommand, r#match::MatchCommand, refresh::RefreshCommand,
    scrim::ScrimCommand,
};
use crate::{Bot, BotResult, error::BotError, rgl::RglProfile};

#[derive(Debug, Commands)]
pub enum AllCommands {
    /// Configure the bot.
    #[command(builder(default_member_permissions(Permissions::MANAGE_GUILD)))]
    Config(ConfigCommand),

    /// Manage scrims.
    #[command(
        autocomplete,
        builder(default_member_permissions(Permissions::MANAGE_GUILD))
    )]
    Scrim(ScrimCommand),

    /// Manage matches.
    #[command(
        autocomplete,
        builder(default_member_permissions(Permissions::MANAGE_GUILD))
    )]
    Match(MatchCommand),

    /// Manage games.
    #[command(
        autocomplete,
        builder(default_member_permissions(Permissions::MANAGE_GUILD))
    )]
    Game(GameCommand),

    /// Refresh the schedule.
    Refresh(RefreshCommand),

    #[command(name = "RGL.gg Profile", context_menu = "user")]
    #[command(builder(
        add_integration_type(InstallationContext::User),
        contexts(vec![
            InteractionContext::Guild,
            InteractionContext::PrivateChannel,
        ])
    ))]
    RglProfile,
}

impl AllCommands {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Config(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Scrim(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Match(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Game(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Refresh(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::RglProfile => {
                let ResolvedTarget::User(user, _) = interaction
                    .data
                    .target()
                    .ok_or(BotError::InvalidInteractionTarget)?
                else {
                    return Err(BotError::InvalidInteractionTarget);
                };

                interaction.defer_ephemeral(ctx).await?;

                interaction
                    .edit_response(ctx, RglProfile::get_from_discord(user.id).await?.response())
                    .await?;

                Ok(())
            }
        }
    }
}

impl AllCommandsAutocomplete {
    #[instrument(skip(self), err)]
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Scrim(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Match(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Game(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
