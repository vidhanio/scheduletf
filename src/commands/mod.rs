mod config;
mod scrim;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Commands;

use self::{config::ConfigCommand, scrim::ScrimCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Commands)]
pub enum AllCommands {
    /// Configure the bot.
    Config(ConfigCommand),

    /// Manage scrimmages.
    Scrim(ScrimCommand),
}

impl AllCommands {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Config(args) => args.run(bot, ctx, interaction).await,
            Self::Scrim(args) => args.run(bot, ctx, interaction).await,
        }
    }
}
