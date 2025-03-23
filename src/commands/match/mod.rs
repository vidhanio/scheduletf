mod edit;
mod host;
mod join;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{edit::EditCommand, host::HostCommand, join::JoinCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum MatchCommand {
    /// Add a hosted match to the schedule.
    Host(HostCommand),

    /// Add a joined match to the schedule.
    Join(JoinCommand),

    /// Edit an existing match.
    #[command(autocomplete)]
    Edit(EditCommand),
}

impl MatchCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Host(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Join(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Edit(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}

impl MatchCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Edit(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
