use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;
use sqlx::query_as;
use tracing::error;

use crate::{
    models::{DbScrim, NextDay, Scrim, Time},
    utils::success_embed,
    Bot, BotResult,
};

#[derive(Clone, Debug, SubCommand)]
pub struct CancelCommand {
    /// The next day of the week the scrim is scheduled for.
    day: NextDay,

    /// The time the scrim is scheduled for.
    time: Time,
}

impl CancelCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let timestamp = self.day.to_datetime(self.time);

        let scrim = Scrim::from(
            query_as!(
                DbScrim,
                "DELETE FROM scrims
                WHERE guild_id = $1 AND timestamp = $2
                RETURNING *",
                i64::from(guild.id),
                timestamp,
            )
            .fetch_one(&mut *tx)
            .await?,
        );

        if let Some((serveme_api_key, reservation_id)) =
            guild.serveme_api_key.zip(scrim.reservation_id())
        {
            if let Err(error) = bot
                .delete_serveme_reservation(&serveme_api_key, reservation_id)
                .await
            {
                error!(?error, "Failed to delete serveme reservation");
            }
        };

        if let Err(error) = guild.id.delete_scheduled_event(ctx, scrim.event_id).await {
            error!(?error, "Failed to delete event");
        }

        if let Some((games_channel, game_message)) = guild.games_channel_id.zip(scrim.message_id) {
            if let Err(error) = games_channel.delete_message(ctx, game_message).await {
                error!(?error, "Failed to delete game message");
            }
        }

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Scrim deleted."), scrim.embed()]),
            )
            .await?;

        Ok(())
    }
}
