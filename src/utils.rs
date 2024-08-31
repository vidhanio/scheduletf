#![allow(dead_code)]

use serenity::all::{
    colours, ChannelId, CommandInteraction, ComponentInteraction, ComponentInteractionDataKind,
    Context, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse, GenericId, RoleId, UserId,
};

use crate::{error::BotError, BotResult};

pub fn error_embed(error: &BotError) -> CreateEmbed {
    CreateEmbed::new()
        .title("Error")
        .description(format!("```\n{error}\n```"))
        .color(colours::css::DANGER)
}

pub fn error_message(error: &BotError) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .embed(error_embed(error))
        .ephemeral(true)
}

pub async fn handle_error_inner(error: BotError, interaction: &CommandInteraction, ctx: &Context) {
    tracing::error!(?error);

    if interaction
        .create_response(
            &ctx,
            CreateInteractionResponse::Message(error_message(&error)),
        )
        .await
        .is_err()
    {
        if let Err(error) = interaction
            .edit_response(
                &ctx,
                EditInteractionResponse::new().add_embed(error_embed(&error)),
            )
            .await
        {
            tracing::error!(?error, "could not create or edit response");
        }
    }
}

macro_rules! handle_error {
    ($ctx:expr, $interaction:expr, $result:expr) => {
        match $result {
            Ok(value) => value,
            Err(error) => {
                crate::utils::handle_error_inner(error, &$interaction, &$ctx).await;

                return;
            }
        }
    };
}
pub(crate) use handle_error;

pub fn warning_embed(title: impl Into<String>) -> CreateEmbed {
    CreateEmbed::new().title(title).color(colours::css::WARNING)
}

pub fn warning_message(title: impl Into<String>) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .embed(warning_embed(title))
        .ephemeral(true)
}

pub fn success_embed(title: impl Into<String>) -> CreateEmbed {
    CreateEmbed::new()
        .title(title)
        .color(colours::css::POSITIVE)
}

pub fn success_message(
    title: impl Into<String>,
    body: impl Into<String>,
) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .embed(success_embed(title).description(body))
        .ephemeral(true)
}

pub fn success_message_title(title: impl Into<String>) -> CreateInteractionResponseMessage {
    CreateInteractionResponseMessage::new()
        .embed(success_embed(title))
        .ephemeral(true)
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
