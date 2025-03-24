use std::{
    fmt::{self, Display, Formatter},
    sync::Arc,
};

use migration::SimpleExpr;
use rand::distr::{Alphanumeric, SampleString};
use sea_orm::{
    ActiveValue::Unchanged, DbErr, FromQueryResult, IntoActiveModel, PartialModelTrait,
    QueryResult, entity::prelude::*,
};
use serenity::all::{
    Context, CreateEmbed, FormattedTimestamp, FormattedTimestampStyle, Mentionable,
};
use serenity_commands::BasicOption;
use time::{Duration, OffsetDateTime};

use super::{
    ConnectInfo, GameFormat, MapList, OpponentUserId, ReservationId, ServemeApiKey, TeamGuildId,
    team_guild,
};
use crate::{
    BotResult,
    error::BotError,
    rgl::{RglMatch, RglMatchId, RglSeason, RglTeamId},
    serveme::{
        CreateReservationRequest, EditReservationRequest, FindServersRequest,
        GetReservationRequest, ReservationResponse,
    },
    utils::{OffsetDateTimeEtExt, time_string},
};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "game")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: TeamGuildId,
    #[sea_orm(primary_key, auto_increment = false)]
    pub timestamp: OffsetDateTime,
    pub reservation_id: Option<ReservationId>,
    pub connect_info: Option<ConnectInfo>,
    pub opponent_user_id: Option<OpponentUserId>,
    pub game_format: Option<GameFormat>,
    pub maps: Option<MapList>,
    pub rgl_match_id: Option<RglMatchId>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::team_guild::Entity",
        from = "Column::GuildId",
        to = "super::team_guild::Column::Id",
        on_update = "NoAction",
        on_delete = "NoAction"
    )]
    TeamGuild,
}

impl Related<super::team_guild::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TeamGuild.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(DerivePartialModel)]
#[sea_orm(entity = "Entity", from_query_result)]
struct GameInner {
    guild_id: TeamGuildId,
    timestamp: OffsetDateTime,
    reservation_id: Option<ReservationId>,
    connect_info: Option<ConnectInfo>,
    opponent_user_id: Option<OpponentUserId>,
    game_format: Option<GameFormat>,
    maps: Option<MapList>,
    rgl_match_id: Option<RglMatchId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Game<D = ScrimOrMatch> {
    pub guild_id: TeamGuildId,
    pub timestamp: OffsetDateTime,
    pub server: GameServer,
    pub details: D,
}

impl Game {
    pub async fn embed(&self, guild: &team_guild::Model) -> BotResult<CreateEmbed> {
        let description = self
            .server
            .connect_info_block(guild.serveme_api_key.as_ref())
            .await?;
        let kind = self.details.kind();
        let title = format!(
            "{} **{kind}:** {}",
            kind.emoji(),
            self.timestamp.string_et()
        );
        let mut fields = vec![
            (
                "Date/Time",
                FormattedTimestamp::new(
                    self.timestamp.into(),
                    Some(FormattedTimestampStyle::LongDateTime),
                )
                .to_string(),
                false,
            ),
            ("Map(s)", self.details.maps().await?.list(true), false),
        ];

        match &self.details {
            ScrimOrMatch::Scrim(scrim) => {
                fields.extend([
                    (
                        "Opponent",
                        scrim.opponent_user_id.mention().to_string(),
                        true,
                    ),
                    ("Game Format", scrim.game_format.to_string(), true),
                ]);
            }
            ScrimOrMatch::Match(match_) => {
                let rgl_match = RglMatch::get(match_.rgl_match_id).await?;
                let opponent = rgl_match.opponent_team(guild.rgl_team_id()?)?;

                fields.extend([
                    (
                        "Opponent",
                        format!("[{}]({})", opponent.team_name, opponent.team_id.url()),
                        true,
                    ),
                    (
                        "RGL Match",
                        format!("[{}]({})", rgl_match.match_name, match_.rgl_match_id.url(),),
                        true,
                    ),
                ]);
            }
        }

        if let GameServer::Hosted(reservation_id) = self.server {
            fields.push((
                "Reservation",
                format!("[`{reservation_id}`]({})", reservation_id.url()),
                true,
            ));
        }

        Ok(CreateEmbed::new()
            .title(title)
            .description(description)
            .fields(fields))
    }

    pub async fn schedule_entry(
        &self,
        guild: &team_guild::Model,
        include_connect: bool,
    ) -> BotResult<String> {
        let time = time_string(self.timestamp.time_et());

        let (kind, opponent) = match &self.details {
            ScrimOrMatch::Scrim(scrim) => (
                "Scrim".to_owned(),
                scrim.opponent_user_id.mention().to_string(),
            ),
            ScrimOrMatch::Match(match_) => {
                let rgl_team = guild.rgl_team_id()?;

                let rgl_match = RglMatch::get(match_.rgl_match_id).await?;

                let opponent = rgl_match.opponent_team(rgl_team)?;

                (
                    format!("[Match]({})", match_.rgl_match_id.url()),
                    format!("[{}]({})", opponent.team_name, opponent.team_id.url()),
                )
            }
        };

        let (whitespace, connect_info) = if include_connect {
            (
                ' ',
                self.server
                    .connect_info_block(guild.serveme_api_key.as_ref())
                    .await?,
            )
        } else {
            ('\n', String::new())
        };

        Ok(format!(
            "{} **{time}:** {kind} vs. {opponent} - {}{whitespace}{connect_info}",
            self.details.kind().emoji(),
            self.details.maps().await?.list(false),
        ))
    }
}

impl<D: GameDetails> Game<D> {
    fn start_end_times(&self) -> (OffsetDateTime, OffsetDateTime) {
        let duration = match self.details.kind() {
            GameKind::Scrim => Duration::HOUR,
            GameKind::Match => Duration::hours(2),
        };

        (
            self.timestamp - Duration::minutes(15),
            self.timestamp + duration + Duration::minutes(15),
        )
    }

    pub async fn get_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let reservation_id = self.server.reservation_id()?;

        GetReservationRequest::send(api_key, reservation_id).await
    }

    pub async fn create_reservation(
        &mut self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let (starts_at, ends_at) = self.start_end_times();

        let servers = FindServersRequest { starts_at, ends_at }
            .send(api_key)
            .await?;

        let server_id = servers
            .servers
            .iter()
            .find(|server| {
                server.ip_and_port.starts_with("chi") || server.ip_and_port.starts_with("ks")
            })
            .ok_or(BotError::NoServemeServers)?
            .id;

        let kind = self.details.kind();

        let (first_map, server_config_id) = self
            .details
            .maps()
            .await?
            .server_config(kind, self.details.game_format().await?);

        let prefix = kind.prefix();

        let password = format!(
            "{prefix}.{}",
            Alphanumeric.sample_string(&mut rand::rng(), 8)
        );

        let rcon = format!(
            "{prefix}.rcon.{}",
            Alphanumeric.sample_string(&mut rand::rng(), 32)
        );

        let reservation = CreateReservationRequest {
            starts_at,
            ends_at,
            first_map,
            server_id,
            password,
            rcon,
            server_config_id,
            enable_plugins: true,
            enable_demos_tf: true,
        }
        .send(api_key)
        .await?;

        self.server = GameServer::Hosted(reservation.id);

        Ok(reservation)
    }

    pub async fn edit_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let reservation_id = self.server.reservation_id()?;

        let reservation = self.get_reservation(api_key).await?;

        let (starts_at, ends_at) = self.start_end_times();

        let (first_map, server_config_id) = if starts_at <= reservation.starts_at {
            self.details
                .maps()
                .await?
                .server_config(self.details.kind(), self.details.game_format().await?)
        } else {
            (None, None)
        };

        let starts_at = (starts_at < reservation.starts_at).then_some(starts_at);
        let ends_at = (ends_at > reservation.ends_at).then_some(ends_at);

        let req = EditReservationRequest {
            starts_at,
            ends_at,
            first_map,
            server_config_id,
        };

        if req == EditReservationRequest::default() {
            return Ok(reservation);
        }

        req.send(api_key, reservation_id).await
    }
}

impl<D: GameDetails> TryFrom<Model> for Game<D> {
    type Error = BotError;

    fn try_from(model: Model) -> Result<Self, Self::Error> {
        let server = match (model.reservation_id, model.connect_info) {
            (Some(reservation_id), None) => GameServer::Hosted(reservation_id),
            (None, Some(connect_info)) => GameServer::Joined(connect_info),
            (None, None) => GameServer::Undecided,
            (Some(_), Some(_)) => {
                return Err(BotError::InvalidGameDetails);
            }
        };

        let details = D::from_parts(
            model.opponent_user_id,
            model.game_format,
            model.maps,
            model.rgl_match_id,
        )
        .ok_or(BotError::InvalidGameDetails)?;

        Ok(Self {
            guild_id: model.guild_id,
            timestamp: model.timestamp,
            server,
            details,
        })
    }
}

impl<D: GameDetails> PartialModelTrait for Game<D> {
    fn select_cols<S: sea_orm::SelectColumns>(select: S) -> S {
        Self::select_cols_nested(select, None)
    }

    fn select_cols_nested<S: sea_orm::SelectColumns>(select: S, pre: Option<&str>) -> S {
        GameInner::select_cols_nested(select, pre)
    }
}

impl<D: GameDetails> FromQueryResult for Game<D> {
    fn from_query_result(res: &QueryResult, pre: &str) -> Result<Self, DbErr> {
        let inner = GameInner::from_query_result(res, pre)?;

        let server = match (inner.reservation_id, inner.connect_info) {
            (Some(reservation_id), None) => GameServer::Hosted(reservation_id),
            (None, Some(connect_info)) => GameServer::Joined(connect_info),
            (None, None) => GameServer::Undecided,
            (Some(_), Some(_)) => {
                return Err(DbErr::Custom(
                    "game cannot be both hosted and joined".to_owned(),
                ));
            }
        };

        let details = D::from_parts(
            inner.opponent_user_id,
            inner.game_format,
            inner.maps,
            inner.rgl_match_id,
        )
        .ok_or(DbErr::Custom("game must be either scrim or match".into()))?;

        Ok(Self {
            guild_id: inner.guild_id,
            timestamp: inner.timestamp,
            server,
            details,
        })
    }
}

impl<D: GameDetails> IntoActiveModel<ActiveModel> for Game<D> {
    fn into_active_model(self) -> ActiveModel {
        let mut active_model = ActiveModel {
            guild_id: Unchanged(self.guild_id),
            timestamp: Unchanged(self.timestamp),
            ..Default::default()
        };

        match self.server {
            GameServer::Hosted(reservation_id) => {
                active_model.reservation_id = Unchanged(Some(reservation_id));
                active_model.connect_info = Unchanged(None);
            }
            GameServer::Joined(connect_info) => {
                active_model.reservation_id = Unchanged(None);
                active_model.connect_info = Unchanged(Some(connect_info));
            }
            GameServer::Undecided => {
                active_model.reservation_id = Unchanged(None);
                active_model.connect_info = Unchanged(None);
            }
        }

        let (opponent_user_id, game_format, maps, rgl_match_id) = self.details.into_parts();

        active_model.opponent_user_id = Unchanged(opponent_user_id);
        active_model.game_format = Unchanged(game_format);
        active_model.maps = Unchanged(maps);
        active_model.rgl_match_id = Unchanged(rgl_match_id);

        active_model
    }
}

pub trait GameDetails: Into<ScrimOrMatch> + Sync + Sized {
    fn from_parts(
        opponent_user_id: Option<OpponentUserId>,
        game_format: Option<GameFormat>,
        maps: Option<MapList>,
        rgl_match_id: Option<RglMatchId>,
    ) -> Option<Self>;

    fn into_parts(
        self,
    ) -> (
        Option<OpponentUserId>,
        Option<GameFormat>,
        Option<MapList>,
        Option<RglMatchId>,
    );

    fn filter_expr() -> SimpleExpr;

    fn kind(&self) -> GameKind;

    async fn opponent_string(&self, ctx: &Context, team_id: Option<RglTeamId>)
    -> BotResult<String>;

    async fn maps(&self) -> BotResult<MapList>;

    async fn game_format(&self) -> BotResult<GameFormat>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScrimOrMatch {
    Scrim(Scrim),
    Match(Match),
}

impl GameDetails for ScrimOrMatch {
    fn from_parts(
        opponent_user_id: Option<OpponentUserId>,
        game_format: Option<GameFormat>,
        maps: Option<MapList>,
        rgl_match_id: Option<RglMatchId>,
    ) -> Option<Self> {
        match (opponent_user_id, game_format, maps, rgl_match_id) {
            (Some(opponent_user_id), Some(game_format), Some(maps), None) => {
                Some(Self::Scrim(Scrim {
                    opponent_user_id,
                    game_format,
                    maps,
                }))
            }
            (None, None, None, Some(rgl_match_id)) => Some(Self::Match(Match { rgl_match_id })),
            _ => None,
        }
    }

    fn into_parts(
        self,
    ) -> (
        Option<OpponentUserId>,
        Option<GameFormat>,
        Option<MapList>,
        Option<RglMatchId>,
    ) {
        match self {
            Self::Scrim(scrim) => (
                Some(scrim.opponent_user_id),
                Some(scrim.game_format),
                Some(scrim.maps),
                None,
            ),
            Self::Match(match_) => (None, None, None, Some(match_.rgl_match_id)),
        }
    }

    fn filter_expr() -> SimpleExpr {
        true.into()
    }

    async fn opponent_string(
        &self,
        ctx: &Context,
        team_id: Option<RglTeamId>,
    ) -> BotResult<String> {
        match self {
            Self::Scrim(scrim) => scrim.opponent_string(ctx, team_id).await,
            Self::Match(match_) => match_.opponent_string(ctx, team_id).await,
        }
    }

    fn kind(&self) -> GameKind {
        match self {
            Self::Scrim(_) => GameKind::Scrim,
            Self::Match(_) => GameKind::Match,
        }
    }

    async fn maps(&self) -> BotResult<MapList> {
        match self {
            Self::Scrim(scrim) => scrim.maps().await,
            Self::Match(match_) => match_.maps().await,
        }
    }

    async fn game_format(&self) -> BotResult<GameFormat> {
        match self {
            Self::Scrim(scrim) => scrim.game_format().await,
            Self::Match(match_) => match_.game_format().await,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scrim {
    pub opponent_user_id: OpponentUserId,
    pub game_format: GameFormat,
    pub maps: MapList,
}

impl From<Scrim> for ScrimOrMatch {
    fn from(scrim: Scrim) -> Self {
        Self::Scrim(scrim)
    }
}

impl GameDetails for Scrim {
    fn from_parts(
        opponent_user_id: Option<OpponentUserId>,
        game_format: Option<GameFormat>,
        maps: Option<MapList>,
        rgl_match_id: Option<RglMatchId>,
    ) -> Option<Self> {
        match (opponent_user_id, game_format, maps, rgl_match_id) {
            (Some(opponent_user_id), Some(game_format), Some(maps), None) => Some(Self {
                opponent_user_id,
                game_format,
                maps,
            }),
            _ => None,
        }
    }

    fn into_parts(
        self,
    ) -> (
        Option<OpponentUserId>,
        Option<GameFormat>,
        Option<MapList>,
        Option<RglMatchId>,
    ) {
        (
            Some(self.opponent_user_id),
            Some(self.game_format),
            Some(self.maps),
            None,
        )
    }

    fn filter_expr() -> SimpleExpr {
        Expr::col(Column::RglMatchId).is_null()
    }

    fn kind(&self) -> GameKind {
        GameKind::Scrim
    }

    async fn opponent_string(&self, ctx: &Context, _: Option<RglTeamId>) -> BotResult<String> {
        let user = self.opponent_user_id.to_user(ctx).await?;

        Ok(user.global_name.unwrap_or(user.name))
    }

    async fn maps(&self) -> BotResult<MapList> {
        Ok(self.maps.clone())
    }

    async fn game_format(&self) -> BotResult<GameFormat> {
        Ok(self.game_format)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub rgl_match_id: RglMatchId,
}

impl From<Match> for ScrimOrMatch {
    fn from(match_: Match) -> Self {
        Self::Match(match_)
    }
}

impl GameDetails for Match {
    fn from_parts(
        opponent_user_id: Option<OpponentUserId>,
        game_format: Option<GameFormat>,
        maps: Option<MapList>,
        rgl_match_id: Option<RglMatchId>,
    ) -> Option<Self> {
        match (opponent_user_id, game_format, maps, rgl_match_id) {
            (None, None, None, Some(rgl_match_id)) => Some(Self { rgl_match_id }),
            _ => None,
        }
    }

    fn into_parts(
        self,
    ) -> (
        Option<OpponentUserId>,
        Option<GameFormat>,
        Option<MapList>,
        Option<RglMatchId>,
    ) {
        (None, None, None, Some(self.rgl_match_id))
    }

    fn filter_expr() -> SimpleExpr {
        Expr::col(Column::RglMatchId).is_not_null()
    }

    fn kind(&self) -> GameKind {
        GameKind::Match
    }

    async fn opponent_string(&self, _: &Context, team_id: Option<RglTeamId>) -> BotResult<String> {
        let rgl_match = RglMatch::get(self.rgl_match_id).await?;

        let rgl_team = rgl_match.opponent_team(team_id.ok_or(BotError::NoRglTeam)?)?;

        Ok(rgl_team.team_name)
    }

    async fn maps(&self) -> BotResult<MapList> {
        let rgl_match = RglMatch::get(self.rgl_match_id).await?;

        Ok(MapList(
            rgl_match.maps.iter().map(|m| m.map_name.clone()).collect(),
        ))
    }

    async fn game_format(&self) -> BotResult<GameFormat> {
        let rgl_match = RglMatch::get(self.rgl_match_id).await?;
        let season = RglSeason::get(rgl_match.season_id).await?;
        Ok(season.format_name)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameKind {
    Scrim,
    Match,
}

impl GameKind {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Scrim => "scrim",
            Self::Match => "match",
        }
    }

    const fn emoji(self) -> char {
        match self {
            Self::Scrim => '🎯',
            Self::Match => '🏆',
        }
    }
}

impl Display for GameKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scrim => write!(f, "Scrim"),
            Self::Match => write!(f, "Match"),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum GameServer {
    Hosted(ReservationId),
    Joined(ConnectInfo),
    #[default]
    Undecided,
}

impl GameServer {
    pub const fn is_hosted(&self) -> bool {
        matches!(self, Self::Hosted(_))
    }

    pub const fn is_joined(&self) -> bool {
        matches!(self, Self::Joined(_))
    }

    pub const fn reservation_id(&self) -> BotResult<ReservationId> {
        match self {
            Self::Hosted(reservation_id) => Ok(*reservation_id),
            _ => Err(BotError::GameNotHosted),
        }
    }

    pub async fn connect_info_block(
        &self,
        serveme_api_key: Option<&ServemeApiKey>,
    ) -> BotResult<String> {
        let conn = match (self, serveme_api_key) {
            (Self::Hosted(reservation_id), Some(api_key)) => Ok(Some(
                GetReservationRequest::send(api_key, *reservation_id)
                    .await?
                    .connect_info(),
            )),
            (Self::Hosted(_), None) => Err(BotError::NoServemeApiKey),
            (Self::Joined(connect_info), _) => Ok(Some(connect_info.clone())),
            (Self::Undecided, _) => Ok(None),
        }?;

        Ok(conn.map_or_else(
            || "```\nNo connect info\n```".to_owned(),
            |c| c.code_block(),
        ))
    }
}

impl BasicOption for GameServer {
    type Partial = String;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> serenity::all::CreateCommandOption {
        Option::<String>::create_option(name, description)
    }

    fn from_value(
        value: Option<&serenity::all::CommandDataOptionValue>,
    ) -> serenity_commands::Result<Self> {
        Option::<String>::from_value(value)?.map_or_else(
            || Ok(Self::Undecided),
            |value| {
                (value.parse::<ReservationId>().map(GameServer::Hosted))
                    .or_else(|_| value.parse::<ConnectInfo>().map(GameServer::Joined))
                    .map_err(|_| {
                        serenity_commands::Error::Custom(Box::new(BotError::InvalidGameServer))
                    })
            },
        )
    }
}
