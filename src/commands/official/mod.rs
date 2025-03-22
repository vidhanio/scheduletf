mod add;
mod delete;
mod edit;
mod host;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{add::AddCommand, delete::DeleteCommand, edit::EditCommand, show::ShowCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum OfficialCommand {
    /// Add a new official match to the schedule.
    #[command(autocomplete)]
    Add(AddCommand),

    /// Show the details of an official match.
    #[command(autocomplete)]
    Show(ShowCommand),

    /// Edit an existing scrim.
    #[command(autocomplete)]
    Edit(EditCommand),

    /// Delete an official match from the schedule.
    #[command(autocomplete)]
    Delete(DeleteCommand),
}

impl OfficialCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Add(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Show(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Edit(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Delete(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}

impl OfficialCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Add(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Show(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Edit(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Delete(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
