mod changelevel;
mod delete;
mod rcon;
mod show;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{
    changelevel::ChangelevelCommand, delete::DeleteCommand, rcon::RconCommand, show::ShowCommand,
};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum GameCommand {
    /// Show the details of a game.
    #[command(autocomplete)]
    Show(ShowCommand),

    /// Delete a game from the schedule.
    #[command(autocomplete)]
    Delete(DeleteCommand),

    /// Run a command on the game server.
    #[command(autocomplete)]
    Rcon(RconCommand),

    /// Change the map of a game.
    #[command(autocomplete)]
    Changelevel(ChangelevelCommand),
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
            Self::Rcon(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Changelevel(cmd) => cmd.run(bot, ctx, interaction).await,
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
            Self::Rcon(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Changelevel(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
