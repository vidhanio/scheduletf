use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;

use crate::{
    Bot, BotResult,
    entities::{
        ReservationId,
        game::{Game, GameServer, Match},
    },
    rgl::{RglMatch, RglMatchId},
    utils::success_embed,
};

#[derive(Clone, Debug, SubCommand)]
pub struct HostCommand {
    /// The match ID of the RGL official.
    match_id: RglMatchId,

    /// An existing reservation to set up and modify. If not provided, a new
    /// reservation will be created.
    reservation_id: Option<ReservationId>,
}

impl HostCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let rgl_match = RglMatch::get(self.match_id).await?;

        guild.ensure_time_open(&tx, rgl_match.match_date).await?;

        let mut game = Game {
            guild_id: guild.id,
            timestamp: rgl_match.match_date,
            server: self
                .reservation_id
                .map(GameServer::Hosted)
                .unwrap_or_default(),
            details: Match {
                rgl_match_id: self.match_id,
            },
        };

        let serveme_api_key = guild.serveme_api_key()?;

        if game.server.is_hosted() {
            game.edit_reservation(serveme_api_key).await?;
        } else {
            game.create_reservation(serveme_api_key).await?;
        }

        let game = Game::try_from(game.into_active_model().insert(&tx).await?)?;

        let embed = game.embed(Some(serveme_api_key), guild.rgl_team_id).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Official scheduled."), embed]),
            )
            .await?;

        Ok(())
    }
}
