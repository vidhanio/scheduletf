use serenity::all::{ChannelId, GuildId};

use super::GameFormat;

pub struct DbGuild {
    #[allow(dead_code)]
    pub id: i64,
    pub rgl_team_id: Option<i32>,
    pub game_format: Option<i16>,
    pub games_channel_id: Option<i64>,
    pub serveme_api_key: Option<String>,
}

impl From<DbGuild> for Guild {
    fn from(db: DbGuild) -> Self {
        Self {
            id: GuildId::new(db.id as _),
            rgl_team_id: db.rgl_team_id.map(|id| id as _),
            game_format: db.game_format.map(Into::into),
            games_channel_id: db.games_channel_id.map(|id| ChannelId::new(id as _)),
            serveme_api_key: db.serveme_api_key,
        }
    }
}

#[derive(Debug)]
pub struct Guild {
    pub id: GuildId,
    pub rgl_team_id: Option<u32>,
    pub game_format: Option<GameFormat>,
    pub games_channel_id: Option<ChannelId>,
    pub serveme_api_key: Option<String>,
}
