use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use sqlx::{query, query_as};
use tracing::error;

use crate::{
    models::{DbScrim, Scrim},
    utils::success_embed,
    Bot, BotResult,
};

#[derive(Clone, Debug, SubCommand)]
pub struct RefreshCommand;

impl RefreshCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let scrims = query_as!(
            DbScrim,
            "SELECT * FROM scrims
            WHERE guild_id = $1 AND timestamp > NOW()
            ORDER BY timestamp",
            i64::from(guild.id)
        )
        .fetch_all(&mut *tx)
        .await?;

        for scrim in scrims {
            let mut scrim = Scrim::from(scrim);

            if let Err(error) = guild.id.delete_scheduled_event(ctx, scrim.event_id).await {
                error!(?error, "Failed to delete scheduled event");
            }

            if let Some((games_channel, schedule_message)) =
                guild.games_channel_id.zip(scrim.message_id)
            {
                if let Err(error) = games_channel.delete_message(ctx, schedule_message).await {
                    error!(?error, "Failed to delete schedule message");
                }
            }

            scrim.event_id = guild
                .id
                .create_scheduled_event(ctx, scrim.create_event())
                .await?
                .id;

            let message_id = if let Some(games_channel) = guild.games_channel_id {
                Some(games_channel.send_message(ctx, scrim.message()).await?.id)
            } else {
                None
            };

            query!(
                "UPDATE scrims SET event_id = $1, message_id = $2
                WHERE guild_id = $3 AND timestamp = $4",
                i64::from(scrim.event_id),
                message_id.map(i64::from),
                i64::from(guild.id),
                scrim.timestamp,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().embeds(vec![success_embed(
                    "Scrim events and schedule messages refreshed.",
                )]),
            )
            .await?;

        Ok(())
    }
}
