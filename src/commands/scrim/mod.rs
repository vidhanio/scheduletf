mod edit;
mod host;
mod join;
mod lfs;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{edit::EditCommand, host::HostCommand, join::JoinCommand, lfs::LfsCommand};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum ScrimCommand {
    /// Host a new scrim.
    #[command(autocomplete)]
    Host(HostCommand),

    /// Join a scrim hosted by another team.
    #[command(autocomplete)]
    Join(JoinCommand),

    /// Edit an existing scrim.
    #[command(autocomplete)]
    Edit(EditCommand),

    /// Generate Looking for Scrim messages.
    Lfs(LfsCommand),
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
            Self::Edit(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Lfs(cmd) => cmd.run(bot, ctx, interaction).await,
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
            Self::Edit(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}
