use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::SubCommand;
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        ConnectInfo, GameFormat, MapList,
        game::{Game, GameServer, Scrim},
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
    opponent: Option<UserId>,

    /// Space-separated list of maps to be played.
    #[command(autocomplete)]
    maps: Option<MapList>,

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

        let game = Game {
            guild_id: guild.id,
            timestamp: self.date_time,
            server: self
                .connect_info
                .map(GameServer::Joined)
                .unwrap_or_default(),
            details: Scrim {
                opponent_user_id: self.opponent.map(Into::into),
                game_format: self
                    .game_format
                    .or(guild.game_format)
                    .ok_or(BotError::NoGameFormat)?,
                maps: self.maps.unwrap_or_default(),
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
