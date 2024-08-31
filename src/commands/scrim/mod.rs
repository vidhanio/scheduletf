mod schedule;

use serenity::all::{CommandInteraction, Context};
use serenity_commands::Command;

use self::schedule::ScheduleCommand;
use crate::{Bot, BotResult};

#[derive(Debug, Command)]
pub enum ScrimCommand {
    /// Schedule a scrimmage.
    Schedule(ScheduleCommand),
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
            Self::Schedule(args) => args.run(bot, ctx, interaction).await,
        }
    }
}
