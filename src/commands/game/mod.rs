mod delete;
mod show;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{delete::DeleteCommand, show::ShowCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum GameCommand {
    /// Show the details of a game.
    #[command(autocomplete)]
    Show(ShowCommand),

    /// Delete a game from the schedule.
    #[command(autocomplete)]
    Delete(DeleteCommand),
}

impl GameCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Show(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Delete(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}

impl GameCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Show(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Delete(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
