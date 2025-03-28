use std::{
    collections::BTreeMap,
    iter,
    sync::{Arc, LazyLock},
    vec,
};

use moka::future::Cache;
use reqwest::{StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serenity::all::AutocompleteChoice;
use time::OffsetDateTime;

use crate::{
    BotResult, HTTP_CLIENT,
    entities::{ConnectInfo, GameFormat, Map, MapList, ReservationId, ServemeApiKey},
};

static CACHE: LazyLock<Cache<ReservationId, Arc<ReservationResponse>>> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(std::time::Duration::from_secs(10))
        .build()
});

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReservationWrapper<T> {
    reservation: T,
}

impl<T> From<T> for ReservationWrapper<T> {
    fn from(reservation: T) -> Self {
        Self { reservation }
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
                    .reservation
                    .into())
            })
            .await?)
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
                .reservation,
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
                .reservation,
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
                .reservation;

            Ok(Some(reservation))
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReservationResponse {
    pub id: ReservationId,
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
