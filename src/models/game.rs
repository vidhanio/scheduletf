use std::fmt::{Display, Formatter, Result};

use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, ChannelId, CreateActionRow, CreateButton, CreateCommandOption, CreateEmbed,
    CreateMessage, CreateScheduledEvent, EditMessage, EditScheduledEvent, FormattedTimestamp,
    FormattedTimestampStyle, GuildId, Mentionable, MessageId, ScheduledEventId, ScheduledEventType,
    UserId,
};
use serenity_commands::BasicOption;
use sqlx::{query, PgExecutor};
use time::{macros::time, Duration, OffsetDateTime, Weekday};

use crate::{
    error::BotError,
    serveme::{ConnectInfo, EditReservationRequest, FindServersResponse, ReservationRequest},
    utils::{embed, ScrimOffsetExtension},
    BotResult,
};

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
pub enum NextDay {
    Today,
    NextSunday,
    NextMonday,
    NextTuesday,
    NextWednesday,
    NextThursday,
    NextFriday,
    NextSaturday,
}

impl NextDay {
    pub fn to_datetime(self, time: Time) -> OffsetDateTime {
        let now = OffsetDateTime::now_scrim();

        let today_date = now.date();

        let date = match self {
            Self::Today => today_date,
            Self::NextSunday => today_date.next_occurrence(Weekday::Sunday),
            Self::NextMonday => today_date.next_occurrence(Weekday::Monday),
            Self::NextTuesday => today_date.next_occurrence(Weekday::Tuesday),
            Self::NextWednesday => today_date.next_occurrence(Weekday::Wednesday),
            Self::NextThursday => today_date.next_occurrence(Weekday::Thursday),
            Self::NextFriday => today_date.next_occurrence(Weekday::Friday),
            Self::NextSaturday => today_date.next_occurrence(Weekday::Saturday),
        };

        OffsetDateTime::new_utc(date, time.into()).replace_with_scrim_offset()
    }
}

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
#[allow(clippy::enum_variant_names)]
pub enum Time {
    #[choice(name = "6:30 PM ET")]
    SixThirty,

    #[choice(name = "7:30 PM ET")]
    SevenThirty,

    #[choice(name = "8:30 PM ET")]
    EightThirty,

    #[choice(name = "9:30 PM ET")]
    NineThirty,

    #[choice(name = "10:30 PM ET")]
    TenThirty,

    #[choice(name = "11:30 PM ET")]
    ElevenThirty,
}

impl From<Time> for time::Time {
    fn from(time: Time) -> Self {
        match time {
            Time::SixThirty => time!(18:30),
            Time::SevenThirty => time!(19:30),
            Time::EightThirty => time!(20:30),
            Time::NineThirty => time!(21:30),
            Time::TenThirty => time!(22:30),
            Time::ElevenThirty => time!(23:30),
        }
    }
}

#[derive(Debug)]
pub struct DbScrim {
    #[allow(dead_code)]
    pub guild_id: i64,
    pub timestamp: OffsetDateTime,
    pub event_id: i64,
    pub message_id: Option<i64>,
    pub opponent_user_id: i64,
    pub reservation_id: Option<i32>,
    pub server_ip_and_port: Option<String>,
    pub server_password: Option<String>,
    pub map_1: Option<String>,
    pub map_2: Option<String>,
    pub rgl_match_id: Option<i32>,
}

impl DbScrim {
    pub async fn insert(&self, c: impl PgExecutor<'_>) -> sqlx::Result<()> {
        query!(
            r#"INSERT INTO scrims (
                guild_id,
                timestamp,
                event_id,
                message_id,
                opponent_user_id,
                reservation_id,
                server_ip_and_port,
                server_password,
                map_1,
                map_2,

        Ok(())
    }
}

impl From<DbScrim> for Scrim {
    fn from(db: DbScrim) -> Self {
        let game_format = if db.game_format == 6 {
            GameFormat::Sixes
        } else {
            GameFormat::Highlander
        };

        let opponent = UserId::new(db.opponent_user_id as _);

        let server_info = if let Some((ip_and_port, password)) = db.ip_and_port.zip(db.password) {
            Some(
                if let Some((reservation_id, rcon)) = db.reservation_id.zip(db.serveme_rcon) {
                    ServerInfo::Serveme {
                        reservation_id: reservation_id as _,
                        rcon_password: rcon,
                        connect_info: ConnectInfo {
                            ip_and_port,
                            password,
                        },
                    }
                } else {
                    ServerInfo::ExternalServer(ConnectInfo {
                        ip_and_port,
                        password,
                    })
                },
            )
        } else {
            None
        };

        Self {
            guild_id: GuildId::new(db.guild_id as _),
            timestamp: db.timestamp,
            opponent_user_id: opponent,
            game_format,
            map_1: db.map_1.map(Map),
            map_2: db.map_2.map(Map),
            message_id: db.message_id.map(|id| MessageId::new(id as _)),
            event_id: ScheduledEventId::new(db.event_id as _),
            status: db.status.into(),
            server_info,
        }
    }
}

#[derive(Debug)]
pub struct Scrim {
    #[allow(dead_code)]
    pub guild_id: GuildId,
    pub game_format: GameFormat,
    pub timestamp: OffsetDateTime,
    pub opponent_user_id: UserId,
    pub map_1: Option<Map>,
    pub map_2: Option<Map>,
    pub event_id: ScheduledEventId,
    pub message_id: Option<MessageId>,
    pub status: Status,
    pub server_info: Option<ServerInfo>,
}

impl Scrim {
    pub fn title(&self) -> String {
        format!("Scrim - {}", self.timestamp.short_date())
    }

    pub fn local_date_time(&self) -> String {
        FormattedTimestamp::new(
            self.timestamp.into(),
            Some(FormattedTimestampStyle::LongDateTime),
        )
        .to_string()
    }

    pub fn event_url(&self) -> String {
        format!(
            "https://discord.com/events/{}/{}",
            self.guild_id, self.event_id
        )
    }

    pub fn actions(&self) -> Vec<CreateActionRow> {
        let buttons = if self.status == Status::Active {
            Some(CreateActionRow::Buttons(vec![
                CreateButton::new(format!("map:1:{}", self.timestamp.unix_timestamp()))
                    .label("Begin 1st Map")
                    .style(ButtonStyle::Primary)
                    .disabled(self.map_1.is_none()),
                CreateButton::new(format!("map:2:{}", self.timestamp.unix_timestamp()))
                    .label("Begin 2nd Map")
                    .style(ButtonStyle::Primary)
                    .disabled(self.map_2.is_none()),
            ]))
        } else {
            None
        };

        buttons.into_iter().collect()
    }

    pub fn message(&self) -> CreateMessage {
        CreateMessage::new()
            .content(self.event_url())
            .components(self.actions())
    }

    pub fn edit_message(&self) -> EditMessage {
        EditMessage::new()
            .content(self.event_url())
            .components(self.actions())
    }

    pub fn embed(&self) -> CreateEmbed {
        let mut fields = vec![
            (
                "Local Date & Time",
                FormattedTimestamp::new(
                    self.timestamp.into(),
                    Some(FormattedTimestampStyle::LongDateTime),
                )
                .to_string(),
                true,
            ),
            (
                "Opponent",
                self.opponent_user_id.mention().to_string(),
                true,
            ),
            ("Game Format", self.game_format.to_string(), true),
            (
                "1st Half Map",
                self.map_1
                    .as_ref()
                    .map_or_else(|| "Undecided".to_owned(), |m| format!("`{m}`")),
                true,
            ),
            (
                "2nd Half Map",
                self.map_2
                    .as_ref()
                    .map_or_else(|| "Undecided".to_owned(), |m| format!("`{m}`")),
                true,
            ),
            ("Connect Info", self.connect_info(), false),
        ];

        if let Some(ServerInfo::Serveme {
            reservation_id,
            rcon_password,
            ..
        }) = &self.server_info
        {
            fields.extend([
                (
                    "na.serveme.tf Reservation",
                    format!("https://na.serveme.tf/reservations/{reservation_id}",),
                    false,
                ),
                ("RCON Password", format!("`{rcon_password}`"), false),
            ]);
        }

        embed(self.title()).fields(fields)
    }

    pub fn connect_info(&self) -> String {
        self.server_info.as_ref().map_or_else(
            || "Not set".to_owned(),
            |server| format!("`{}`", server.connect_info().connect_command()),
        )
    }

    pub fn event_description(&self) -> String {
        format!(
            "**Opponent**: {}\n\
            **Game Format**: {}\n\
            **Maps**: {}/{}",
            self.opponent_user_id.mention(),
            self.game_format,
            self.map_1
                .as_ref()
                .map_or_else(|| "Undecided".to_owned(), |m| format!("`{m}`")),
            self.map_2
                .as_ref()
                .map_or_else(|| "Undecided".to_owned(), |m| format!("`{m}`")),
        )
    }

    pub fn create_event(&self) -> CreateScheduledEvent {
        CreateScheduledEvent::new(ScheduledEventType::External, self.title(), self.timestamp)
            .end_time(self.timestamp + Duration::HOUR)
            .location(self.connect_info())
            .description(self.event_description())
    }

    pub fn edit_event(&self) -> EditScheduledEvent {
        EditScheduledEvent::new()
            .name(self.title())
            .start_time(self.timestamp)
            .end_time(self.timestamp + Duration::HOUR)
            .location(self.connect_info())
            .description(self.event_description())
    }

    pub fn new_reservation_request(
        &self,
        servers: &FindServersResponse,
    ) -> BotResult<ReservationRequest> {
        let server = servers
            .servers
            .iter()
            .find(|server| {
                server.ip_and_port.starts_with("ks") || server.ip_and_port.starts_with("chi")
            })
            .ok_or(BotError::NoServemeServers)?;

        let server_config_id = self
            .map_1
            .as_ref()
            .and_then(|map| map.config_name_id(self.game_format))
            .map(|(_, id)| id);

        let mut password_generator = rand::thread_rng().sample_iter(&Alphanumeric);

        let password = format!(
            "scrim.{}",
            password_generator
                .by_ref()
                .take(8)
                .map(char::from)
                .collect::<String>()
        );

        let rcon = format!(
            "scrim.rcon.{}",
            password_generator
                .take(32)
                .map(char::from)
                .collect::<String>()
        );

        Ok(ReservationRequest {
            starts_at: self.timestamp - 10 * Duration::MINUTE,
            ends_at: self.timestamp + Duration::HOUR,
            first_map: self.map_1.clone(),
            server_id: server.id,
            password,
            rcon,
            server_config_id,
            enable_plugins: true,
            enable_demos_tf: true,
        })
    }

    pub fn edit_reservation_request(&self) -> EditReservationRequest {
        let server_config_id = self
            .map_1
            .as_ref()
            .and_then(|map| map.config_name_id(self.game_format))
            .map(|(_, id)| id);

        EditReservationRequest {
            starts_at: Some(self.timestamp - 10 * Duration::MINUTE),
            ends_at: Some(self.timestamp + Duration::HOUR),
            first_map: self.map_1.clone(),
            server_config_id,
        }
    }

    pub const fn reservation_id(&self) -> Option<u32> {
        match &self.server_info {
            Some(ServerInfo::Serveme { reservation_id, .. }) => Some(*reservation_id),
            _ => None,
        }
    }
}

impl From<Scrim> for DbScrim {
    fn from(scrim: Scrim) -> Self {
        let (reservation_id, serveme_rcon, ip_and_port, password) = scrim.server_info.map_or_else(
            || (None, None, None, None),
            |server| match server {
                ServerInfo::Serveme {
                    reservation_id,
                    rcon_password,
                    connect_info,
                } => (
                    Some(reservation_id as _),
                    Some(rcon_password),
                    Some(connect_info.ip_and_port),
                    Some(connect_info.password),
                ),
                ServerInfo::ExternalServer(connect_info) => (
                    None,
                    None,
                    Some(connect_info.ip_and_port),
                    Some(connect_info.password),
                ),
            },
        );

        Self {
            guild_id: scrim.guild_id.into(),
            timestamp: scrim.timestamp,
            opponent_user_id: scrim.opponent_user_id.into(),
            game_format: scrim.game_format.into(),
            map_1: scrim.map_1.map(Into::into),
            map_2: scrim.map_2.map(Into::into),
            message_id: scrim.message_id.map(Into::into),
            status: 0,
            event_id: scrim.event_id.into(),
            reservation_id,
            serveme_rcon,
            ip_and_port,
            password,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Map(pub String);

impl Map {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn config_name_id(&self, format: GameFormat) -> Option<(&'static str, u32)> {
        match format {
            GameFormat::Sixes => {
                if self.0.starts_with("cp_") {
                    Some(("rgl_6s_5cp_scrim", 69))
                } else if self.0.starts_with("koth_") {
                    Some(("rgl_6s_koth_scrim", 113))
                } else {
                    None
                }
            }
            GameFormat::Highlander => {
                if self.0.starts_with("pl_") || self.0.starts_with("cp_") {
                    Some(("rgl_HL_stopwatch", 55))
                } else if self.0.starts_with("koth_") {
                    Some(("rgl_HL_koth_bo5", 54))
                } else {
                    None
                }
            }
        }
    }
}

impl BasicOption for Map {
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(
        value: Option<&serenity::model::prelude::CommandDataOptionValue>,
    ) -> serenity_commands::Result<Self> {
        String::from_value(value).map(Self)
    }
}

impl Display for Map {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(&self.0)
    }
}

impl From<String> for Map {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<Map> for String {
    fn from(map: Map) -> Self {
        map.0
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Waiting,
    Active,
    Completed,
}

impl From<Status> for i16 {
    fn from(status: Status) -> Self {
        match status {
            Status::Waiting => 0,
            Status::Active => 1,
            Status::Completed => 2,
        }
    }
}

impl From<i16> for Status {
    fn from(status: i16) -> Self {
        match status {
            0 => Self::Waiting,
            1 => Self::Active,
            _ => Self::Completed,
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        f.write_str(match self {
            Self::Waiting => "Waiting",
            Self::Active => "Active",
            Self::Completed => "Completed",
        })
    }
}

#[derive(Debug)]
pub enum ServerInfo {
    Serveme {
        reservation_id: u32,
        rcon_password: String,
        connect_info: ConnectInfo,
    },
    ExternalServer(ConnectInfo),
}

impl ServerInfo {
    pub const fn connect_info(&self) -> &ConnectInfo {
        match self {
            Self::Serveme { connect_info, .. } | Self::ExternalServer(connect_info) => connect_info,
        }
    }
}
