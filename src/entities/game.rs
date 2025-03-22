use std::{
    cmp::Ordering,
    collections::HashSet,
    convert::Infallible,
    fmt::{self, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::{Arc, LazyLock},
};

use rand::distr::{Alphanumeric, SampleString};
use sea_orm::{
    ColIdx, TryGetError, TryGetable,
    entity::prelude::*,
    sea_query::{ArrayType, Nullable, ValueType, ValueTypeErr},
};
use serde::{Deserialize, Serialize};
use serenity::all::{
    CommandDataOptionValue, CreateCommandOption, CreateEmbed, FormattedTimestamp,
    FormattedTimestampStyle, Mentionable, MessageId, ScheduledEventId, UserId,
};
use serenity_commands::BasicOption;
use time::{Duration, OffsetDateTime};

use super::{
    discord_id,
    team_guild::{GameFormat, ServemeApiKey, TeamGuildId},
};
use crate::{
    BotResult,
    error::BotError,
    serveme::{
        CreateReservationRequest, DeleteReservationRequest, EditReservationRequest,
        FindServersRequest, GetReservationRequest, ReservationResponse,
    },
    utils::{OffsetDateTimeEtExt, time_string},
};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "game")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: TeamGuildId,
    #[sea_orm(primary_key, auto_increment = false)]
    pub timestamp: OffsetDateTime,
    pub game_format: GameFormat,
    pub opponent_user_id: OpponentUserId,
    pub reservation_id: Option<ReservationId>,
    pub server_ip_and_port: Option<String>,
    pub server_password: Option<String>,
    pub maps: Option<Maps>,
    pub rgl_match_id: Option<RglMatchId>,
}

impl Model {
    fn start_end_times(&self) -> (OffsetDateTime, OffsetDateTime) {
        (
            self.timestamp - Duration::minutes(15),
            self.timestamp + Duration::HOUR + Duration::minutes(15),
        )
    }

    pub fn first_map(&self) -> Option<Map> {
        self.maps.as_ref().and_then(|maps| maps.0.first().cloned())
    }

    pub fn first_config(&self, format: GameFormat) -> Option<ServerConfig> {
        self.first_map().and_then(|map| map.config(format))
    }

    pub async fn embed(&self, api_key: Option<&ServemeApiKey>) -> BotResult<CreateEmbed> {
        let game_info = self.game_info()?;
        let embed = CreateEmbed::new().title(game_info.title(self.timestamp));

        Ok(match self.game_info()? {
            GameInfo::Official(_) => todo!(),
            GameInfo::Scrim { server, maps } => {
                let embed = embed
                    .description(server.connect_info_block(api_key).await?)
                    .field(
                        "Date/Time",
                        FormattedTimestamp::new(
                            self.timestamp.into(),
                            Some(FormattedTimestampStyle::LongDateTime),
                        )
                        .to_string(),
                        false,
                    )
                    .field("Maps", maps.list(), false)
                    .field(
                        "Opponent",
                        self.opponent_user_id.mention().to_string(),
                        true,
                    )
                    .field("Game Format", self.game_format.to_string(), true);

                if let Some(reservation_id) = self.reservation_id {
                    embed.field(
                        "Reservation",
                        format!("[`{}`]({})", reservation_id, reservation_id.url()),
                        true,
                    )
                } else {
                    embed
                }
            }
        })
    }

    pub async fn schedule_entry(
        &self,
        api_key: Option<&ServemeApiKey>,
        include_connect: bool,
    ) -> BotResult<String> {
        let game_info = self.game_info()?;
        let title = game_info.schedule_entry(self.timestamp, self.opponent_user_id);

        match game_info {
            GameInfo::Official(_) => todo!(),
            GameInfo::Scrim { server, .. } => {
                let connect_info = if include_connect {
                    server.connect_info_block(api_key).await?
                } else {
                    "\n".into()
                };

                Ok(format!("{title} {connect_info}"))
            }
        }
    }

    fn game_info(&self) -> BotResult<GameInfo> {
        match (
            self.rgl_match_id,
            &self.maps,
            &self.reservation_id,
            &self.server_ip_and_port,
            &self.server_password,
        ) {
            (Some(match_id), None, None, None, None) => Ok(GameInfo::Official(match_id)),
            (None, Some(maps), Some(reservation_id), None, None) => Ok(GameInfo::Scrim {
                maps: maps.clone(),
                server: Server::Hosted(*reservation_id),
            }),
            (None, Some(maps), None, Some(ip_and_port), Some(password)) => Ok(GameInfo::Scrim {
                maps: maps.clone(),
                server: Server::Joined(ConnectInfo {
                    ip_and_port: ip_and_port.clone(),
                    password: password.clone(),
                }),
            }),
            (None, Some(maps), None, None, None) => Ok(GameInfo::Scrim {
                maps: maps.clone(),
                server: Server::Undecided,
            }),
            (Some(_), _, _, _, _) | (None, None, _, _, _) => Err(BotError::InvalidMatchInfo),
            (None, Some(_), _, _, _) => Err(BotError::InvalidServerInfo),
        }
    }

    pub async fn get_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let Some(reservation_id) = self.reservation_id else {
            return Err(BotError::NoReservationId);
        };

        GetReservationRequest::send(api_key, reservation_id).await
    }

    pub async fn create_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let (starts_at, ends_at) = self.start_end_times();

        let servers = FindServersRequest { starts_at, ends_at }
            .send(api_key)
            .await?;

        let server = servers
            .servers
            .iter()
            .find(|server| {
                server.ip_and_port.starts_with("chi") || server.ip_and_port.starts_with("ks")
            })
            .ok_or(BotError::NoServemeServers)?;

        let first_map = self.first_map();
        let server_config_id = self.first_config(self.game_format).map(|c| c.id);

        let password = format!("scrim.{}", Alphanumeric.sample_string(&mut rand::rng(), 8));

        let rcon = format!(
            "scrim.rcon.{}",
            Alphanumeric.sample_string(&mut rand::rng(), 32)
        );

        CreateReservationRequest {
            starts_at,
            ends_at,
            first_map,
            server_id: server.id,
            password,
            rcon,
            server_config_id,
            enable_plugins: true,
            enable_demos_tf: true,
        }
        .send(api_key)
        .await
    }

    pub async fn edit_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Arc<ReservationResponse>> {
        let Some(reservation_id) = self.reservation_id else {
            return Err(BotError::NoReservationId);
        };

        let reservation = GetReservationRequest::send(api_key, reservation_id).await?;

        let (starts_at, ends_at) = self.start_end_times();

        let (first_map, server_config_id) = (starts_at <= reservation.starts_at)
            .then(|| Some((self.first_map()?, self.first_config(self.game_format)?.id)))
            .flatten()
            .unzip();

        let starts_at = starts_at.min(reservation.starts_at);
        let ends_at = ends_at.max(reservation.ends_at);

        EditReservationRequest {
            starts_at: Some(starts_at),
            ends_at: Some(ends_at),
            first_map,
            server_config_id,
        }
        .send(api_key, reservation_id)
        .await
    }

    pub async fn delete_reservation(
        &self,
        api_key: &ServemeApiKey,
    ) -> BotResult<Option<ReservationResponse>> {
        let Some(reservation_id) = self.reservation_id else {
            return Err(BotError::NoReservationId);
        };

        DeleteReservationRequest::send(api_key, reservation_id).await
    }
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
    Guilds,
}

impl Related<super::team_guild::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Guilds.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

discord_id!(MatchEventId(ScheduledEventId));
discord_id!(?MatchMessageId(MessageId));
discord_id!(OpponentUserId(UserId));

#[derive(
    Copy,
    Clone,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    BasicOption,
    DeriveValueType,
    Serialize,
    Deserialize,
)]
#[serde(transparent)]
pub struct ReservationId(pub i32);

impl ReservationId {
    pub fn url(self) -> String {
        format!("https://na.serveme.tf/reservations/{self}")
    }
}

impl Display for ReservationId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Nullable for ReservationId {
    fn null() -> Value {
        Value::Int(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Maps(pub Vec<Map>);

impl Maps {
    pub fn list(&self) -> String {
        if self.is_empty() {
            "Maps not set".to_owned()
        } else {
            self.iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    }

    pub fn autocomplete_value(&self) -> String {
        self.iter().map(Map::as_str).collect::<Vec<_>>().join(",")
    }
}

impl Display for Maps {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.iter()
            .map(Map::as_str)
            .collect::<Vec<_>>()
            .join(", ")
            .fmt(f)
    }
}

impl FromStr for Maps {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split(',')
                .filter_map(|s| {
                    let s = s.trim();
                    (!s.is_empty()).then(|| Map::new(s))
                })
                .collect(),
        ))
    }
}

impl Deref for Maps {
    type Target = Vec<Map>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Maps {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl BasicOption for Maps {
    type Partial = String;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(value: Option<&CommandDataOptionValue>) -> serenity_commands::Result<Self> {
        let value = String::from_value(value)?;

        Ok(value.parse::<Self>().unwrap())
    }
}

impl From<Maps> for Value {
    fn from(source: Maps) -> Self {
        Self::Array(
            ArrayType::String,
            Some(Box::new(
                source
                    .0
                    .into_iter()
                    .map(|s| Self::String(Some(Box::new(s.0))))
                    .collect(),
            )),
        )
    }
}

impl TryGetable for Maps {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        <Vec<String> as sea_orm::TryGetable>::try_get_by(res, idx)
            .map(|v| Self(v.into_iter().map(Map).collect()))
    }
}

impl ValueType for Maps {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <Vec<String> as ValueType>::try_from(v).map(|v| Self(v.into_iter().map(Map).collect()))
    }

    fn type_name() -> String {
        stringify!(Maps).to_owned()
    }

    fn array_type() -> ArrayType {
        <Vec<String> as ValueType>::array_type()
    }

    fn column_type() -> ColumnType {
        <Vec<String> as ValueType>::column_type()
    }
}

impl Nullable for Maps {
    fn null() -> Value {
        <Vec<String> as Nullable>::null()
    }
}

static SIXES_MAPS: LazyLock<HashSet<Map>> = LazyLock::new(|| {
    [
        "cp_gullywash_f9",
        "cp_metalworks_f5",
        "cp_process_f12",
        "cp_snakewater_final1",
        "cp_sultry_b8a",
        "cp_sunshine",
        "koth_bagel_rc10",
        "koth_clearcut_b17",
        "cp_granary_pro_rc8",
        "koth_product_final",
    ]
    .into_iter()
    .map(Map::new)
    .collect()
});

static HL_MAPS: LazyLock<HashSet<Map>> = LazyLock::new(|| {
    [
        "cp_steel_f12",
        "koth_ashville_final1",
        "koth_lakeside_f5",
        "koth_product_final",
        "pl_swiftwater_final1",
        "pl_upward_f12",
        "pl_vigil_rc10",
    ]
    .into_iter()
    .map(Map::new)
    .collect()
});

static ALL_MAPS: LazyLock<HashSet<Map>> =
    LazyLock::new(|| SIXES_MAPS.union(&HL_MAPS).cloned().collect());

#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, DeriveValueType, Serialize, Deserialize,
)]
#[serde(transparent)]
pub struct Map(pub String);

impl Map {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn as_str(&self) -> &str {
        self
    }

    pub fn config(&self, format: GameFormat) -> Option<ServerConfig> {
        match format {
            GameFormat::Sixes => {
                if self.0.starts_with("cp_") {
                    Some(ServerConfig::SIXES_5CP)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::SIXES_KOTH)
                } else {
                    None
                }
            }
            GameFormat::Highlander => {
                if self.0.starts_with("pl_") || self.0.starts_with("cp_") {
                    Some(ServerConfig::HL_STOPWATCH)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::HL_KOTH)
                } else {
                    None
                }
            }
        }
    }

    pub fn is_official(&self, game_format: Option<GameFormat>) -> bool {
        match game_format {
            Some(GameFormat::Sixes) => &SIXES_MAPS,
            Some(GameFormat::Highlander) => &HL_MAPS,
            None => &ALL_MAPS,
        }
        .contains(self)
    }

    pub fn cmp_with_format(&self, other: &Self, game_format: Option<GameFormat>) -> Ordering {
        match (
            self.is_official(game_format),
            other.is_official(game_format),
        ) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) | (false, false) => {
                self.to_ascii_lowercase().cmp(&other.to_ascii_lowercase())
            }
        }
    }
}

impl Deref for Map {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0.as_str()
    }
}

impl BasicOption for Map {
    type Partial = String;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(value: Option<&CommandDataOptionValue>) -> serenity_commands::Result<Self> {
        String::from_value(value).map(Self)
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServerConfig {
    pub name: &'static str,
    pub id: u32,
}

impl ServerConfig {
    const HL_KOTH: Self = Self::new("rgl_HL_koth_bo5", 54);
    const HL_STOPWATCH: Self = Self::new("rgl_HL_stopwatch", 55);
    const SIXES_5CP: Self = Self::new("rgl_6s_5cp_scrim", 69);
    const SIXES_KOTH: Self = Self::new("rgl_6s_koth_scrim", 113);

    const fn new(name: &'static str, id: u32) -> Self {
        Self { name, id }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, DeriveValueType)]
pub struct RglMatchId(pub i32);

impl Display for RglMatchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Nullable for RglMatchId {
    fn null() -> Value {
        i32::null()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameInfo {
    Official(RglMatchId),
    Scrim { maps: Maps, server: Server },
}

impl GameInfo {
    fn title(&self, timestamp: OffsetDateTime) -> String {
        let timestamp = timestamp.et_short_date();

        match self {
            Self::Official(_) => format!("**Official Match:** {timestamp}"),
            Self::Scrim { .. } => format!("**Scrim:** {timestamp}"),
        }
    }

    fn schedule_entry(&self, timestamp: OffsetDateTime, opponent: OpponentUserId) -> String {
        let timestamp = timestamp.to_et_offset();
        let time = time_string(timestamp.time());

        match self {
            Self::Official(_) => {
                format!("**{time}:** Official Match vs. {}", opponent.mention())
            }
            Self::Scrim { maps, .. } => {
                format!(
                    "**{time}:** Scrim vs. {} ({})",
                    opponent.mention(),
                    maps.list()
                )
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Server {
    Hosted(ReservationId),
    Joined(ConnectInfo),
    Undecided,
}

impl Server {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectInfo {
    pub ip_and_port: String,
    pub password: String,
}

impl BasicOption for ConnectInfo {
    type Partial = String;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(value: Option<&CommandDataOptionValue>) -> serenity_commands::Result<Self> {
        let value = String::from_value(value)?;

        value
            .parse()
            .map_err(|err| serenity_commands::Error::Custom(Box::new(err)))
    }
}

impl ConnectInfo {
    pub fn code_block(&self) -> String {
        format!("```\n{self}\n```")
    }
}

impl FromStr for ConnectInfo {
    type Err = BotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (mut ip_and_port, s) = s
            .trim()
            .strip_prefix("connect ")
            .ok_or(BotError::InvalidConnectCommand)?
            .split_once(';')
            .ok_or(BotError::InvalidConnectCommand)?;

        ip_and_port = ip_and_port.trim();

        if ip_and_port.starts_with('"') && ip_and_port.ends_with('"') && ip_and_port.len() >= 2 {
            ip_and_port = &ip_and_port[1..ip_and_port.len() - 1];
        }

        let mut password = s
            .trim_start()
            .strip_prefix("password ")
            .ok_or(BotError::InvalidConnectCommand)?
            .trim_start();

        if password.starts_with('"') && password.ends_with('"') && password.len() >= 2 {
            password = &password[1..password.len() - 1];
        }

        Ok(Self {
            ip_and_port: ip_and_port.to_owned(),
            password: password.to_owned(),
        })
    }
}

impl Display for ConnectInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "connect {}; password \"{}\"",
            self.ip_and_port, self.password
        )
    }
}
