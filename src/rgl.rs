use std::{
    fmt::{self, Display, Formatter},
    hash::Hash,
    result::Result,
    sync::{Arc, LazyLock},
};

use moka::future::Cache;
use scraper::{Html, Selector};
use sea_orm::{
    DeriveValueType,
    sea_query::{Nullable, Value},
};
use serde::{Deserialize, de::Deserializer};
use serenity::all::{
    Colour, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, EditInteractionResponse,
    UserId,
};
use serenity_commands::BasicOption;
use time::OffsetDateTime;

use crate::{
    BotResult, HTTP_CLIENT,
    entities::{GameFormat, Map},
    error::BotError,
};

#[allow(clippy::unreadable_literal)]
const RGL_ORANGE: Colour = Colour(0xE29455);

fn build_rgl_cache<K: Hash + Eq + Send + Sync + 'static, V: Clone + Send + Sync + 'static>()
-> Cache<K, V> {
    Cache::builder()
        .time_to_live(std::time::Duration::from_secs(10))
        .build()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglProfile {
    pub steam_id: SteamId,
    pub avatar: String,
    pub name: String,
    pub current_teams: RglProfileTeams,
}

impl RglProfile {
    pub async fn get(steam_id: SteamId) -> BotResult<Arc<Self>> {
        static CACHE: LazyLock<Cache<SteamId, Arc<RglProfile>>> = LazyLock::new(build_rgl_cache);

        Ok(CACHE
            .try_get_with(steam_id, async {
                HTTP_CLIENT
                    .get(format!("https://api.rgl.gg/v0/profile/{steam_id}"))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .map_err(Into::into)
            })
            .await?)
    }

    pub async fn get_from_discord(user_id: UserId) -> BotResult<Arc<Self>> {
        let steam_id = SteamId::get_from_user_id(user_id).await?;

        Self::get(steam_id).await
    }

    pub fn url(&self, game_format: Option<GameFormat>) -> String {
        game_format.map_or_else(
            || self.steam_id.rgl_url(),
            |game_format| format!("{}&r={}", self.steam_id.rgl_url(), game_format.rgl_id()),
        )
    }

    pub fn response(&self) -> EditInteractionResponse {
        let embed = self.embed();
        let buttons = self.steam_id.buttons();

        EditInteractionResponse::new()
            .embed(embed)
            .components(vec![buttons])
    }

    fn embed(&self) -> CreateEmbed {
        CreateEmbed::default()
            .title(&self.name)
            .url(self.url(None))
            .thumbnail(&self.avatar)
            .color(RGL_ORANGE)
            .author(
                CreateEmbedAuthor::new("RGL.gg")
                    .url("https://rgl.gg")
                    .icon_url("https://liquipedia.net/commons/images/6/66/RGL_Logo.png"),
            )
            .fields([
                (
                    "Sixes",
                    self.current_teams.sixes.as_ref().map_or_else(
                        || "Not Rostered".to_owned(),
                        RglProfileTeam::embed_field_body,
                    ),
                    false,
                ),
                (
                    "Highlander",
                    self.current_teams.highlander.as_ref().map_or_else(
                        || "Not Rostered".to_owned(),
                        RglProfileTeam::embed_field_body,
                    ),
                    false,
                ),
            ])
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglProfileTeams {
    pub sixes: Option<RglProfileTeam>,
    pub highlander: Option<RglProfileTeam>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglProfileTeam {
    pub id: RglTeamId,
    pub name: String,
    pub division_id: DivisionId,
    pub division_name: String,
}

impl RglProfileTeam {
    fn embed_field_body(&self) -> String {
        format!(
            "[{}]({}) - [{}]({})",
            self.name,
            self.id.url(),
            self.division_name
                .strip_prefix("RGL-")
                .unwrap_or(&self.division_name),
            self.division_id.url(),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglTeam {
    pub season_id: SeasonId,
}

impl RglTeam {
    pub async fn get(team_id: RglTeamId) -> BotResult<Arc<Self>> {
        static CACHE: LazyLock<Cache<RglTeamId, Arc<RglTeam>>> = LazyLock::new(build_rgl_cache);

        Ok(CACHE
            .try_get_with(team_id, async {
                HTTP_CLIENT
                    .get(format!("https://api.rgl.gg/v0/teams/{team_id}"))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .map_err(Into::into)
            })
            .await?)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglMatch {
    pub match_id: RglMatchId,
    pub season_id: SeasonId,
    #[serde(with = "time::serde::iso8601")]
    pub match_date: OffsetDateTime,
    pub match_name: String,
    pub teams: (RglMatchTeam, RglMatchTeam),
    pub maps: Vec<RglMatchMap>,
}

impl RglMatch {
    pub async fn get(match_id: RglMatchId) -> BotResult<Arc<Self>> {
        static CACHE: LazyLock<Cache<RglMatchId, Arc<RglMatch>>> = LazyLock::new(build_rgl_cache);

        Ok(CACHE
            .try_get_with(match_id, async {
                HTTP_CLIENT
                    .get(format!("https://api.rgl.gg/v0/matches/{match_id}"))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .map_err(Into::into)
            })
            .await?)
    }

    pub fn opponent_team(&self, team_id: RglTeamId) -> BotResult<RglMatchTeam> {
        match (
            self.teams.0.team_id == team_id,
            self.teams.1.team_id == team_id,
        ) {
            (true, false) => Ok(self.teams.1.clone()),
            (false, true) => Ok(self.teams.0.clone()),
            _ => Err(BotError::TeamNotInMatch),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglMatchTeam {
    pub team_name: String,
    pub team_id: RglTeamId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglMatchMap {
    pub map_name: Map,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglSeason {
    pub format_name: GameFormat,
}

impl RglSeason {
    pub async fn get(season_id: SeasonId) -> BotResult<Arc<Self>> {
        static CACHE: LazyLock<Cache<SeasonId, Arc<RglSeason>>> = LazyLock::new(build_rgl_cache);

        Ok(CACHE
            .try_get_with(season_id, async {
                HTTP_CLIENT
                    .get(format!("https://api.rgl.gg/v0/seasons/{season_id}"))
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await
                    .map_err(Into::into)
            })
            .await?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SteamId(pub u64);

impl SteamId {
    pub async fn get_from_user_id(user_id: UserId) -> BotResult<Self> {
        static CACHE: LazyLock<Cache<UserId, SteamId>> = LazyLock::new(|| {
            Cache::builder()
                .time_to_live(std::time::Duration::from_secs(24 * 60 * 60))
                .build()
        });

        static STEAM_SELECTOR: LazyLock<Selector> = LazyLock::new(|| {
            Selector::parse(
                "a#ContentPlaceHolder1_ContentPlaceHolder1_ContentPlaceHolder1_hlSteamProfile",
            )
            .expect("static selector should be valid")
        });

        CACHE
            .try_get_with(user_id, async {
                let html = HTTP_CLIENT
                    .get(format!(
                        "https://rgl.gg/Public/PlayerProfile.aspx?d={user_id}"
                    ))
                    .send()
                    .await?
                    .error_for_status()?
                    .text()
                    .await?;

                let document = Html::parse_document(&html);

                document
                    .select(&STEAM_SELECTOR)
                    .next()
                    .and_then(|element| {
                        element
                            .value()
                            .attr("href")?
                            .rsplit_once('/')?
                            .1
                            .parse()
                            .ok()
                    })
                    .map(SteamId)
                    .ok_or(BotError::RglProfileNotFound)
            })
            .await
            .map_err(Into::into)
    }

    fn buttons(self) -> CreateActionRow {
        CreateActionRow::Buttons(vec![
            CreateButton::new_link(self.steam_url())
                .label("Steam")
                .emoji('👤'),
            CreateButton::new_link(self.logstf_url())
                .label("Logs")
                .emoji('🪵'),
            CreateButton::new_link(self.demostf_url())
                .label("Demos")
                .emoji('🎥'),
            CreateButton::new_link(self.trendstf_url())
                .label("Trends")
                .emoji('📈'),
            CreateButton::new_link(self.moretf_url())
                .label("More")
                .emoji('🔍'),
        ])
    }

    fn rgl_url(self) -> String {
        format!("https://rgl.gg/Public/PlayerProfile.aspx?p={self}")
    }

    fn steam_url(self) -> String {
        format!("https://steamcommunity.com/profiles/{self}")
    }

    fn logstf_url(self) -> String {
        format!("https://logs.tf/profile/{self}")
    }

    fn demostf_url(self) -> String {
        format!("https://demos.tf/profiles/{self}")
    }

    fn trendstf_url(self) -> String {
        format!("https://trends.tf/player/{self}")
    }

    fn moretf_url(self) -> String {
        format!("https://more.tf/profile/{self}")
    }
}

impl From<u64> for SteamId {
    fn from(id: u64) -> Self {
        Self(id)
    }
}

impl Display for SteamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl<'de> Deserialize<'de> for SteamId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum SteamId {
            U64(u64),
            String(String),
        }

        let id = SteamId::deserialize(deserializer)?;

        match id {
            SteamId::U64(id) => Ok(Self(id)),
            SteamId::String(id) => id
                .parse::<u64>()
                .map(Self)
                .map_err(|_| serde::de::Error::custom("invalid steam id")),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, BasicOption, DeriveValueType)]
#[serde(transparent)]
pub struct RglTeamId(pub i32);

impl RglTeamId {
    pub fn url(self) -> String {
        format!("https://rgl.gg/Public/Team.aspx?t={self}")
    }
}

impl Display for RglTeamId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

impl Nullable for RglTeamId {
    fn null() -> Value {
        i32::null()
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct SeasonId(pub i32);

impl SeasonId {
    pub fn url(self) -> String {
        format!("https://rgl.gg/Public/LeagueTable?s={self}")
    }
}

impl Display for SeasonId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(transparent)]
pub struct DivisionId(pub i32);

impl DivisionId {
    pub fn url(self) -> String {
        format!("https://rgl.gg/Public/LeagueTable?g={self}")
    }
}

impl Display for DivisionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Deserialize, BasicOption, DeriveValueType)]
#[serde(transparent)]
pub struct RglMatchId(pub i32);

impl RglMatchId {
    pub fn url(self) -> String {
        format!("https://rgl.gg/Public/Match?m={self}")
    }
}

impl Nullable for RglMatchId {
    fn null() -> Value {
        i32::null()
    }
}

impl Display for RglMatchId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, f)
    }
}
