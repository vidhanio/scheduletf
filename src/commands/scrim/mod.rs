mod cancel;
mod edit;
mod host;
mod join;
mod refresh;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::{
    cancel::CancelCommand, edit::EditCommand, host::HostCommand, join::JoinCommand,
    refresh::RefreshCommand,
};
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum ScrimCommand {
    /// Host a new scrim.
    Host(HostCommand),

    /// Join a scrim hosted by another team.
    Join(JoinCommand),

    /// Edit an existing scrim.
    Edit(EditCommand),

    /// Cancel an existing scrim.
    Cancel(CancelCommand),

    /// Refresh all scrim messages and events.
    Refresh(RefreshCommand),
}

impl ScrimCommand {
    #[allow(clippy::too_many_lines)]
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
            Self::Cancel(cmd) => cmd.run(bot, ctx, interaction).await,
            Self::Refresh(cmd) => cmd.run(bot, ctx, interaction).await,
        }
    }
}
