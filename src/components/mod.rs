use serenity::all::{ComponentInteraction, ComponentInteractionDataKind, Context};
use sqlx::query;
use time::OffsetDateTime;

use crate::{
    error::BotError,
    models::{GameFormat, Map},
    serveme::EditReservationRequest,
    utils::success_response,
    Bot, BotResult,
};

pub async fn run(bot: &Bot, ctx: &Context, interaction: &ComponentInteraction) -> BotResult {
    if let Some(custom_id) = interaction.data.custom_id.strip_prefix("map:") {
        map(bot, ctx, interaction, custom_id).await?;
    }

    Ok(())
}

pub async fn map(
    bot: &Bot,
    ctx: &Context,
    interaction: &ComponentInteraction,
    custom_id: &str,
) -> BotResult {
    let (idx, timestamp) = custom_id
        .split_once(':')
        .ok_or_else(|| BotError::InvalidComponentInteraction(custom_id.to_owned()))?;

    if !matches!(interaction.data.kind, ComponentInteractionDataKind::Button) {
        return Err(BotError::InvalidComponentInteraction(custom_id.to_owned()));
    }

    let idx = idx.parse::<u8>()?;
    let timestamp = OffsetDateTime::from_unix_timestamp(timestamp.parse::<i64>()?)?;

    let (guild, mut tx) = bot.get_guild_tx(interaction.guild_id).await?;

    let serveme_api_key = guild
        .serveme_api_key
        .as_deref()
        .ok_or_else(|| BotError::NoServemeApiKey)?;

    let row = query!(
        "SELECT reservation_id, map_1, map_2, game_format FROM scrims
        WHERE guild_id = $1 AND timestamp = $2",
        i64::from(guild.id),
        timestamp,
    )
    .fetch_one(&mut *tx)
    .await?;

    let reservation_id = row
        .reservation_id
        .ok_or_else(|| BotError::ScrimNotHosted(timestamp))?;

    let map = match idx {
        1 => row.map_1,
        2 => row.map_2,
        _ => return Err(BotError::InvalidComponentInteraction(custom_id.to_owned())),
    }
    .map(Map);

    let game_format = GameFormat::from(row.game_format);

    let server_config_id = map
        .as_ref()
        .and_then(|map| map.config_name_id(game_format))
        .map(|(_, id)| id);

    EditReservationRequest {
        starts_at: None,
        ends_at: None,
        first_map: map,
        server_config_id,
    }
    .send(&bot.http_client, serveme_api_key, reservation_id as _)
    .await?;

    interaction
        .create_response(ctx, success_response("Map changed."))
        .await?;

    Ok(())
}
