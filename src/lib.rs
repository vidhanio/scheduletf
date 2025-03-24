mod autocomplete;
mod commands;
mod components;
mod config;
mod entities;
mod error;
mod rgl;
mod serveme;
mod utils;

use std::sync::{Arc, LazyLock};

use commands::AllCommandsAutocomplete;
use components::AllComponents;
use entities::team_guild;
use migration::{Migrator, MigratorTrait};
use sea_orm::{
    ActiveValue::Set, Database, DatabaseConnection, DatabaseTransaction, TransactionTrait,
    prelude::*,
};
use serenity::all::{
    Command, Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready, async_trait,
};
use serenity_commands::{AutocompleteCommands, Commands};
use tracing::{error, info, instrument};
use utils::handle_error;

pub use self::config::Config;
use self::{commands::AllCommands, error::BotError};

type BotResult<T = ()> = Result<T, BotError>;

pub async fn run(config: Config) -> BotResult {
    info!("connecting to database...");

    let db = Database::connect(&config.database_url).await?;

    info!("running migrations...");
    Migrator::up(&db, None).await?;

    let bot = Bot {
        config: Arc::new(config),
        db,
    };

    info!("building client...");

    let mut client = serenity::Client::builder(
        &bot.config.discord_bot_token,
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::GUILD_SCHEDULED_EVENTS,
    )
    .event_handler(bot)
    .await?;

    info!("starting client...");

    client.start().await?;

    Ok(())
}

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(reqwest::Client::new);

#[derive(Debug, Clone)]
pub struct Bot {
    config: Arc<Config>,
    db: DatabaseConnection,
}

impl Bot {
    #[instrument(skip(self))]
    async fn get_guild(&self, guild_id: Option<GuildId>) -> BotResult<team_guild::Model> {
        let (guild, tx) = self.get_guild_tx(guild_id).await?;

        tx.commit().await?;

        Ok(guild)
    }

    #[instrument(skip(self))]
    async fn get_guild_tx(
        &self,
        guild_id: Option<GuildId>,
    ) -> BotResult<(team_guild::Model, DatabaseTransaction)> {
        let guild_id = guild_id.ok_or(BotError::NoGuild)?;

        let tx = self.db.begin().await?;

        let guild = team_guild::Entity::find_by_id(guild_id).one(&tx).await?;

        let guild = if let Some(guild) = guild {
            guild
        } else {
            team_guild::ActiveModel {
                id: Set(guild_id.into()),
                ..Default::default()
            }
            .insert(&tx)
            .await?
        };

        Ok((guild, tx))
    }
}

#[async_trait]
impl EventHandler for Bot {
    #[instrument(skip(self, ctx))]
    async fn ready(&self, ctx: Context, _: Ready) {
        let commands = AllCommands::create_commands();

        if let Some(guilds) = &self.config.guilds {
            info!(?self.config.guilds, "registering guild commands");

            for guild in guilds {
                match guild
                    .set_commands(&ctx.http, commands[..commands.len() - 1].to_vec())
                    .await
                {
                    Ok(commands) => info!(?guild, ?commands, "registered guild commands"),
                    Err(error) => error!(?guild, ?error, "failed to register guild commands"),
                }
            }

            match Command::create_global_command(ctx, commands.last().unwrap().clone()).await {
                Ok(command) => info!(?command, "registered global user command"),
                Err(error) => error!(?error, "failed to register global user command"),
            }
        } else {
            info!("no guilds configured, registering global commands");

            match Command::set_global_commands(&ctx.http, commands).await {
                Ok(commands) => info!(?commands, "registered global commands"),
                Err(error) => error!(?error, "failed to register global commands"),
            }
        }
    }

    #[instrument(skip(self, ctx))]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        match interaction {
            Interaction::Command(interaction) => {
                let command = handle_error!(
                    ctx,
                    interaction,
                    AllCommands::from_command_data(&interaction.data).map_err(Into::into)
                );

                handle_error!(
                    ctx,
                    interaction,
                    command.run(self, &ctx, &interaction).await
                );
            }
            Interaction::Autocomplete(interaction) => {
                let command = handle_error!(
                    ctx,
                    interaction,
                    AllCommandsAutocomplete::from_command_data(&interaction.data)
                        .map_err(Into::into)
                );

                handle_error!(
                    ctx,
                    interaction,
                    command.autocomplete(self, &ctx, &interaction).await
                );
            }
            Interaction::Component(interaction) => {
                let command = handle_error!(
                    ctx,
                    interaction,
                    AllComponents::from_component_data(&interaction.data)
                );

                handle_error!(
                    ctx,
                    interaction,
                    command.run(self, &ctx, &interaction).await
                );
            }
            _ => {
                error!(?interaction, "unsupported interaction type");
            }
        }
    }
}
