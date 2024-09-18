mod game;
mod guild;

use std::fmt::{self, Display, Formatter};

use serenity_commands::BasicOption;

pub use self::{game::*, guild::*};

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
pub enum GameFormat {
    Sixes,
    Highlander,
}

impl Display for GameFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Sixes => "Sixes",
            Self::Highlander => "Highlander",
        })
    }
}

impl From<i16> for GameFormat {
    fn from(format: i16) -> Self {
        match format {
            6 => Self::Sixes,
            _ => Self::Highlander,
        }
    }
}

impl From<GameFormat> for i16 {
    fn from(format: GameFormat) -> Self {
        match format {
            GameFormat::Sixes => 6,
            GameFormat::Highlander => 9,
        }
    }
}
