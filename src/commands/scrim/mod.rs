mod cancel;
mod edit;
mod host;
mod join;
mod show;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{
    cancel::CancelCommand, edit::EditCommand, host::HostCommand, join::JoinCommand,
    show::ShowCommand,
};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum ScrimCommand {
    /// Host a new scrim.
    #[command(autocomplete)]
    Host(HostCommand),

    /// Join a scrim hosted by another team.
    #[command(autocomplete)]
    Join(JoinCommand),

    /// Show the details of a scrim.
    #[command(autocomplete)]
    Show(ShowCommand),

    /// Edit an existing scrim.
    #[command(autocomplete)]
    Edit(EditCommand),

    /// Cancel an existing scrim.
    #[command(autocomplete)]
    Cancel(CancelCommand),
}

impl ScrimCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Host(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Join(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Show(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Edit(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Cancel(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}

impl ScrimCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Host(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Join(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Show(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Edit(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Cancel(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
