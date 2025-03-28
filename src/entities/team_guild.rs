use std::{
    collections::{BTreeMap, HashSet},
    iter, mem,
    string::ToString,
};

use sea_orm::{
    ActiveValue::Set,
    DatabaseTransaction, IntoActiveModel, QueryOrder, QuerySelect, SelectModel, Selector,
    entity::prelude::*,
    sea_query::{Func, SimpleExpr},
};
use serenity::{
    all::{
        AutocompleteChoice, CommandInteraction, Context, CreateAutocompleteResponse, CreateEmbed,
        CreateInteractionResponse, CreateMessage, DiscordJsonError, EditMessage, ErrorResponse,
        HttpError, Mentionable,
    },
    futures::{StreamExt, TryStreamExt, stream},
};
use time::{Date, Duration, OffsetDateTime, Time};

use super::{
    GameFormat, MapList, ReservationId, ScheduleChannelId, ScheduleMessageId, ServemeApiKey,
    TeamGuildId,
    game::{Game, GameDetails, ScrimOrMatch},
};
use crate::{
    BotResult,
    autocomplete::{
        DEFAULT_TIME_CHOICES, TIME_CHOICES, day_aliases, day_choices, split_datetime_query,
        time_aliases,
    },
    components::RefreshButton,
    entities::game,
    error::BotError,
    rgl::RglTeamId,
    serveme::{GetReservationRequest, MapsRequest, ReservationResponse},
    utils::{OffsetDateTimeEtExt, date_string},
};

#[derive(Clone, Debug, PartialEq, Eq, Default, DeriveEntityModel)]
#[sea_orm(table_name = "team_guild")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: TeamGuildId,
    pub rgl_team_id: Option<RglTeamId>,
    pub game_format: Option<GameFormat>,
    pub schedule_channel_id: Option<ScheduleChannelId>,
    pub schedule_message_id: Option<ScheduleMessageId>,
    pub serveme_api_key: Option<ServemeApiKey>,
}

impl Model {
    pub async fn get_game<D: GameDetails>(
        &self,
        tx: &DatabaseTransaction,
        date_time: OffsetDateTime,
    ) -> BotResult<Game<D>> {
        game::Entity::find_by_id((self.id, date_time))
            .filter(D::filter_expr())
            .into_partial_model()
            .one(tx)
            .await?
            .ok_or(BotError::GameNotFound)
    }

    fn select_games<D: GameDetails>(&self, limit: Option<u64>) -> Selector<SelectModel<Game<D>>> {
        self.find_related(game::Entity)
            .filter(game::Column::Timestamp.gt(OffsetDateTime::now_et() - Duration::hours(6)))
            .filter(D::filter_expr())
            .order_by_asc(game::Column::Timestamp)
            .limit(limit)
            .into_partial_model()
    }

    pub async fn select_closest_active_games<D: GameDetails>(
        &self,
    ) -> BotResult<Selector<SelectModel<Game<D>>>> {
        let reservations = GetReservationRequest::send_many(self.serveme_api_key()?).await?;

        let ready_reservation_ids = reservations
            .iter()
            .filter(|r| r.status.is_ready())
            .map(|r| r.id);

        Ok(self
            .find_related(game::Entity)
            .filter(D::filter_expr())
            .filter(game::Column::ReservationId.is_in(ready_reservation_ids))
            .order_by_desc(game::Column::Timestamp.lt(OffsetDateTime::now_et()))
            .order_by_asc(SimpleExpr::from(Func::greatest([
                game::Column::Timestamp
                    .into_expr()
                    .sub(Expr::current_timestamp()),
                game::Column::Timestamp
                    .into_expr()
                    .sub(Expr::current_timestamp())
                    .mul(-1),
            ])))
            .into_partial_model())
    }

    pub async fn ensure_time_open(
        &self,
        tx: &DatabaseTransaction,
        date_time: OffsetDateTime,
    ) -> BotResult {
        game::Entity::find_by_id((self.id, date_time))
            .select_only()
            .expr(1)
            .into_tuple::<i32>()
            .one(tx)
            .await?
            .is_none()
            .then_some(())
            .ok_or(BotError::TimeSlotTaken)
    }

    pub fn serveme_api_key(&self) -> BotResult<&ServemeApiKey> {
        self.serveme_api_key
            .as_ref()
            .ok_or(BotError::NoServemeApiKey)
    }

    pub fn rgl_team_id(&self) -> BotResult<RglTeamId> {
        self.rgl_team_id.ok_or(BotError::NoRglTeam)
    }

    pub async fn autocomplete_times(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        tx: DatabaseTransaction,
        query: &str,
    ) -> BotResult {
        let (_, day_query, time_query) = split_datetime_query(query);

        let dates = day_choices()
            .filter_map(|(date, names)| {
                names
                    .iter()
                    .any(|n| n.starts_with(&day_query))
                    .then_some(date)
            })
            .collect::<Vec<_>>();

        let taken_datetimes = self
            .find_related(game::Entity)
            .filter(
                game::Column::Timestamp.gt(OffsetDateTime::now_et().replace_time(Time::MIDNIGHT)),
            )
            .select_only()
            .column(game::Column::Timestamp)
            .into_tuple::<OffsetDateTime>()
            .all(&tx)
            .await?
            .into_iter()
            .collect::<HashSet<_>>();

        let min_timestamp = OffsetDateTime::now_et() - Duration::minutes(30);

        let datetimes = match dates.as_slice() {
            [] => {
                vec![]
            }
            [date] => TIME_CHOICES
                .iter()
                .filter(|(_, names)| names.iter().any(|n| n.starts_with(&time_query)))
                .map(|(time, _)| OffsetDateTime::new_et(*date, *time))
                .filter(|datetime| {
                    !taken_datetimes.contains(datetime) && datetime >= &min_timestamp
                })
                .take(25)
                .collect::<Vec<_>>(),
            dates => {
                if time_query.is_empty() {
                    dates
                        .iter()
                        .flat_map(|date| {
                            DEFAULT_TIME_CHOICES
                                .into_iter()
                                .map(|time| OffsetDateTime::new_et(*date, time))
                        })
                        .filter(|datetime| {
                            !taken_datetimes.contains(datetime) && datetime >= &min_timestamp
                        })
                        .take(25)
                        .collect::<Vec<_>>()
                } else {
                    dates
                        .iter()
                        .flat_map(|date| {
                            TIME_CHOICES
                                .iter()
                                .filter(|(_, names)| {
                                    names.iter().any(|n| n.starts_with(&time_query))
                                })
                                .map(|(time, _)| OffsetDateTime::new_et(*date, *time))
                        })
                        .filter(|datetime| {
                            !taken_datetimes.contains(datetime) && datetime >= &min_timestamp
                        })
                        .take(25)
                        .collect::<Vec<_>>()
                }
            }
        };

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(
                        datetimes
                            .into_iter()
                            .map(|datetime| {
                                AutocompleteChoice::new(
                                    datetime.string_et_relative(),
                                    datetime.unix_timestamp(),
                                )
                            })
                            .collect(),
                    ),
                ),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn autocomplete_games<D: GameDetails>(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        tx: DatabaseTransaction,
        selector: Option<Selector<SelectModel<Game<D>>>>,
        query: &str,
    ) -> BotResult {
        let (_, day_query, time_query) = split_datetime_query(query);

        let matches = selector
            .unwrap_or_else(|| self.select_games::<D>(None))
            .all(&tx)
            .await?
            .into_iter()
            .filter(|game| {
                let date_matches = day_aliases(game.timestamp.date_et())
                    .iter()
                    .any(|n| n.starts_with(&day_query));

                let time_matches = time_aliases(game.timestamp.time_et())
                    .iter()
                    .any(|n| n.starts_with(&time_query));

                date_matches && time_matches
            })
            .take(25);

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(
                        stream::iter(matches)
                            .map(Ok)
                            .and_then(async |m| {
                                let opponent =
                                    m.details.opponent_string(ctx, self.rgl_team_id).await?;

                                let vs = opponent
                                    .map(|opponent| format!(" vs. {opponent}"))
                                    .unwrap_or_default();

                                BotResult::Ok(AutocompleteChoice::new(
                                    format!(
                                        "{}: {}{vs}",
                                        m.timestamp.string_et_relative(),
                                        m.details.name(),
                                    ),
                                    m.timestamp.unix_timestamp(),
                                ))
                            })
                            .try_collect()
                            .await?,
                    ),
                ),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn autocomplete_reservations<D: GameDetails>(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        tx: DatabaseTransaction,
        filter: impl Fn(&ReservationResponse) -> bool,
        query: &str,
    ) -> BotResult {
        let (query, day_query, time_query) = split_datetime_query(query);

        let reservations = GetReservationRequest::send_many(self.serveme_api_key()?)
            .await?
            .into_iter()
            .filter(|r| filter(r))
            .map(|r| r.id);

        let data = self
            .find_related(game::Entity)
            .filter(D::filter_expr())
            .filter(game::Column::ReservationId.is_in(reservations))
            .order_by_asc(game::Column::Timestamp)
            .select_only()
            .column(game::Column::Timestamp)
            .column(game::Column::ReservationId)
            .into_tuple::<(OffsetDateTime, ReservationId)>()
            .all(&tx)
            .await?;

        let mut map = BTreeMap::<ReservationId, Vec<OffsetDateTime>>::new();

        for (datetime, reservation) in data {
            map.entry(reservation).or_default().push(datetime);
        }

        let data = map
            .into_iter()
            .filter(|(reservation, datetimes)| {
                let date_matches = datetimes.iter().any(|datetime| {
                    day_aliases(datetime.date_et())
                        .iter()
                        .any(|n| n.starts_with(&day_query))
                });

                let time_matches = datetimes.iter().any(|datetime| {
                    time_aliases(datetime.time_et())
                        .iter()
                        .any(|n| n.starts_with(&time_query))
                });

                let reservation_matches = reservation.to_string().starts_with(&query);

                (date_matches && time_matches) || reservation_matches
            })
            .take(25);

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(
                        data.map(|(reservation, datetimes)| {
                            let datetimes = datetimes
                                .iter()
                                .map(OffsetDateTime::string_et_relative)
                                .collect::<Vec<_>>()
                                .join(", ");

                            AutocompleteChoice::new(
                                format!("{reservation} ({datetimes})"),
                                reservation.0,
                            )
                        })
                        .collect(),
                    ),
                ),
            )
            .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn autocomplete_maps(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        game_format: Option<GameFormat>,
        query: &str,
    ) -> BotResult {
        let game_format = game_format.or(self.game_format);

        let maps = query.parse::<MapList>().unwrap();

        let all_maps = MapsRequest::send(
            self.serveme_api_key
                .as_ref()
                .ok_or(BotError::NoServemeApiKey)?,
            game_format,
        )
        .await?;

        let trailing_sep =
            query.ends_with(',') || query.ends_with('/') || query.ends_with(char::is_whitespace);

        interaction
            .create_response(
                ctx,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new()
                        .set_choices(all_maps.autocomplete_choices(&maps, trailing_sep)),
                ),
            )
            .await?;

        Ok(())
    }

    async fn schedule_embed(&self, tx: &DatabaseTransaction) -> BotResult<CreateEmbed> {
        let games = self.select_games::<ScrimOrMatch>(Some(25)).all(tx).await?;

        let mut map = BTreeMap::<Date, Vec<Game>>::new();

        for game in games {
            let date = game.timestamp.date_et();

            map.entry(date).or_default().push(game);
        }

        let embed = CreateEmbed::new().title("🗓️ Schedule");

        let embed = if map.is_empty() {
            embed.description("No upcoming games.")
        } else {
            embed.fields(
                stream::iter(map)
                    .map(Ok)
                    .and_then(async |(date, games)| {
                        BotResult::Ok((
                            format!("**{}**", date_string(date)),
                            stream::iter(
                                games
                                    .iter()
                                    .zip(games.iter().skip(1).map(Some).chain(iter::once(None))),
                            )
                            .map(Ok)
                            .and_then(async |(game, next_game)| {
                                let include_connect = !next_game
                                    .is_some_and(|next_game| game.server == next_game.server);
                                game.schedule_entry(self, include_connect).await
                            })
                            .try_collect::<String>()
                            .await?,
                            false,
                        ))
                    })
                    .try_collect::<Vec<_>>()
                    .await?,
            )
        };

        Ok(embed)
    }

    pub async fn refresh_schedule(&mut self, ctx: &Context, tx: &DatabaseTransaction) -> BotResult {
        let Some(schedule_channel) = self.schedule_channel_id else {
            return Err(BotError::NoScheduleChannel);
        };

        let embed = self.schedule_embed(tx).await?;

        if let Some(schedule_message) = self.schedule_message_id {
            let res = schedule_channel
                .edit_message(
                    ctx,
                    schedule_message,
                    EditMessage::new()
                        .embed(embed.clone())
                        .button(RefreshButton::create()),
                )
                .await;

            match res {
                Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                    error: DiscordJsonError { code: 10008, .. },
                    ..
                }))) => {}
                _ => return res.map(|_| ()).map_err(Into::into),
            }
        }

        let message = schedule_channel
            .send_message(
                ctx,
                CreateMessage::new()
                    .embed(embed)
                    .button(RefreshButton::create()),
            )
            .await?;

        let mut guild = mem::take(self).into_active_model();
        guild.schedule_message_id = Set(Some(message.id.into()));
        *self = guild.update(tx).await?;

        Ok(())
    }

    pub fn config_embed(&self) -> CreateEmbed {
        CreateEmbed::new()
            .title("⚙️ Configuration")
            .field(
                "RGL Team ID",
                self.rgl_team_id.map_or_else(
                    || "Not set".to_owned(),
                    |id| format!("[`{id}`]({})", id.url()),
                ),
                true,
            )
            .field(
                "na.serveme.tf API Key",
                self.serveme_api_key.as_ref().map_or_else(
                    || "Not set".to_owned(),
                    |key| format!("`{}`", "*".repeat(key.0.len())),
                ),
                true,
            )
            .field(
                "Default Game Format",
                self.game_format
                    .as_ref()
                    .map_or_else(|| "Not set".to_owned(), ToString::to_string),
                true,
            )
            .field(
                "Schedule Channel",
                self.schedule_channel_id
                    .map_or_else(|| "Not set".to_owned(), |c| c.mention().to_string()),
                true,
            )
            .field(
                "Schedule Message",
                self.schedule_message_id
                    .zip(self.schedule_channel_id)
                    .map_or_else(
                        || "Not created".to_owned(),
                        |(m, c)| m.link(*c, Some(*self.id)),
                    ),
                true,
            )
    }
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::game::Entity")]
    Game,
}

impl Related<super::game::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Game.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
