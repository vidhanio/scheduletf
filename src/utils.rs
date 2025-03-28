#![allow(dead_code)]

use serenity::all::{CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage};
use time::{Date, OffsetDateTime, Time, UtcOffset};

use crate::error::BotError;

macro_rules! handle_error {
    ($ctx:expr, $interaction:expr, $result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error);

                if $interaction
                    .create_response(
                        &$ctx,
                        serenity::all::CreateInteractionResponse::Message(
                            crate::utils::error_message(&error),
                        ),
                    )
                    .await
                    .is_err()
                {
                    if let Err(error) = $interaction
                        .edit_response(
                            &$ctx,
                            serenity::all::EditInteractionResponse::new()
                                .add_embed(crate::utils::error_embed(&error)),
                        )
                        .await
                    {
                        tracing::error!(?error, "could not create or edit response");
                    }
                }

                return;
            }
        }
    };
}
pub(crate) use handle_error;

mod tf2_colours {
    #![allow(clippy::unreadable_literal)]

    use serenity::all::Colour;

    pub const ORANGE: Colour = Colour(0xCF7336);

    pub const GREEN: Colour = Colour(0x729E42);
    pub const YELLOW: Colour = Colour(0xE7B53B);
    pub const RED: Colour = Colour(0xB8383B);
}

pub fn create_message() -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new().ephemeral(true)
}

pub fn embed(title: impl Into<String>) -> CreateEmbed {
    CreateEmbed::new().title(title).colour(tf2_colours::ORANGE)
}

pub fn error_embed(error: &BotError) -> CreateEmbed {
    embed("Error")
        .description(error.to_string())
        .color(tf2_colours::RED)
}

pub fn error_message(error: &BotError) -> CreateInteractionResponseMessage {
    create_message().embed(error_embed(error)).ephemeral(true)
}

pub fn warning_embed(description: impl Into<String>) -> CreateEmbed {
    embed("Warning")
        .description(description)
        .color(tf2_colours::YELLOW)
}

pub fn warning_message(description: impl Into<String>) -> CreateInteractionResponseMessage {
    create_message()
        .embed(warning_embed(description))
        .ephemeral(true)
}

pub fn success_embed(description: impl Into<String>) -> CreateEmbed {
    embed("Success")
        .description(description)
        .color(tf2_colours::GREEN)
}

pub fn success_message(description: impl Into<String>) -> CreateInteractionResponseMessage {
    create_message()
        .embed(success_embed(description))
        .ephemeral(true)
}

pub fn success_response(description: impl Into<String>) -> CreateInteractionResponse {
    CreateInteractionResponse::Message(success_message(description))
}

pub trait OffsetDateTimeEtExt {
    fn new_et(date: Date, time: Time) -> Self;

    fn now_et() -> Self;

    fn et_offset(&self) -> UtcOffset;

    fn replace_with_et_offset(&self) -> Self;

    fn to_et_offset(&self) -> Self;

    fn string_et(&self) -> String;

    fn string_et_relative(&self) -> String;

    fn date_et(&self) -> Date;

    fn time_et(&self) -> Time;
}

impl OffsetDateTimeEtExt for OffsetDateTime {
    fn new_et(date: Date, time: Time) -> Self {
        Self::new_utc(date, time).replace_with_et_offset()
    }

    fn et_offset(&self) -> UtcOffset {
        let ny = tzdb::time_zone::america::NEW_YORK;

        let local_time_type = ny.find_local_time_type(self.unix_timestamp()).unwrap();

        UtcOffset::from_whole_seconds(local_time_type.ut_offset()).unwrap()
    }

    fn now_et() -> Self {
        let now = Self::now_utc();

        now.to_et_offset()
    }

    fn replace_with_et_offset(&self) -> Self {
        self.replace_offset(self.et_offset())
    }

    fn to_et_offset(&self) -> Self {
        self.to_offset(self.et_offset())
    }

    fn string_et(&self) -> String {
        let this = self.to_et_offset();

        let weekday = this.weekday();
        let month = this.month();
        let day = this.day();
        let hour_24 = this.hour();
        let hour = if hour_24 == 0 {
            12
        } else if hour_24 > 12 {
            hour_24 - 12
        } else {
            hour_24
        };
        let minute = this.minute();
        let ampm = if hour_24 >= 12 { "PM" } else { "AM" };

        format!("{weekday}, {month} {day} at {hour}:{minute:02} {ampm}")
    }

    fn string_et_relative(&self) -> String {
        let this = self.to_et_offset();

        let now_date = Self::now_et().date();
        let date = if this.date() == now_date {
            "Today".to_owned()
        } else if this.date() == now_date.next_day().unwrap() {
            "Tomorrow".to_owned()
        } else {
            format!("{}, {} {}", this.weekday(), this.month(), this.day())
        };

        let hour_24 = this.hour();
        let hour = if hour_24 == 0 {
            12
        } else if hour_24 > 12 {
            hour_24 - 12
        } else {
            hour_24
        };
        let minute = this.minute();
        let ampm = if hour_24 >= 12 { "PM" } else { "AM" };

        format!("{date} at {hour}:{minute:02} {ampm}")
    }

    fn date_et(&self) -> Date {
        self.to_et_offset().date()
    }

    fn time_et(&self) -> Time {
        self.to_et_offset().time()
    }
}

pub fn date_string(date: Date) -> String {
    let weekday = date.weekday();
    let month = date.month();
    let day = date.day();

    format!("{weekday}, {month} {day}")
}

pub fn time_string(time: Time) -> String {
    let hour_24 = time.hour();
    let hour = if hour_24 == 0 {
        12
    } else if hour_24 > 12 {
        hour_24 - 12
    } else {
        hour_24
    };
    let minute = time.minute();
    let ampm = if hour_24 >= 12 { "PM" } else { "AM" };

    format!("{hour}:{minute:02} {ampm}")
}
