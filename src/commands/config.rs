use sea_orm::{ActiveModelTrait, IntoActiveModel};
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
                    #[doc = concat!("The ", stringify!($doc), ". If left empty, this unsets the option.")]
                    $field: Option<$field_ty>,
                },
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
    RglTeam { id: RglTeamId },

    "division to use in LFS messages"
    ScrimDivision { division: String },
}

impl ConfigCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        match self {
            Self::Show => {
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            create_message().embed(guild.config_embed()),
                        ),
                    )
                    .await?;
            }
            Self::Set(cmd) => {
                let mut guild = guild.into_active_model();

                match cmd {
                    ConfigSetCommand::Serveme { key } => {
                        guild.serveme_api_key.set_if_not_equals(key);
                    }
                    ConfigSetCommand::GameFormat { format } => {
                        guild.game_format.set_if_not_equals(format);
                    }
                    ConfigSetCommand::ScheduleChannel { channel } => {
                        guild.schedule_channel_id.set_if_not_equals(channel);
                    }
                    ConfigSetCommand::RglTeam { id } => {
                        guild.rgl_team_id.set_if_not_equals(id);

                        if let Some(team_id) = id {
                            let team = RglTeam::get(team_id).await?;

                            let season = RglSeason::get(team.season_id).await?;

                            guild
                                .game_format
                                .set_if_not_equals(Some(season.format_name));
                        }
                    }
                    ConfigSetCommand::ScrimDivision { division } => {
                        guild.scrim_division.set_if_not_equals(division);
                    }
                }

                let guild = guild.update(&tx).await?;

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
            }
        }

        Ok(())
    }
}
