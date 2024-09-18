use serenity::all::{
    ChannelId, ChannelType, CommandInteraction, Context, CreateInteractionResponse, Mentionable,
};
use serenity_commands::Command;
use sqlx::query;

use crate::{
    models::GameFormat,
    utils::{create_message, embed, success_embed},
    Bot, BotResult,
};

#[derive(Debug, Command)]
pub enum ConfigCommand {
    /// Show the current configuration.
    Show,

    /// Set the na.serveme.tf api key.
    Serveme {
        /// The na.serveme.tf API key.
        api_key: Option<String>,
    },

    /// Set the game format for this server.
    GameFormat {
        /// The game format.
        game_format: Option<GameFormat>,
    },

    /// Set the games channel.
    GamesChannel {
        /// The games channel.
        #[command(builder(channel_types(vec![ChannelType::Text])))]
        games_channel: Option<ChannelId>,
    },
}

impl ConfigCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let embed = match self {
            Self::Show => {
                let fields = vec![
                    (
                        "na.serveme.tf API Key",
                        guild
                            .serveme_api_key
                            .map(|_| format!("`{}`", "*".repeat(32))),
                    ),
                    (
                        "Games Channel",
                        guild.games_channel_id.map(|id| id.mention().to_string()),
                    ),
                ];

                embed("Configuration").fields(fields.into_iter().map(|(name, value)| {
                    (
                        name.to_string(),
                        value.unwrap_or_else(|| "Not set".to_string()),
                        true,
                    )
                }))
            }
            Self::Serveme { api_key } => {
                query!(
                    r#"UPDATE guilds SET serveme_api_key = $1
                    WHERE id = $2"#,
                    api_key,
                    i64::from(guild.id),
                )
                .execute(&mut *tx)
                .await?;

                success_embed("na.serveme.tf API key set.")
            }
            Self::GamesChannel { games_channel } => {
                query!(
                    r#"UPDATE guilds SET games_channel_id = $1
                    WHERE id = $2"#,
                    games_channel.map(i64::from),
                    i64::from(guild.id),
                )
                .execute(&mut *tx)
                .await?;

                success_embed("Games channel set.")
            }
        };

        interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(create_message().embed(embed)),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }
}
