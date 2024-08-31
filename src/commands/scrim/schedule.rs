use serenity::all::{CommandInteraction, Context, CreateInteractionResponse};
use serenity_commands::{BasicOption, SubCommand};
use sqlx::query;
use time::{
    macros::{offset, time},
    Date, OffsetDateTime, Weekday,
};

use crate::{error::BotError, utils::success_message_title, Bot, BotResult};

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
pub enum Format {
    Sixes,
    Highlander,
}

impl Format {
    const fn to_int(self) -> u8 {
        match self {
            Self::Sixes => 6,
            Self::Highlander => 9,
        }
    }

    const fn short_name(self) -> &'static str {
        match self {
            Self::Sixes => "6s",
            Self::Highlander => "HL",
        }
    }
}

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
pub enum Day {
    Today,
    Sunday,
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}

impl From<Day> for Date {
    fn from(day: Day) -> Self {
        let now_date = OffsetDateTime::now_utc().to_offset(offset!(-5)).date();

        match day {
            Day::Today => now_date,
            Day::Sunday => now_date.next_occurrence(Weekday::Sunday),
            Day::Monday => now_date.next_occurrence(Weekday::Monday),
            Day::Tuesday => now_date.next_occurrence(Weekday::Tuesday),
            Day::Wednesday => now_date.next_occurrence(Weekday::Wednesday),
            Day::Thursday => now_date.next_occurrence(Weekday::Thursday),
            Day::Friday => now_date.next_occurrence(Weekday::Friday),
            Day::Saturday => now_date.next_occurrence(Weekday::Saturday),
        }
    }
}

#[derive(Copy, Clone, Debug, BasicOption)]
#[choice(option_type = "string")]
#[allow(clippy::enum_variant_names)]
pub enum Time {
    #[choice(name = "7:30 PM")]
    SevenThirty,

    #[choice(name = "8:30 PM")]
    EightThirty,

    #[choice(name = "9:30 PM")]
    NineThirty,

    #[choice(name = "10:30 PM")]
    TenThirty,
}

impl From<Time> for time::Time {
    fn from(time: Time) -> Self {
        match time {
            Time::SevenThirty => time!(19:30),
            Time::EightThirty => time!(20:30),
            Time::NineThirty => time!(21:30),
            Time::TenThirty => time!(22:30),
        }
    }
}

#[derive(Clone, Debug, SubCommand)]
pub struct ScheduleCommand {
    /// The game format of the scrimmage.
    format: Format,

    /// The next day of the week the scrimmage is scheduled for.
    day: Day,

    /// The time the scrimmage is scheduled for.
    time: Time,

    /// Whether the scrimmage is hosted by us.
    hosted: bool,

    /// The first map to be played.
    map_1: String,

    /// The second map to be played.
    map_2: String,

    /// Opposing team name or team leader.
    opponent: String,
}

impl ScheduleCommand {
    pub async fn run(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        let guild_id = interaction.guild_id.ok_or(BotError::NoGuild)?;

        let format = self.format.to_int();

        let timestamp =
            OffsetDateTime::new_in_offset(self.day.into(), self.time.into(), offset!(-5));

        query!(
            "INSERT INTO scrims (guild_id, format, timestamp, hosted, map_1, map_2, opponent) VALUES ($1, $2, $3, $4, $5, $6, $7)",
            i64::from(guild_id),
            i16::from(format),
            timestamp,
            self.hosted,
            &self.map_1,
            &self.map_2,
            &self.opponent
        ).execute(&bot.pool).await?;

        interaction
            .create_response(
                &ctx,
                CreateInteractionResponse::Message(success_message_title("Scrimmage Scheduled")),
            )
            .await?;

        Ok(())
    }
}
