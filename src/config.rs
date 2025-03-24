use std::{
    collections::HashSet,
    fmt::{self, Debug, Formatter},
};

use serde::Deserialize;
use serenity::all::GuildId;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub discord_bot_token: String,
    pub database_url: String,
    pub guilds: Option<HashSet<GuildId>>,
    #[serde(default)]
    pub production: bool,
}

impl Config {
    pub fn from_env() -> envy::Result<Self> {
        envy::from_env()
    }
}

impl Debug for Config {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Config")
            .field("guilds", &self.guilds)
            .field("production", &self.production)
            .finish_non_exhaustive()
    }
}
