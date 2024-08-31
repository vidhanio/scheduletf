use serenity::all::{
    ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateSelectMenu, CreateSelectMenuKind,
    EditInteractionResponse,
};
use serenity_commands::Command;
use sqlx::{query, query_scalar};

use crate::{
    error::BotError,
    utils::{self, success_embed, success_message_title},
    Bot, BotResult,
};

#[derive(Debug, Command)]
pub enum ConfigCommand {
    /// Set the na.serveme.tf api key.
    ServemeApiKey {
        /// The api key.
        api_key: String,
    },

    /// Set the voice channel for games.
    VoiceChannel,

    /// Set the schedule channel.
    ScheduleChannel,

    /// Set the logs channel.
    LogsChannel,
}

impl ConfigCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let guild_id = interaction.guild_id.ok_or(BotError::NoGuild)?;
        match self {
            Self::ServemeApiKey { api_key } => {
                query!(
                    "INSERT INTO guilds (id, serveme_api_key) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET serveme_api_key = $2",
                    i64::from(guild_id),
                    api_key,
                )
                .execute(&bot.pool)
                .await?;

                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(success_message_title("API Key Set")),
                    )
                    .await?;
            }
            Self::VoiceChannel => {
                let current = query_scalar!(
                    "SELECT voice_channel FROM guilds WHERE id = $1",
                    i64::from(guild_id),
                )
                .fetch_one(&bot.pool)
                .await?;

                let select_menu = CreateSelectMenu::new(
                    "config:voice-channel",
                    CreateSelectMenuKind::Channel {
                        channel_types: Some(vec![ChannelType::Voice]),
                        default_channels: current.map(|current| vec![(current as u64).into()]),
                    },
                )
                .placeholder("Select the game voice channel.");

                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .select_menu(select_menu)
                                .ephemeral(true),
                        ),
                    )
                    .await?;

                if let Some(component_interaction) = interaction
                    .get_response(ctx)
                    .await?
                    .await_component_interaction(ctx)
                    .await
                {
                    let channel_id = *utils::get_single_from_select::<ChannelId>(
                        "config:voice-channel",
                        &component_interaction,
                    )?;

                    query!(
                        "INSERT INTO guilds (id, voice_channel) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET voice_channel = $2",
                        i64::from(guild_id),
                        i64::from(channel_id),
                    )
                    .execute(&bot.pool)
                    .await?;

                    component_interaction
                        .create_response(ctx, CreateInteractionResponse::Acknowledge)
                        .await?;

                    interaction
                        .edit_response(
                            ctx,
                            EditInteractionResponse::new()
                                .embed(success_embed("Voice Channel Set"))
                                .components(vec![]),
                        )
                        .await?;
                }
            }
            Self::ScheduleChannel => {
                let current = query_scalar!(
                    "SELECT schedule_channel FROM guilds WHERE id = $1",
                    i64::from(guild_id),
                )
                .fetch_one(&bot.pool)
                .await?;

                let select_menu = CreateSelectMenu::new(
                    "config:schedule-channel",
                    CreateSelectMenuKind::Channel {
                        channel_types: Some(vec![ChannelType::Text]),
                        default_channels: current.map(|current| vec![(current as u64).into()]),
                    },
                )
                .placeholder("Select the announcement channel.");

                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .select_menu(select_menu)
                                .ephemeral(true),
                        ),
                    )
                    .await?;

                if let Some(component_interaction) = interaction
                    .get_response(ctx)
                    .await?
                    .await_component_interaction(ctx)
                    .await
                {
                    let channel_id = *utils::get_single_from_select::<ChannelId>(
                        "config:schedule-channel",
                        &component_interaction,
                    )?;

                    query!(
                        "INSERT INTO guilds (id, schedule_channel) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET schedule_channel = $2",
                        i64::from(guild_id),
                        i64::from(channel_id),
                    )
                    .execute(&bot.pool)
                    .await?;

                    component_interaction
                        .create_response(ctx, CreateInteractionResponse::Acknowledge)
                        .await?;

                    interaction
                        .edit_response(
                            ctx,
                            EditInteractionResponse::new()
                                .embed(success_embed("Schedule Channel Set"))
                                .components(vec![]),
                        )
                        .await?;
                }
            }
            Self::LogsChannel => {
                let current = query_scalar!(
                    "SELECT logs_channel FROM guilds WHERE id = $1",
                    i64::from(guild_id),
                )
                .fetch_one(&bot.pool)
                .await?;

                let select_menu = CreateSelectMenu::new(
                    "config:logs-channel",
                    CreateSelectMenuKind::Channel {
                        channel_types: Some(vec![ChannelType::Text]),
                        default_channels: current.map(|current| vec![(current as u64).into()]),
                    },
                )
                .placeholder("Select the logs channel.");

                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .select_menu(select_menu)
                                .ephemeral(true),
                        ),
                    )
                    .await?;

                if let Some(component_interaction) = interaction
                    .get_response(ctx)
                    .await?
                    .await_component_interaction(ctx)
                    .await
                {
                    let channel_id = *utils::get_single_from_select::<ChannelId>(
                        "config:logs-channel",
                        &component_interaction,
                    )?;

                    query!(
                        "INSERT INTO guilds (id, logs_channel) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET logs_channel = $2",
                        i64::from(guild_id),
                        i64::from(channel_id),
                    )
                    .execute(&bot.pool)
                    .await?;

                    component_interaction
                        .create_response(ctx, CreateInteractionResponse::Acknowledge)
                        .await?;

                    interaction
                        .edit_response(
                            ctx,
                            EditInteractionResponse::new()
                                .embed(success_embed("Logs Channel Set"))
                                .components(vec![]),
                        )
                        .await?;
                }
            }
        }

        Ok(())
    }
}
