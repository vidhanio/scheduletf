use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        game::{self, ConnectInfo, Maps},
        team_guild::GameFormat,
    },
    error::BotError,
    utils::success_embed,
};

#[derive(Clone, Debug, SubCommand)]
pub struct JoinCommand {
    /// The date/time to schedule the scrim for.
    #[command(autocomplete)]
    date_time: OffsetDateTime,

    /// Opposing team's contacted team member. Enter their user ID if they are
    /// not in the server.
    opponent: UserId,

    /// Comma-separated list of maps to be played.
    #[command(autocomplete)]
    maps: Option<Maps>,

    /// The game format of the scrim. Defaults to the guild's default game
    /// format.
    game_format: Option<GameFormat>,

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

        guild.ensure_time_open(&tx, self.date_time).await?;

        let (ip_and_port, password) = self
            .connect_info
            .map(|connect_info| (connect_info.ip_and_port, connect_info.password))
            .unzip();

        let game = game::Model {
            guild_id: guild.id,
            timestamp: self.date_time,
            game_format: self
                .game_format
                .or(guild.game_format)
                .ok_or(BotError::NoGameFormat)?,
            opponent_user_id: self.opponent.into(),
            reservation_id: None,
            server_ip_and_port: ip_and_port,
            server_password: password,
            maps: Some(self.maps.unwrap_or_default()),
            rgl_match_id: None,
        };

        let game = game.into_active_model().reset_all().insert(&tx).await?;

        let embed = game.embed(None).await?;

        guild.refresh_schedule(ctx, &tx).await?;

        tx.commit().await?;

        interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new()
                    .embeds(vec![success_embed("Scrim scheduled."), embed]),
            )
            .await?;

        Ok(())
    }
}

impl JoinCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::DateTime { date_time, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_times(ctx, interaction, tx, &date_time)
                    .await
            }
            Self::Maps {
                maps, game_format, ..
            } => {
                let guild = bot.get_guild(interaction.guild_id).await?;

                guild
                    .autocomplete_maps(ctx, interaction, game_format.flatten().into_value(), &maps)
                    .await
            }
        }
    }
}
