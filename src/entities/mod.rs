use std::{
    collections::BTreeMap,
    convert::Infallible,
    fmt::{self, Display, Formatter},
    ops::{Deref, DerefMut},
    str::FromStr,
    sync::LazyLock,
};

use game::GameKind;
use regex::Regex;
use sea_orm::{
    ColIdx, DbErr, DeriveActiveEnum, DeriveValueType, EnumIter, QueryResult, TryFromU64,
    TryGetError, TryGetable, Value,
    sea_query::{ArrayType, ColumnType, Nullable, ValueType, ValueTypeErr},
};
use serde::{Deserialize, Serialize};
use serenity::all::{
    AutocompleteChoice, ChannelId, ChannelType, CommandDataOptionValue, CreateCommandOption,
    GuildId, MessageId, UserId,
};
use serenity_commands::BasicOption;

use crate::error::BotError;

pub mod game;
pub mod team_guild;

macro_rules! discord_id {
    (? $Id:ident($DiscordId:ident)) => {
        discord_id!($Id($DiscordId));

        impl sea_orm::sea_query::Nullable for $Id {
            fn null() -> Value {
                <i64 as sea_orm::sea_query::Nullable>::null()
            }
        }
    };
    ($Id:ident($DiscordId:ident)) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
        pub struct $Id(pub $DiscordId);

        impl std::ops::Deref for $Id {
            type Target = $DiscordId;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<i64> for $Id {
            fn from(source: i64) -> Self {
                $Id($DiscordId::new(source as _))
            }
        }

        impl From<$Id> for i64 {
            fn from(source: $Id) -> Self {
                source.get() as _
            }
        }

        impl From<$DiscordId> for $Id {
            fn from(source: $DiscordId) -> Self {
                $Id(source)
            }
        }

        impl From<$Id> for $DiscordId {
            fn from(source: $Id) -> Self {
                source.0
            }
        }

        impl From<$Id> for Value {
            fn from(source: $Id) -> Self {
                i64::from(source).into()
            }
        }

        impl sea_orm::TryGetable for $Id {
            fn try_get_by<I: sea_orm::ColIdx>(
                res: &QueryResult,
                idx: I,
            ) -> Result<Self, sea_orm::TryGetError> {
                <i64 as sea_orm::TryGetable>::try_get_by(res, idx).map($Id::from)
            }
        }

        impl sea_orm::sea_query::ValueType for $Id {
            fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
                <i64 as sea_orm::sea_query::ValueType>::try_from(v).map($Id::from)
            }

            fn type_name() -> String {
                stringify!($Id).to_owned()
            }

            fn array_type() -> sea_orm::sea_query::ArrayType {
                sea_orm::sea_query::ArrayType::BigInt
            }

            fn column_type() -> sea_orm::sea_query::ColumnType {
                sea_orm::sea_query::ColumnType::BigInteger
            }
        }

        impl std::fmt::Display for $Id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

discord_id!(TeamGuildId(GuildId));
discord_id!(?ScheduleChannelId(ChannelId));
discord_id!(?ScheduleMessageId(MessageId));
discord_id!(?OpponentUserId(UserId));

impl TryFromU64 for TeamGuildId {
    fn try_from_u64(n: u64) -> Result<Self, DbErr> {
        i64::try_from_u64(n).map(Into::into)
    }
}

impl BasicOption for ScheduleChannelId {
    type Partial = ChannelId;

    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> serenity::all::CreateCommandOption {
        ChannelId::create_option(name, description).channel_types(vec![ChannelType::Text])
    }

    fn from_value(
        value: Option<&serenity::all::CommandDataOptionValue>,
    ) -> serenity_commands::Result<Self> {
        ChannelId::from_value(value).map(Self)
    }
}

#[derive(
    Clone, Debug, Copy, PartialEq, Eq, Hash, EnumIter, BasicOption, DeriveActiveEnum, Deserialize,
)]
#[sea_orm(rs_type = "i16", db_type = "SmallInteger")]
#[option(option_type = "string")]
#[serde(rename_all = "PascalCase")]
pub enum GameFormat {
    Sixes = 6,
    Highlander = 9,
}

impl GameFormat {
    pub const fn rgl_id(self) -> u8 {
        match self {
            Self::Sixes => 40,
            Self::Highlander => 24,
        }
    }
}

impl Display for GameFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sixes => f.write_str("Sixes"),
            Self::Highlander => f.write_str("Highlander"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, BasicOption, DeriveValueType, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServemeApiKey(pub String);

impl ServemeApiKey {
    pub fn auth_header(&self) -> String {
        format!("Token token={self}")
    }
}

impl Display for ServemeApiKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Nullable for ServemeApiKey {
    fn null() -> Value {
        String::null()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectInfo {
    pub ip_and_port: String,
    pub password: String,
}

impl ConnectInfo {
    pub fn code_block(&self) -> String {
        format!("```\n{self}\n```")
    }
}

impl FromStr for ConnectInfo {
    type Err = BotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        static IP_AND_PORT: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r#"^\s*connect\s+(?:(.*?)|"(.*)")\s*;\s*password\s+(?:(.*?)|"(.*)")\s*"#)
                .unwrap()
        });

        let captures = IP_AND_PORT
            .captures(s)
            .ok_or(BotError::InvalidConnectInfo)?;

        let ip_and_port = captures
            .get(1)
            .or_else(|| captures.get(2))
            .ok_or(BotError::InvalidConnectInfo)?;

        let password = captures
            .get(3)
            .or_else(|| captures.get(4))
            .ok_or(BotError::InvalidConnectInfo)?;

        Ok(Self {
            ip_and_port: ip_and_port.as_str().to_owned(),
            password: password.as_str().to_owned(),
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

impl TryGetable for ConnectInfo {
    fn try_get_by<I: sea_orm::ColIdx>(
        res: &QueryResult,
        idx: I,
    ) -> Result<Self, sea_orm::TryGetError> {
        <String as TryGetable>::try_get_by(res, idx).and_then(|s| {
            s.parse::<Self>().map_err(|e| {
                TryGetError::DbErr(DbErr::TryIntoErr {
                    from: "String",
                    into: "ConnectInfo",
                    source: e.into(),
                })
            })
        })
    }
}

impl From<ConnectInfo> for Value {
    fn from(source: ConnectInfo) -> Self {
        source.to_string().into()
    }
}

impl ValueType for ConnectInfo {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        <String as ValueType>::try_from(v).and_then(|s| s.parse::<Self>().map_err(|_| ValueTypeErr))
    }

    fn type_name() -> String {
        stringify!(ConnectInfo).to_owned()
    }

    fn column_type() -> ColumnType {
        <String as ValueType>::column_type()
    }

    fn array_type() -> ArrayType {
        <String as ValueType>::array_type()
    }
}

impl Nullable for ConnectInfo {
    fn null() -> Value {
        String::null()
    }
}

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

impl FromStr for ReservationId {
    type Err = BotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<i32>()
            .map(Self)
            .map_err(|_| BotError::InvalidReservationId)
    }
}

impl Nullable for ReservationId {
    fn null() -> Value {
        Value::Int(None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MapList(pub Vec<Map>);

impl MapList {
    pub fn autocomplete_choice<'a>(this: impl IntoIterator<Item = &'a Map>) -> AutocompleteChoice {
        let maps = this.into_iter().map(Map::as_str).collect::<Vec<_>>();

        let name = maps.join(", ");
        let value = maps.join(",");

        AutocompleteChoice::new(name, value)
    }

    pub fn server_config(&self, kind: GameKind, format: GameFormat) -> (Option<Map>, Option<u32>) {
        self.first()
            .and_then(|m| Some((m.clone(), m.config(kind, format)?.id)))
            .unzip()
    }

    pub fn list(&self, full: bool) -> String {
        if self.is_empty() {
            "Maps not set".to_owned()
        } else if full {
            self.iter()
                .map(|m| format!("`{m}`"))
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            self.iter()
                .map(Map::short_map_name)
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

impl Display for MapList {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.iter()
            .map(Map::as_str)
            .collect::<Vec<_>>()
            .join(", ")
            .fmt(f)
    }
}

impl FromStr for MapList {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            s.split_whitespace()
                .flat_map(|s| s.split(','))
                .flat_map(|s| s.split('/'))
                .filter(|s| !s.is_empty())
                .map(Map::new)
                .collect(),
        ))
    }
}

impl Deref for MapList {
    type Target = Vec<Map>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for MapList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl BasicOption for MapList {
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

impl From<MapList> for Value {
    fn from(source: MapList) -> Self {
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

impl TryGetable for MapList {
    fn try_get_by<I: ColIdx>(res: &QueryResult, idx: I) -> Result<Self, TryGetError> {
        <Vec<String> as TryGetable>::try_get_by(res, idx)
            .map(|v| Self(v.into_iter().map(Map).collect()))
    }
}

impl ValueType for MapList {
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

impl Nullable for MapList {
    fn null() -> Value {
        <Vec<String> as Nullable>::null()
    }
}

static SIXES_MAPS: LazyLock<BTreeMap<Map, &'static str>> = LazyLock::new(|| {
    [
        ("cp_gullywash_f9", "Gullywash"),
        ("cp_metalworks_f5", "Metalworks"),
        ("cp_process_f12", "Process"),
        ("cp_snakewater_final1", "Snakewater"),
        ("cp_sultry_b8a", "Sultry"),
        ("cp_sunshine", "Sunshine"),
        ("koth_bagel_rc10", "Bagel"),
        ("koth_clearcut_b17", "Clearcut"),
        ("cp_granary_pro_rc8", "Granary"),
        ("koth_product_final", "Product"),
    ]
    .into_iter()
    .map(|(map, title)| (Map::new(map), title))
    .collect()
});

static HL_MAPS: LazyLock<BTreeMap<Map, &'static str>> = LazyLock::new(|| {
    [
        ("cp_steel_f12", "Steel"),
        ("koth_ashville_final1", "Ashville"),
        ("koth_lakeside_f5", "Lakeside"),
        ("koth_product_final", "Product"),
        ("pl_swiftwater_final1", "Swiftwater"),
        ("pl_upward_f12", "Upward"),
        ("pl_vigil_rc10", "Vigil"),
    ]
    .into_iter()
    .map(|(map, title)| (Map::new(map), title))
    .collect()
});

static ALL_MAPS: LazyLock<BTreeMap<Map, &'static str>> = LazyLock::new(|| {
    SIXES_MAPS
        .iter()
        .map(|(map, title)| (map.clone(), *title))
        .chain(HL_MAPS.iter().map(|(map, title)| (map.clone(), *title)))
        .collect()
});

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

    pub fn short_map_name(&self) -> String {
        ALL_MAPS
            .get(self)
            .map_or_else(|| format!("`{self}`"), |&title| title.to_owned())
    }

    pub fn config(&self, kind: GameKind, format: GameFormat) -> Option<ServerConfig> {
        match (kind, format) {
            (GameKind::Scrim, GameFormat::Sixes) => {
                if self.0.starts_with("cp_") {
                    Some(ServerConfig::SCRIM_6S_5CP)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::SCRIM_6S_KOTH)
                } else {
                    None
                }
            }
            (GameKind::Scrim, GameFormat::Highlander) => {
                if self.0.starts_with("pl_") || self.0.starts_with("cp_") {
                    Some(ServerConfig::HL_STOPWATCH)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::SCRIM_HL_KOTH)
                } else {
                    None
                }
            }
            (GameKind::Match, GameFormat::Sixes) => {
                if self.0.starts_with("cp_") {
                    Some(ServerConfig::MATCH_6S_5CP)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::MATCH_6S_KOTH)
                } else {
                    None
                }
            }
            (GameKind::Match, GameFormat::Highlander) => {
                if self.0.starts_with("pl_") || self.0.starts_with("cp_") {
                    Some(ServerConfig::HL_STOPWATCH)
                } else if self.0.starts_with("koth_") {
                    Some(ServerConfig::MATCH_HL_KOTH)
                } else {
                    None
                }
            }
        }
    }

    pub fn official_maps(game_format: Option<GameFormat>) -> &'static BTreeMap<Self, &'static str> {
        match game_format {
            Some(GameFormat::Sixes) => &SIXES_MAPS,
            Some(GameFormat::Highlander) => &HL_MAPS,
            None => &ALL_MAPS,
        }
    }

    pub fn is_official(&self, game_format: Option<GameFormat>) -> bool {
        Self::official_maps(game_format).contains_key(self)
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
    const HL_STOPWATCH: Self = Self::new("rgl_HL_stopwatch", 55);
    const MATCH_6S_5CP: Self = Self::new("rgl_6s_5cp_match_pro", 109);
    const MATCH_6S_KOTH: Self = Self::new("rgl_6s_koth_pro", 110);
    const MATCH_HL_KOTH: Self = Self::new("rgl_HL_koth", 53);
    const SCRIM_6S_5CP: Self = Self::new("rgl_6s_5cp_scrim", 69);
    const SCRIM_6S_KOTH: Self = Self::new("rgl_6s_koth_scrim", 113);
    const SCRIM_HL_KOTH: Self = Self::new("rgl_HL_koth_bo5", 54);

    const fn new(name: &'static str, id: u32) -> Self {
        Self { name, id }
    }
}
