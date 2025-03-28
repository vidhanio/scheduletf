use std::sync::Arc;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum BotError {
    #[error("HTTP error: `{0}`")]
    Http(#[from] reqwest::Error),

    #[error("Database error: `{0}`")]
    Database(#[from] sea_orm::error::DbErr),

    #[error("Serenity error: `{0}`")]
    Serenity(#[source] Box<serenity::Error>),

    #[error("Command parsing error: `{0}`")]
    CommandParse(#[from] serenity_commands::Error),

    #[error(transparent)]
    Arc(#[from] Arc<Self>),

    #[error("No guild associated with interaction.")]
    NoGuild,

    #[error("Invalid interaction target.")]
    InvalidInteractionTarget,

    #[error("Invalid component interaction")]
    InvalidComponentInteraction,

    #[error("Invalid game details.")]
    InvalidGameDetails,

    #[error("No na.serveme.tf servers found.")]
    NoServemeServers,

    #[error("invalid IP/port from na.servemetf.")]
    InvalidServemeIpPort,

    #[error("Invalid connect info.")]
    InvalidConnectInfo,

    #[error("Invalid reservation ID.")]
    InvalidReservationId,

    #[error("Invalid game server (reservation ID/connect info).")]
    InvalidGameServer,

    #[error("Time slot already taken.")]
    TimeSlotTaken,

    #[error("Game not found.")]
    GameNotFound,

    #[error("Game not hosted.")]
    GameNotHosted,

    #[error("RGL.gg profile not found.")]
    RglProfileNotFound,

    #[error("Team not in match.")]
    TeamNotInMatch,

    #[error("na.serveme.tf API key not set. Set one with `/config set serveme`.")]
    NoServemeApiKey,

    #[error(
        "No game format provided. Either set a default game format with `/config set game-format` or provide one in the command."
    )]
    NoGameFormat,

    #[error("No schedule channel set. Set one with `/config set schedule-channel`.")]
    NoScheduleChannel,

    #[error("No RGL team set. Set one with `/config set rgl-team`.")]
    NoRglTeam,
}

impl From<serenity::Error> for BotError {
    fn from(err: serenity::Error) -> Self {
        Self::Serenity(Box::new(err))
    }
}
