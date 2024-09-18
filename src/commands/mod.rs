mod config;
mod scrim;

use serenity::all::{CommandInteraction, Context, Permissions};
use serenity_commands::Commands;
use tracing::instrument;

use self::{config::ConfigCommand, scrim::ScrimCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Commands)]
pub enum AllCommands {
    /// Configure the bot.
    #[command(builder(default_member_permissions(Permissions::MANAGE_GUILD)))]
    Config(ConfigCommand),

    /// Manage scrims.
    #[command(builder(default_member_permissions(Permissions::MANAGE_GUILD)))]
    Scrim(ScrimCommand),
}

impl AllCommands {
    #[instrument(skip(self), err)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Config(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Scrim(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}
