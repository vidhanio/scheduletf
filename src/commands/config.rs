use sea_orm::{ActiveModelTrait, ActiveValue::Set, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, CreateInteractionResponse};
use serenity_commands::{Command, SubCommandGroup};

use crate::{
    Bot, BotResult,
    entities::{GameFormat, ScheduleChannelId, ServemeApiKey},
    rgl::{RglSeason, RglTeam, RglTeamId},
    utils::{create_message, success_embed},
};

#[derive(Debug, Command)]
pub enum ConfigCommand {
    /// Show the current configuration.
    Show,

    /// Set a configuration option.
    Set(ConfigSetCommand),

    /// Unset a configuration option.
    Unset(ConfigUnsetCommand),
}

macro_rules! config_commands {
    (
        $(
            $doc:literal
            $name:ident { $field:ident : $field_ty:ty },
        )*
    ) => {
        #[derive(Debug, SubCommandGroup)]
        pub enum ConfigSetCommand {
            $(
                #[doc = concat!("Set the ", $doc, ".")]
                $name {
                    #[doc = concat!("The ", stringify!($doc), ".")]
                    $field: $field_ty,
                },
            )*
        }

        #[derive(Debug, SubCommandGroup)]
        pub enum ConfigUnsetCommand {
            $(
                #[doc = concat!("Unset the ", $doc, ".")]
                $name,
            )*
        }
    };
}

config_commands! {
    "na.serveme.tf API key"
    Serveme { key: ServemeApiKey },

    "default game format"
    GameFormat { format: GameFormat },

    "schedule channel"
    ScheduleChannel { channel: ScheduleChannelId },

    "RGL team ID"
    RglTeam { team_id: RglTeamId },
}

impl ConfigCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let active_guild = match self {
            Self::Show => {
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            create_message().embed(guild.config_embed()),
                        ),
                    )
                    .await?;

                return Ok(());
            }
            Self::Set(cmd) => {
                let mut guild = guild.into_active_model();

                match cmd {
                    ConfigSetCommand::Serveme { key } => {
                        guild.serveme_api_key.set_if_not_equals(Some(key));
                    }
                    ConfigSetCommand::GameFormat { format } => {
                        guild.game_format.set_if_not_equals(Some(format));
                    }
                    ConfigSetCommand::ScheduleChannel { channel } => {
                        guild.schedule_channel_id.set_if_not_equals(Some(channel));
                    }
                    ConfigSetCommand::RglTeam { team_id } => {
                        guild.rgl_team_id.set_if_not_equals(Some(team_id));

                        let team = RglTeam::get(team_id).await?;

                        let season = RglSeason::get(team.season_id).await?;

                        guild
                            .game_format
                            .set_if_not_equals(Some(season.format_name));
                    }
                }

                guild
            }
            Self::Unset(cmd) => {
                let mut guild = guild.into_active_model();

                match cmd {
                    ConfigUnsetCommand::Serveme => {
                        guild.serveme_api_key = Set(None);
                    }
                    ConfigUnsetCommand::GameFormat => {
                        guild.game_format.set_if_not_equals(None);
                    }
                    ConfigUnsetCommand::ScheduleChannel => {
                        guild.schedule_channel_id.set_if_not_equals(None);
                        guild.schedule_message_id.set_if_not_equals(None);
                    }
                    ConfigUnsetCommand::RglTeam => {
                        guild.rgl_team_id.set_if_not_equals(None);
                    }
                }

                guild
            }
        };

        let guild = active_guild.update(&tx).await?;

        interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(create_message().embeds(vec![
                    success_embed("Configuration updated."),
                    guild.config_embed(),
                ])),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
