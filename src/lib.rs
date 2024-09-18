mod commands;
mod components;
mod config;
mod error;
mod models;
mod serveme;
mod utils;

use std::sync::Arc;

use models::{DbGuild, DbScrim, Guild, Scrim};
use serenity::all::{
    async_trait, Context, EventHandler, GatewayIntents, GuildId, Interaction, Ready,
    ScheduledEvent, ScheduledEventStatus,
};
use serenity_commands::Commands;
use serveme::{DeleteReservationRequest, FindServersRequest, ReservationResponse};
use sqlx::{query, query_as, PgPool, Postgres};
use time::Duration;
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

#[derive(Debug, Clone)]
pub struct Bot {
    config: Arc<Config>,
    pool: PgPool,
    http_client: reqwest::Client,
}

impl Bot {
    #[instrument(skip(self))]
    async fn get_guild_tx(
        &self,
        guild_id: Option<GuildId>,
    ) -> BotResult<(Guild, sqlx::Transaction<'_, Postgres>)> {
        let guild_id = i64::from(guild_id.ok_or(BotError::NoGuild)?);

        let mut tx = self.pool.begin().await?;

        query!(
            r#"
            INSERT INTO guilds (id)
            VALUES ($1)
            ON CONFLICT (id) DO NOTHING
            "#,
            guild_id,
        )
        .execute(&mut *tx)
        .await?;

        let db_guild = query_as!(
            DbGuild,
            r#"
            SELECT * FROM guilds WHERE id = $1
            "#,
            guild_id,
        )
        .fetch_one(&mut *tx)
        .await?;

        Ok((db_guild.into(), tx))
    }

    #[instrument(skip(self))]
    pub async fn new_serveme_reservation(
        &self,
        api_key: &str,
        scrim: &Scrim,
    ) -> BotResult<ReservationResponse> {
        let servers = FindServersRequest {
            starts_at: scrim.timestamp - 10 * Duration::MINUTE,
            ends_at: scrim.timestamp + Duration::HOUR,
        }
        .send(&self.http_client, api_key)
        .await?;

        scrim
            .new_reservation_request(&servers)?
            .send(&self.http_client, api_key)
            .await
    }

    #[instrument(skip(self))]
    pub async fn edit_serveme_reservation(
        &self,
        api_key: &str,
        scrim: &Scrim,
        reservation_id: u32,
    ) -> BotResult<ReservationResponse> {
        scrim
            .edit_reservation_request()
            .send(&self.http_client, api_key, reservation_id)
            .await
    }

    #[instrument(skip(self))]
    pub async fn delete_serveme_reservation(
        &self,
        api_key: &str,
        reservation_id: u32,
    ) -> BotResult<Option<ReservationResponse>> {
        DeleteReservationRequest::send(&self.http_client, api_key, reservation_id).await
    }
}

#[async_trait]
impl EventHandler for Bot {
    #[instrument(skip(self, ctx))]
    async fn ready(&self, ctx: Context, _: Ready) {
        let commands = AllCommands::create_commands();

        for guild in &self.config.guilds {
            match guild.set_commands(&ctx.http, commands.clone()).await {
                Ok(commands) => info!(?guild, ?commands, "registered commands"),
                Err(error) => error!(?guild, ?error, "failed to register commands"),
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
            Interaction::Component(interaction) => {
                handle_error!(
                    ctx,
                    interaction,
                    components::run(self, &ctx, &interaction).await
                );
            }
            _ => {}
        };
    }

    async fn guild_scheduled_event_update(&self, ctx: Context, event: ScheduledEvent) {
        if event.creator_id != Some(ctx.cache.current_user().id) {
            return;
        }

        if event.status == ScheduledEventStatus::Active {
            let (guild, mut tx) = handle_error!(self.get_guild_tx(Some(event.guild_id)).await);

            let scrim = handle_error!(query_as!(
                DbScrim,
                r#"
                UPDATE scrims
                SET status = 1
                WHERE event_id = $1
                RETURNING *
                "#,
                i64::from(event.id)
            )
            .fetch_one(&mut *tx)
            .await
            .map(Scrim::from));

            if let Some((games_channel, message_id)) = guild.games_channel_id.zip(scrim.message_id)
            {
                handle_error!(
                    games_channel
                        .edit_message(&ctx, message_id, scrim.edit_message())
                        .await
                );
            }
        }
    }
}
