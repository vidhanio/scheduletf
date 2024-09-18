#![allow(dead_code)]

use serenity::all::{
    ChannelId, ComponentInteraction, ComponentInteractionDataKind, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, GenericId, RoleId, UserId,
};
use time::{OffsetDateTime, UtcOffset};

use crate::{error::BotError, BotResult};

macro_rules! handle_error {
    ($result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                tracing::error!(?error);

                return;
            }
        }
    };
    ($ctx:expr, $interaction:expr, $result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
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

#[derive(Clone, Copy, Debug)]
pub enum ComponentInteractionDataType {
    Button,
    StringSelect,
    UserSelect,
    RoleSelect,
    MentionableSelect,
    ChannelSelect,
    Unknown,
}

impl From<&ComponentInteractionDataKind> for ComponentInteractionDataType {
    fn from(kind: &ComponentInteractionDataKind) -> Self {
        match kind {
            ComponentInteractionDataKind::Button => Self::Button,
            ComponentInteractionDataKind::StringSelect { .. } => Self::StringSelect,
            ComponentInteractionDataKind::UserSelect { .. } => Self::UserSelect,
            ComponentInteractionDataKind::RoleSelect { .. } => Self::RoleSelect,
            ComponentInteractionDataKind::MentionableSelect { .. } => Self::MentionableSelect,
            ComponentInteractionDataKind::ChannelSelect { .. } => Self::ChannelSelect,
            ComponentInteractionDataKind::Unknown(..) => Self::Unknown,
        }
    }
}
pub trait SelectMenuDataKind: Sized {
    const DATA_TYPE: ComponentInteractionDataType;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>>;
}

impl SelectMenuDataKind for String {
    const DATA_TYPE: ComponentInteractionDataType = ComponentInteractionDataType::StringSelect;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>> {
        if let ComponentInteractionDataKind::StringSelect { values } = data {
            Some(values)
        } else {
            None
        }
    }
}

impl SelectMenuDataKind for UserId {
    const DATA_TYPE: ComponentInteractionDataType = ComponentInteractionDataType::UserSelect;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>> {
        if let ComponentInteractionDataKind::UserSelect { values } = data {
            Some(values)
        } else {
            None
        }
    }
}

impl SelectMenuDataKind for RoleId {
    const DATA_TYPE: ComponentInteractionDataType = ComponentInteractionDataType::RoleSelect;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>> {
        if let ComponentInteractionDataKind::RoleSelect { values } = data {
            Some(values)
        } else {
            None
        }
    }
}

impl SelectMenuDataKind for GenericId {
    const DATA_TYPE: ComponentInteractionDataType = ComponentInteractionDataType::MentionableSelect;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>> {
        if let ComponentInteractionDataKind::MentionableSelect { values } = data {
            Some(values)
        } else {
            None
        }
    }
}

impl SelectMenuDataKind for ChannelId {
    const DATA_TYPE: ComponentInteractionDataType = ComponentInteractionDataType::ChannelSelect;

    fn values(data: &ComponentInteractionDataKind) -> Option<&Vec<Self>> {
        if let ComponentInteractionDataKind::ChannelSelect { values } = data {
            Some(values)
        } else {
            None
        }
    }
}

pub fn get_single_from_select<'a, T: SelectMenuDataKind>(
    component_name: &str,
    interaction: &'a ComponentInteraction,
) -> BotResult<&'a T> {
    T::values(&interaction.data.kind).map_or_else(
        || {
            Err(BotError::IncorrectInteractionDataKind {
                component: component_name.into(),
                expected: T::DATA_TYPE,
                got: (&interaction.data.kind).into(),
            })
        },
        |values| match values.as_slice() {
            [value] => Ok(value),
            not_one_value => Err(BotError::IncorrectNumberOfItems {
                component: component_name.into(),
                expected: 1,
                got: not_one_value.len(),
            }),
        },
    )
}

pub trait ScrimOffsetExtension {
    fn now_scrim() -> Self;

    fn scrim_offset(&self) -> UtcOffset;

    fn replace_with_scrim_offset(&self) -> Self;

    fn to_scrim_offset(&self) -> Self;

    fn short_date(&self) -> String;
}

impl ScrimOffsetExtension for OffsetDateTime {
    fn scrim_offset(&self) -> UtcOffset {
        let ny = tzdb::time_zone::america::NEW_YORK;

        let local_time_type = ny
            .find_local_time_type(self.unix_timestamp())
            .expect("local time type exists");

        UtcOffset::from_whole_seconds(local_time_type.ut_offset()).expect("offset is valid")
    }

    fn now_scrim() -> Self {
        let now = Self::now_utc();

        now.to_scrim_offset()
    }

    fn replace_with_scrim_offset(&self) -> Self {
        self.replace_offset(self.scrim_offset())
    }

    fn to_scrim_offset(&self) -> Self {
        self.to_offset(self.scrim_offset())
    }

    fn short_date(&self) -> String {
        let this = self.to_scrim_offset();

        let weekday = this.weekday();
        let hour_24 = this.hour();
        let hour = if hour_24 == 0 {
            12
        } else if hour_24 > 12 {
            hour_24 - 12
        } else {
            hour_24
        };
        let minute = this.minute();

        format!("{weekday} {hour}:{minute:02}")
    }
}
