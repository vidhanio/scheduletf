use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    str::FromStr,
};

use reqwest::{header::AUTHORIZATION, StatusCode};
use serde::{Deserialize, Serialize};
use serenity::all::CreateCommandOption;
use serenity_commands::BasicOption;
use time::OffsetDateTime;

use crate::{error::BotError, models::Map, BotResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationWrapper<T> {
    pub reservation: T,
}

impl<T> From<T> for ReservationWrapper<T> {
    fn from(reservation: T) -> Self {
        Self { reservation }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReservationError {
    pub error: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindServersRequest {
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,

    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,
}

impl FindServersRequest {
    pub async fn send(
        &self,
        client: &reqwest::Client,
        api_key: &str,
    ) -> BotResult<FindServersResponse> {
        client
            .post("https://na.serveme.tf/api/reservations/find_servers")
            .header(AUTHORIZATION, format!("Token token={api_key}"))
            .json(&ReservationWrapper::from(self))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(Into::into)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct FindServersResponse {
    pub servers: Vec<Server>,
    pub server_configs: Vec<ServerConfig>,
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

#[derive(Debug, Clone, Serialize)]
pub struct ReservationRequest {
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

impl ReservationRequest {
    pub async fn send(
        &self,
        client: &reqwest::Client,
        api_key: &str,
    ) -> BotResult<ReservationResponse> {
        let reservation = client
            .post("https://na.serveme.tf/api/reservations")
            .header(AUTHORIZATION, format!("Token token={api_key}"))
            .json(&ReservationWrapper::from(self))
            .send()
            .await?
            .error_for_status()?
            .json::<ReservationWrapper<ReservationResponse>>()
            .await?
            .reservation;

        if let Some(errors) = reservation.errors {
            return Err(BotError::ServemeReservation(errors));
        }

        Ok(reservation)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EditReservationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts_at: Option<OffsetDateTime>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends_at: Option<OffsetDateTime>,

    pub first_map: Option<Map>,
    pub server_config_id: Option<u32>,
}

impl EditReservationRequest {
    pub async fn send(
        &self,
        client: &reqwest::Client,
        api_key: &str,
        reservation_id: u32,
    ) -> BotResult<ReservationResponse> {
        let reservation = client
            .patch(format!(
                "https://na.serveme.tf/api/reservations/{reservation_id}"
            ))
            .header(AUTHORIZATION, format!("Token token={api_key}"))
            .json(&ReservationWrapper::from(self))
            .send()
            .await?
            .error_for_status()?
            .json::<ReservationWrapper<ReservationResponse>>()
            .await?
            .reservation;

        if let Some(errors) = reservation.errors {
            return Err(BotError::ServemeReservation(errors));
        }

        Ok(reservation)
    }
}

#[derive(Debug, Clone)]
pub struct DeleteReservationRequest;

impl DeleteReservationRequest {
    pub async fn send(
        client: &reqwest::Client,
        api_key: &str,
        reservation_id: u32,
    ) -> BotResult<Option<ReservationResponse>> {
        let resp = client
            .delete(format!(
                "https://na.serveme.tf/api/reservations/{reservation_id}"
            ))
            .header(AUTHORIZATION, format!("Token token={api_key}"))
            .send()
            .await?
            .error_for_status()?;

        if resp.status() == StatusCode::NO_CONTENT {
            Ok(None)
        } else {
            let reservation = resp
                .json::<ReservationWrapper<ReservationResponse>>()
                .await?
                .reservation;

            #[allow(clippy::option_if_let_else)]
            if let Some(errors) = reservation.errors {
                Err(BotError::ServemeReservation(errors))
            } else {
                Ok(Some(reservation))
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReservationResponse {
    pub id: u32,
    pub status: String,
    #[serde(with = "time::serde::iso8601")]
    pub starts_at: OffsetDateTime,
    #[serde(with = "time::serde::iso8601")]
    pub ends_at: OffsetDateTime,
    pub password: String,
    pub rcon: String,
    pub server: Server,
    pub errors: Option<HashMap<String, ReservationError>>,
}

impl ReservationResponse {
    pub fn connect_info(&self) -> ConnectInfo {
        ConnectInfo {
            ip_and_port: self.server.ip_and_port.clone(),
            password: self.password.clone(),
        }
    }

    pub fn reservation_url(&self) -> String {
        format!("https://na.serveme.tf/reservations/{}", self.id)
    }
}

#[derive(Debug, Clone)]
pub struct ConnectInfo {
    pub ip_and_port: String,
    pub password: String,
}

impl BasicOption for ConnectInfo {
    fn create_option(
        name: impl Into<String>,
        description: impl Into<String>,
    ) -> CreateCommandOption {
        String::create_option(name, description)
    }

    fn from_value(
        value: Option<&serenity::model::prelude::CommandDataOptionValue>,
    ) -> serenity_commands::Result<Self> {
        let value = String::from_value(value)?;

        value
            .parse()
            .map_err(|err| serenity_commands::Error::Custom(Box::new(err)))
    }
}

impl ConnectInfo {
    pub fn connect_command(&self) -> String {
        self.to_string()
    }

    pub fn connect_url(&self) -> String {
        format!("steam://connect/{}/{}", self.ip_and_port, self.password)
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

impl FromStr for ConnectInfo {
    type Err = BotError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (mut ip_and_port, s) = s
            .trim_start()
            .strip_prefix("connect")
            .ok_or(BotError::InvalidConnectInfo)?
            .trim_start()
            .split_once(';')
            .ok_or(BotError::InvalidConnectInfo)?;

        ip_and_port = ip_and_port.trim_end();

        if ip_and_port.starts_with('"') && ip_and_port.ends_with('"') && ip_and_port.len() > 1 {
            ip_and_port = &ip_and_port[1..ip_and_port.len() - 1];
        }

        let mut password = s
            .trim_start()
            .strip_prefix("password")
            .ok_or(BotError::InvalidConnectInfo)?
            .trim();

        if password.starts_with('"') && password.ends_with('"') && password.len() > 1 {
            password = &password[1..password.len() - 1];
        }

        Ok(Self {
            ip_and_port: ip_and_port.to_owned(),
            password: password.to_owned(),
        })
    }
}
