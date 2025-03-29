use std::collections::BTreeMap;

use sea_orm::{ColumnTrait, QueryFilter};
use serenity::all::{
    CommandInteraction, Context, CreateEmbed, EditInteractionResponse, Mentionable,
};
use serenity_commands::SubCommand;
use time::{Date, Time};

use crate::{
    Bot, BotResult,
    entities::{
        GameFormat,
        game::{self, ScrimOrMatch},
    },
    error::BotError,
    utils::{OffsetDateTimeEtExt, lfs_date_string, lfs_date_string_single, lfs_time_string},
};

#[derive(Clone, Debug, SubCommand)]
pub struct LfsCommand {
    /// The game format of the LFS message. Defaults to the guild's default game
    /// format.
    game_format: Option<GameFormat>,

    /// The division to use in the LFS message. If not provided, the guild's
    /// default division will be used.
    division: Option<String>,
}

impl LfsCommand {
    #[allow(clippy::too_many_lines)]
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        interaction.defer_ephemeral(ctx).await?;

        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

        let game_format = self
            .game_format
            .or(guild.game_format)
            .ok_or(BotError::NoGameFormat)?;

        let division = self
            .division
            .as_deref()
            .or(guild.scrim_division.as_deref())
            .ok_or(BotError::NoDivision)?;

        let games = guild
            .select_games::<ScrimOrMatch>(|s| {
                s.filter(game::Column::OpponentUserId.is_null())
                    .filter(game::Column::GameFormat.eq(game_format))
            })
            .all(&tx)
            .await?;

        let mut map = BTreeMap::<Date, Vec<Time>>::new();

        for game in games {
            let date = game.timestamp.date_et();
            let time = game.timestamp.time_et();

            map.entry(date).or_default().push(time);
        }

        let timings = match map.len() {
            0 => {
                return Err(BotError::NoScrimsWithoutOpponent);
            }
            1 => {
                let (date, times) = map.into_iter().next().unwrap();
                format!(
                    " {}{}",
                    lfs_date_string_single(date),
                    times
                        .into_iter()
                        .map(lfs_time_string)
                        .collect::<Vec<_>>()
                        .join("/")
                )
            }
            _ => {
                format!(
                    "\n{}",
                    map.into_iter()
                        .map(|(date, games)| {
                            format!(
                                "{} {}",
                                lfs_date_string(date),
                                games
                                    .into_iter()
                                    .map(lfs_time_string)
                                    .collect::<Vec<_>>()
                                    .join("/")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        };

        let embed = CreateEmbed::new()
            .title("Looking for Scrim")
            .description(format!("```\nlfs {division}{timings}\n```"))
            .field(
                "LFS Channel",
                game_format.lfs_channel().mention().to_string(),
                false,
            );

        interaction
            .edit_response(&ctx, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
