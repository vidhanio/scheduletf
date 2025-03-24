use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::SubCommand;

use crate::{
    Bot, BotResult,
    entities::{
        ConnectInfo,
        game::{Game, GameServer, Match},
    },
    rgl::{RglMatch, RglMatchId},
    utils::success_embed,
};

#[derive(Clone, Debug, SubCommand)]
pub struct JoinCommand {
    /// The ID of the RGL.gg match to join.
    match_id: RglMatchId,

    /// The connect info for the other team's server.
    connect_info: Option<ConnectInfo>,
}

impl JoinCommand {
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

        let game = Game {
            guild_id: guild.id,
            timestamp: rgl_match.match_date,
            server: self
                .connect_info
                .map(GameServer::Joined)
                .unwrap_or_default(),
            details: Match {
                rgl_match_id: self.match_id,
            },
        };

        let game = Game::try_from(game.into_active_model().insert(&tx).await?)?;

        let embed = game.embed(&guild).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Match scheduled."), embed]),
            )
            .await?;

        Ok(())
    }
}
