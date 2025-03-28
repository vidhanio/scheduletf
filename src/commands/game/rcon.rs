use sea_orm::{ColumnTrait, ModelTrait, QueryFilter, QueryOrder, QuerySelect};
use serenity::all::{CommandInteraction, Context, CreateAttachment, EditInteractionResponse};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        ReservationId,
        game::{self, ScrimOrMatch},
    },
    error::BotError,
    serveme::GetReservationRequest,
    utils::OffsetDateTimeEtExt,
};

#[derive(Clone, Debug, SubCommand)]
pub struct RconCommand {
    /// The command to run.
    command: String,

    /// The reservation to run the command on. If not provided, the most recent
    /// game will be used.
    #[command(autocomplete)]
    reservation: Option<ReservationId>,
}

impl RconCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let reservation_id = if let Some(reservation_id) = self.reservation {
            reservation_id
        } else {
            guild
                .select_closest_active_games::<ScrimOrMatch>()
                .await?
                .one(&tx)
                .await?
                .ok_or(BotError::NoActiveGames)?
                .server
                .reservation_id()?
        };

        let reservation =
            GetReservationRequest::send(guild.serveme_api_key()?, reservation_id).await?;

        let resp = reservation.rcon(&self.command).await?;

        let edit = if resp.len() + "```\n\n```".len() > 2000 {
            EditInteractionResponse::new()
                .new_attachment(CreateAttachment::bytes(resp.as_bytes(), "rcon.log"))
        } else {
            EditInteractionResponse::new().content(format!("```\n{resp}\n```"))
        };

        interaction.edit_response(&ctx, edit).await?;

        Ok(())
    }
}

impl RconCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Reservation { reservation, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_reservations::<ScrimOrMatch>(
                        ctx,
                        interaction,
                        tx,
                        |r| r.status.is_ready(),
                        &reservation,
                    )
                    .await
            }
        }
    }
}
