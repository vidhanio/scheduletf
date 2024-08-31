mod commands;
mod config;
mod error;
#[allow(
    missing_copy_implementations,
    missing_debug_implementations,
    clippy::pedantic,
    clippy::nursery
)]
#[rustfmt::skip]
mod schema;
mod utils;

use std::sync::Arc;

use serenity::all::{
    async_trait, Context, EventHandler, GatewayIntents, Guild, GuildId, Interaction, Ready,
};
use serenity_commands::Commands;
use sqlx::{query, PgPool};
use tracing::{error, info, instrument};
use utils::handle_error;

pub use self::config::Config;
use self::{commands::AllCommands, error::BotError};

type BotResult<T = ()> = Result<T, BotError>;

pub async fn run(config: Config) -> BotResult {
    info!("connecting to database...");

    let pool = PgPool::connect(&config.database_url).await?;

    let bot = Bot {
        config: Arc::new(config),
        pool,
        http_client: reqwest::Client::new(),
    };

    info!("building client...");

    let mut client =
        serenity::Client::builder(&bot.config.discord_bot_token, GatewayIntents::empty())
            .event_handler(bot)
            .await?;

    info!("starting client...");

    client.start().await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct Bot {
    config: Arc<Config>,
    pool: PgPool,
    http_client: reqwest::Client,
}

impl Bot {
    #[instrument(skip(self))]
    async fn insert_guild(&self, guild: GuildId) -> BotResult {
        query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT DO NOTHING",
            i64::from(guild)
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[async_trait]
impl EventHandler for Bot {
    #[instrument(skip(self, ctx))]
    async fn ready(&self, ctx: Context, _: Ready) {
        let commands = AllCommands::create_commands();

        for guild in &self.config.guilds {
            if let Err(error) = self.insert_guild(*guild).await {
                error!(?guild, ?error, "could not insert guild into database");
            }

            match guild.set_commands(&ctx.http, commands.clone()).await {
                Ok(commands) => info!(?guild, ?commands, "registered commands"),
                Err(error) => error!(?guild, ?error, "failed to register commands"),
            }
        }
    }

    #[instrument(skip(self, ctx))]
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::Command(interaction) = interaction {
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
        };
    }
}
