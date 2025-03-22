use std::{io, sync::Arc};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("`io` error: `{0}`")]
    Io(#[from] io::Error),

    #[error("Integer parsing error: `{0}`")]
    ParseInt(#[from] std::num::ParseIntError),

    #[error("HTTP error: `{0}`")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: `{0}`")]
    Json(#[from] serde_json::Error),

    #[error("Database error: `{0}`")]
    Database(#[from] sea_orm::error::DbErr),

    #[error("Tokio task join error: `{0}`")]
    TokioJoin(#[from] tokio::task::JoinError),

    #[error("Serenity error: `{0}`")]
    Serenity(#[source] Box<serenity::Error>),

    #[error("Command parsing error: `{0}`")]
    CommandParse(#[from] serenity_commands::Error),

    #[error("Time parsing error: `{0}`")]
    TimeParse(#[from] time::error::Parse),

    #[error("No guild associated with interaction.")]
    NoGuild,

    #[error("No `na.serveme.tf` servers found.")]
    NoServemeServers,

    #[error("`na.serveme.tf` API key not set. Run `/config set serveme` to set it.")]
    NoServemeApiKey,

    #[error("Invalid connect command.")]
    InvalidConnectCommand,

    #[error("Game already scheduled for that time.")]
    GameAlreadyScheduled,

    #[error("Game not found.")]
    GameNotFound,

    #[error("Invalid match info.")]
    InvalidMatchInfo,

    #[error("Invalid server info.")]
    InvalidServerInfo,

    #[error("No reservation ID provided.")]
    NoReservationId,

    #[error(
        "No game format provided. Either set a default game format with `/config set game-format` or provide one in the command."
    )]
    NoGameFormat,

    #[error("No schedule channel set. Set one with `/config set schedule-channel`.")]
    NoScheduleChannel,

    #[error("RGL.gg profile not found.")]
    RglProfileNotFound,

    #[error("Invalid interaction target.")]
    InvalidInteractionTarget,

    #[error("Invalid component interaction")]
    InvalidComponentInteraction,

    #[error("Invalid date/time")]
    InvalidDateTime,

    #[error(transparent)]
    Arc(#[from] Arc<Self>),
}

impl From<serenity::Error> for BotError {
    fn from(err: serenity::Error) -> Self {
        Self::Serenity(Box::new(err))
    }
}
