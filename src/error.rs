use std::{collections::HashMap, io};

use thiserror::Error;
use time::OffsetDateTime;

use crate::{
    serveme::ReservationError,
    utils::{ComponentInteractionDataType, ScrimOffsetExtension},
};

#[derive(Debug, Error)]
pub enum BotError {
    #[error("io error: `{0}`")]
    Io(#[from] io::Error),

    #[error("parse int error: `{0}`")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("http error: `{0}`")]
    Http(#[from] reqwest::Error),

    #[error("json error: `{0}`")]
    Json(#[from] serde_json::Error),

    #[error("database error: `{0}`")]
    Database(#[from] sqlx::Error),

    #[error("tokio task join error: `{0}`")]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error("serenity error: `{0}`")]
    Serenity(#[from] serenity::Error),

    #[error("command parse error: `{0}`")]
    CommandParse(#[from] serenity_commands::Error),

    #[error("time parse error: `{0}`")]
    TimeParse(#[from] time::error::Parse),

    #[error("component range error: `{0}`")]
    ComponentRange(#[from] time::error::ComponentRange),

    #[error("no guild associated with interaction")]
    NoGuild,

    #[error("na.serveme.tf reservation error(s):\n```\n{0:?}\n```")]
    ServemeReservation(HashMap<String, ReservationError>),

    #[error("no na.serveme.tf servers found")]
    NoServemeServers,

    #[error("na.serveme.tf config for gamemode not found")]
    NoServemeConfigFound,

    #[error("na.serveme.tf api key not set. run `/config serveme` to set it.")]
    NoServemeApiKey,

    #[error("connect info is invalid")]
    InvalidConnectInfo,

    #[error("scrim already scheduled at `{}`", .0.short_date())]
    ScrimAlreadyScheduled(OffsetDateTime),

    #[error("scrim not hosted: `{}`", .0.short_date())]
    ScrimNotHosted(OffsetDateTime),

    #[error(
        "incorrect interaction data kind for `{component}`: expected `{expected:?}`, got `{got:?}`"
    )]
    IncorrectInteractionDataKind {
        component: String,
        expected: ComponentInteractionDataType,
        got: ComponentInteractionDataType,
    },

    #[error(
        "incorrect amount of items selected for `{component}`: expected `{expected}`, got `{got}`"
    )]
    IncorrectNumberOfItems {
        component: String,
        expected: usize,
        got: usize,
    },

    #[error("invalid component interaction: `{0}`")]
    InvalidComponentInteraction(String),
}
