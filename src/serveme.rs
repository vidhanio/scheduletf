use std::sync::{Arc, LazyLock};

use moka::future::Cache;
use reqwest::{StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::{
    BotResult, HTTP_CLIENT,
    entities::{ConnectInfo, GameFormat, Map, ReservationId, ServemeApiKey},
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
    pub name: String,
    pub ip_and_port: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub id: u32,
    pub file: String,
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone)]
pub struct DeleteReservationRequest;

impl DeleteReservationRequest {
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
    pub status: String,
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,
    pub password: String,
    pub rcon: String,
    pub server: Server,
}

impl ReservationResponse {
    pub fn connect_info(&self) -> ConnectInfo {
        ConnectInfo {
            ip_and_port: self.server.ip_and_port.clone(),
            password: self.password.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MapsRequest;

impl MapsRequest {
    pub async fn send(
        api_key: &ServemeApiKey,
        format: Option<GameFormat>,
    ) -> BotResult<Arc<[Map]>> {
        static MAP_CACHE: LazyLock<Cache<Option<GameFormat>, Arc<[Map]>>> = LazyLock::new(|| {
            Cache::builder()
                .time_to_live(std::time::Duration::from_secs(24 * 60 * 60))
                .build()
        });

        #[derive(Deserialize)]
        struct MapsResponse {
            maps: Vec<Map>,
        }

        Ok(MAP_CACHE
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

                maps.sort_by(|a, b| a.cmp_with_format(b, format));

                Ok(maps.into())
            })
            .await?)
    }
}
