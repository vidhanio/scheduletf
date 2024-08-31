use std::io;

use thiserror::Error;

use crate::utils::ComponentInteractionDataType;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("parse int error: {0}")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("tokio task join error: {0}")]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error("serenity error: {0}")]
    Serenity(#[from] serenity::Error),

    #[error("command parse error: {0}")]
    CommandParse(#[from] serenity_commands::Error),

    #[error("time parse error: {0}")]
    TimeParse(#[from] time::error::Parse),

    #[error("no guild associated with interaction")]
    NoGuild,

    #[error("guild voice channel not set up. run `/setup vc` to set it up.")]
    NoVoiceChannel,

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
}
