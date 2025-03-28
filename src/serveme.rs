use std::{
    collections::{BTreeMap, HashMap},
    iter,
    sync::{Arc, LazyLock},
    vec,
};

use moka::future::Cache;
use rcon::Connection;
use reqwest::{StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serenity::all::AutocompleteChoice;
use thiserror::Error;
use time::OffsetDateTime;
use tokio::net::TcpStream;

use crate::{
    BotResult, HTTP_CLIENT,
    entities::{ConnectInfo, GameFormat, Map, MapList, ReservationId, ServemeApiKey},
    error::BotError,
};

static CACHE: LazyLock<Cache<ReservationId, Arc<ReservationResponse>>> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(std::time::Duration::from_secs(10))
        .build()
});

#[derive(Serialize, Deserialize)]
struct ReservationWrapper<T> {
    reservation: ReservationErrorsWrapper<T>,
}

impl<T> ReservationWrapper<T> {
    pub fn into_result(self) -> Result<T, BotError> {
        if let Some(errors) = self.reservation.errors {
            Err(BotError::Serveme(errors))
        } else {
            Ok(self.reservation.reservation)
        }
    }
}

impl<T> From<T> for ReservationWrapper<T>
where
    T: Serialize,
{
    fn from(reservation: T) -> Self {
        Self {
            reservation: ReservationErrorsWrapper {
                reservation,
                errors: None,
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct ReservationErrorsWrapper<T> {
    #[serde(flatten)]
    reservation: T,
    #[serde(skip_serializing)]
    errors: Option<ServemeError>,
}

#[derive(Debug, Error)]
#[error("na.serveme.tf error: {}", .0.iter().map(|(k, v)| format!("{k}: {v}")).collect::<Vec<_>>().join(", "))]
pub struct ServemeError(pub HashMap<String, String>);

impl<'de> Deserialize<'de> for ServemeError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ErrorWrapper {
            error: String,
        }

        let error = HashMap::<String, ErrorWrapper>::deserialize(deserializer)?
            .into_iter()
            .map(|(k, v)| (k, v.error))
            .collect();

        Ok(Self(error))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct FindServersRequest {
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,

    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,
}

impl FindServersRequest {
    pub async fn send(&self, api_key: &ServemeApiKey) -> BotResult<FindServersResponse> {
        Ok(HTTP_CLIENT
            .post("https://na.serveme.tf/api/reservations/find_servers")
            .header(AUTHORIZATION, api_key.auth_header())
            .json(&ReservationWrapper::from(self))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FindServersResponse {
    pub servers: Vec<Server>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Server {
    pub id: u32,
    pub ip: String,
    pub ip_and_port: String,
}

#[derive(Debug, Clone)]
pub struct GetReservationRequest;

impl GetReservationRequest {
    pub async fn send(
        api_key: &ServemeApiKey,
        reservation_id: ReservationId,
    ) -> BotResult<Arc<ReservationResponse>> {
        Ok(CACHE
            .try_get_with(reservation_id, async {
                Ok(HTTP_CLIENT
                    .get(format!(
                        "https://na.serveme.tf/api/reservations/{reservation_id}"
                    ))
                    .header(AUTHORIZATION, api_key.auth_header())
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<ReservationWrapper<ReservationResponse>>()
                    .await?
                    .into_result()?
                    .into())
            })
            .await?)
    }

    pub async fn send_many(api_key: &ServemeApiKey) -> BotResult<Vec<Arc<ReservationResponse>>> {
        #[derive(Deserialize)]
        struct ReservationsResponse {
            reservations: Vec<Arc<ReservationResponse>>,
        }

        let reservations = HTTP_CLIENT
            .get("https://na.serveme.tf/api/reservations")
            .header(AUTHORIZATION, api_key.auth_header())
            .send()
            .await?
            .error_for_status()?
            .json::<ReservationsResponse>()
            .await?
            .reservations;

        for reservation in &reservations {
            CACHE.insert(reservation.id, Arc::clone(reservation)).await;
        }

        Ok(reservations)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateReservationRequest {
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,

    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,

    pub server_id: u32,
    pub password: String,
    pub rcon: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_map: Option<Map>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_config_id: Option<u32>,
    pub enable_plugins: bool,
    pub enable_demos_tf: bool,
}

impl CreateReservationRequest {
    pub async fn send(&self, api_key: &ServemeApiKey) -> BotResult<Arc<ReservationResponse>> {
        let reservation = Arc::new(
            HTTP_CLIENT
                .post("https://na.serveme.tf/api/reservations")
                .header(AUTHORIZATION, api_key.auth_header())
                .json(&ReservationWrapper::from(self))
                .send()
                .await?
                .error_for_status()?
                .json::<ReservationWrapper<ReservationResponse>>()
                .await?
                .into_result()?,
        );

        CACHE.insert(reservation.id, Arc::clone(&reservation)).await;

        Ok(reservation)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct EditReservationRequest {
    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::iso8601::option"
    )]
    pub starts_at: Option<OffsetDateTime>,

    #[serde(
        skip_serializing_if = "Option::is_none",
        with = "time::serde::iso8601::option"
    )]
    pub ends_at: Option<OffsetDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_map: Option<Map>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_config_id: Option<u32>,
}

impl EditReservationRequest {
    pub async fn send(
        &self,
        api_key: &ServemeApiKey,
        reservation_id: ReservationId,
    ) -> BotResult<Arc<ReservationResponse>> {
        let reservation = Arc::new(
            HTTP_CLIENT
                .patch(format!(
                    "https://na.serveme.tf/api/reservations/{reservation_id}"
                ))
                .header(AUTHORIZATION, api_key.auth_header())
                .json(&ReservationWrapper::from(self))
                .send()
                .await?
                .error_for_status()?
                .json::<ReservationWrapper<ReservationResponse>>()
                .await?
                .into_result()?,
        );

        CACHE.insert(reservation.id, Arc::clone(&reservation)).await;

        Ok(reservation)
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DeleteReservationRequest;

impl DeleteReservationRequest {
    #[allow(dead_code)]
    pub async fn send(
        api_key: &ServemeApiKey,
        reservation_id: ReservationId,
    ) -> BotResult<Option<ReservationResponse>> {
        let resp = HTTP_CLIENT
            .delete(format!(
                "https://na.serveme.tf/api/reservations/{reservation_id}"
            ))
            .header(AUTHORIZATION, api_key.auth_header())
            .send()
            .await?
            .error_for_status()?;

        CACHE.invalidate(&reservation_id).await;

        if resp.status() == StatusCode::NO_CONTENT {
            Ok(None)
        } else {
            let reservation = resp
                .json::<ReservationWrapper<ReservationResponse>>()
                .await?
                .into_result()?;

            Ok(Some(reservation))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReservationResponse {
    pub id: ReservationId,
    pub status: ReservationStatus,
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,
    pub password: String,
    pub rcon: String,
    pub first_map: Option<Map>,
    pub tv_password: String,
    pub tv_port: u16,
    pub server_config_id: Option<u32>,
    pub server: Server,
}

impl ReservationResponse {
    pub fn connect_info(&self) -> ConnectInfo {
        ConnectInfo {
            ip_and_port: self.server.ip_and_port.clone(),
            password: self.password.clone(),
        }
    }

    pub fn stv_connect_info(&self) -> ConnectInfo {
        ConnectInfo {
            ip_and_port: format!("{}:{}", self.server.ip, self.tv_port),
            password: self.tv_password.clone(),
        }
    }

    pub fn rcon_info(&self) -> String {
        format!(
            r#"rcon_address {}; rcon_password "{}""#,
            self.server.ip_and_port, self.rcon
        )
    }

    pub async fn rcon(&self, cmd: &str) -> BotResult<String> {
        let mut rcon_client =
            Connection::<TcpStream>::connect(&self.server.ip_and_port, &self.rcon).await?;

        let resp = rcon_client.cmd(cmd).await?;

        Ok(resp)
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum ReservationStatus {
    #[serde(rename = "Waiting to start")]
    WaitingToStart,

    #[serde(rename = "Starting")]
    Starting,

    #[serde(rename = "Server updating, please be patient")]
    ServerUpdating,

    #[serde(rename = "Ready")]
    Ready,

    #[serde(rename = "SDR Ready")]
    SdrReady,

    #[serde(rename = "Ending")]
    Ending,

    #[serde(rename = "Ended")]
    Ended,

    #[serde(rename = "Unknown")]
    Unknown,
}

impl ReservationStatus {
    pub const fn is_ready(self) -> bool {
        matches!(self, Self::Ready | Self::SdrReady)
    }

    pub const fn is_ended(self) -> bool {
        matches!(self, Self::Ending | Self::Ended)
    }
}

#[derive(Debug, Clone)]
pub struct MapsRequest;

impl MapsRequest {
    pub async fn send(api_key: &ServemeApiKey, format: Option<GameFormat>) -> BotResult<AllMaps> {
        static MAP_CACHE: LazyLock<Cache<Option<GameFormat>, Arc<[Map]>>> = LazyLock::new(|| {
            Cache::builder()
                .time_to_live(std::time::Duration::from_secs(24 * 60 * 60))
                .build()
        });

        #[derive(Deserialize)]
        struct MapsResponse {
            maps: Vec<Map>,
        }

        let official_maps = Map::official_maps(format);

        let unofficial_maps = MAP_CACHE
            .try_get_with(format, async {
                let mut maps = HTTP_CLIENT
                    .get("https://na.serveme.tf/api/maps")
                    .header(AUTHORIZATION, api_key.auth_header())
                    .send()
                    .await?
                    .error_for_status()?
                    .json::<MapsResponse>()
                    .await?
                    .maps;

                maps.retain(|map| !official_maps.contains_key(map));

                Ok(maps.into())
            })
            .await?;

        Ok(AllMaps {
            official: official_maps,
            unofficial: unofficial_maps,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AllMaps {
    pub official: &'static BTreeMap<Map, &'static str>,
    pub unofficial: Arc<[Map]>,
}

impl AllMaps {
    pub fn iter(&self) -> impl Iterator<Item = &Map> {
        self.official.keys().chain(self.unofficial.iter())
    }

    pub fn autocomplete_choices(
        &self,
        maps: &MapList,
        trailing_sep: bool,
    ) -> Vec<AutocompleteChoice> {
        if maps.is_empty() {
            self.iter()
                .map(|map| MapList::autocomplete_choice([map]))
                .take(25)
                .collect()
        } else {
            let choices = self
                .official_autocomplete_choices(maps)
                .into_iter()
                .chain(self.unofficial_autocomplete_choices(maps));

            if trailing_sep {
                choices
                    .flat_map(|maps| {
                        self.iter().map(move |map| {
                            maps.iter()
                                .copied()
                                .chain(iter::once(map))
                                .collect::<Vec<_>>()
                        })
                    })
                    .map(MapList::autocomplete_choice)
                    .take(25)
                    .collect()
            } else {
                choices.map(MapList::autocomplete_choice).take(25).collect()
            }
        }
    }

    fn official_autocomplete_choices<'a>(&'a self, maps: &MapList) -> Vec<Vec<&'a Map>> {
        fn inner(map: &'static Map, children: &[Vec<&'static Map>]) -> Vec<Vec<&'static Map>> {
            if let Some((children, grandchildren)) = children.split_first() {
                children
                    .iter()
                    .flat_map(|choice| {
                        inner(choice, grandchildren).into_iter().map(|mut choices| {
                            choices.insert(0, map);
                            choices
                        })
                    })
                    .collect()
            } else {
                vec![vec![map]]
            }
        }

        let per_map_choices = maps
            .iter()
            .map(|map| {
                self.official
                    .keys()
                    .filter(|official_map| {
                        official_map.to_lowercase().contains(&map.to_lowercase())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        match per_map_choices.split_first() {
            Some((choices, children)) => choices
                .iter()
                .flat_map(|choice| inner(choice, children))
                .collect(),
            _ => {
                vec![]
            }
        }
    }

    fn unofficial_autocomplete_choices<'a>(
        &'a self,
        maps: &'a MapList,
    ) -> impl Iterator<Item = Vec<&'a Map>> {
        let (last_map, maps) = maps
            .split_last()
            .expect("empty map list is handled in `Self::autocomplete_choices`");

        self.unofficial
            .iter()
            .filter(|map| map.to_lowercase().contains(&last_map.to_lowercase()))
            .map(|map| maps.iter().chain(iter::once(map)).collect())
    }
}
