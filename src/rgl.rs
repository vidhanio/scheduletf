use std::{
    fmt::{self, Display, Formatter},
    result::Result,
    sync::LazyLock,
};

use moka::future::Cache;
use scraper::{Html, Selector};
use serde::{Deserialize, de::Deserializer};
use serenity::all::{
    Colour, CreateActionRow, CreateButton, CreateEmbed, CreateEmbedAuthor, EditInteractionResponse,
    UserId,
};

use crate::{BotResult, HTTP_CLIENT, entities::team_guild::GameFormat, error::BotError};

#[allow(clippy::unreadable_literal)]
const RGL_ORANGE: Colour = Colour(0xE29455);

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglProfile {
    pub steam_id: SteamId,
    pub avatar: String,
    pub name: String,
    pub current_teams: RglProfileTeams,
}

impl RglProfile {
    pub async fn get(steam_id: SteamId) -> BotResult<Self> {
        HTTP_CLIENT
            .get(format!("https://api.rgl.gg/v0/profile/{steam_id}"))
            .send()
            .await?
            .error_for_status()?
            .json::<Self>()
            .await
            .map_err(Into::into)
    }

    pub async fn get_from_discord(user_id: UserId) -> BotResult<Self> {
        Self::get(SteamId::get_from_user_id(user_id).await?).await
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

impl RglProfileTeams {
    pub const fn get_team(&self, game_format: GameFormat) -> Option<&RglProfileTeam> {
        match game_format {
            GameFormat::Sixes => self.sixes.as_ref(),
            GameFormat::Highlander => self.highlander.as_ref(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglProfileTeam {
    pub id: RglTeamId,
    pub tag: String,
    pub name: String,
    pub division_name: String,
}

impl RglProfileTeam {
    fn embed_field_body(&self) -> String {
        format!(
            "[{}]({}) - {}",
            self.name,
            self.id.url(),
            self.division_name
                .strip_prefix("RGL-")
                .unwrap_or(&self.division_name),
        )
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RglTeam {
    pub team_id: RglTeamId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SteamId(pub u64);

impl SteamId {
    pub async fn get_from_user_id(user_id: UserId) -> BotResult<Self> {
        static STEAM_ID_CACHE: LazyLock<Cache<UserId, SteamId>> = LazyLock::new(|| {
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

        STEAM_ID_CACHE
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

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(transparent)]
pub struct RglTeamId(pub u32);

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
