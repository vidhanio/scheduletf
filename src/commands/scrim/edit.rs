use paste::paste;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, IntoActiveModel, QuerySelect};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::{SubCommand, SubCommandGroup};
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        game::{self, ConnectInfo, Maps, ReservationId},
        team_guild::{self, GameFormat},
    },
    error::BotError,
    utils::success_embed,
};

macro_rules! edit_command {
    (
        $(
            $doc:literal
            $name:ident {
                $(#[$attr:meta])*
                $field:ident : $field_ty:ty,
            },
        )*
    ) => {
        paste! {
            #[derive(Debug, SubCommandGroup)]
            pub enum EditCommand {
                $(
                    #[doc = concat!("Edit the ", $doc, ".")]
                    #[command(autocomplete)]
                    $name([<Edit $name Command>]),
                )*
            }

            $(
                #[derive(Debug, SubCommand)]
                pub struct [<Edit $name Command>] {
                    /// The scrim to edit.
                    #[command(autocomplete)]
                    scrim: OffsetDateTime,

                    #[doc = concat!("The new ", $doc, ".")]
                    $(#[$attr])*
                    $field: $field_ty,
                }
            )*

            impl EditCommand {
                #[allow(clippy::too_many_lines)]
                pub async fn run(
                    self,
                    bot: &Bot,
                    ctx: &Context,
                    interaction: &CommandInteraction,
                ) -> BotResult {
                    interaction.defer_ephemeral(ctx).await?;

                    let (mut guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                    let datetime = match &self {
                        $(
                            Self::$name(cmd) => cmd.scrim,
                        )*
                    };

                    let game = guild.get_game(&tx, datetime).await?;

                    let game = match self {
                        $(
                            Self::$name(cmd) => {
                                cmd.run(&tx, &guild, game).await?
                            }
                        )*
                    };

                    let embed = game.embed(guild.serveme_api_key.as_ref()).await?;

                    guild.refresh_schedule(ctx, &tx).await?;

                    tx.commit().await?;

                    interaction
                        .edit_response(
                            &ctx,
                            EditInteractionResponse::new()
                                .embeds(vec![
                                    success_embed("Scrim updated."),
                                    embed,
                                ]),
                        )
                        .await?;

                    Ok(())
                }
            }
        }
    };
}

edit_command! {
    "date and time of the scrim"
    DateTime {
        #[command(autocomplete)]
        date_time: OffsetDateTime,
    },

    "opposing team's contact"
    Opponent {
        opponent: UserId,
    },

    "game format of the scrim"
    GameFormat {
        game_format: GameFormat,
    },

    "maps to be played"
    Maps {
        #[command(autocomplete)]
        maps: Option<Maps>,
    },

    "reservation ID of the server"
    ReservationId {
        #[command(autocomplete)]
        reservation_id: Option<ReservationId>,
    },

    "external connect info, if they are hosting"
    ConnectInfo {
        connect_info: Option<ConnectInfo>,
    },
}

impl EditDateTimeCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        _: &team_guild::Model,
        game: game::Model,
    ) -> BotResult<game::Model> {
        let mut game = game.into_active_model();

        game.timestamp.set_if_not_equals(self.date_time);

        Ok(game.update(tx).await?)
    }
}

impl EditOpponentCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        _: &team_guild::Model,
        game: game::Model,
    ) -> BotResult<game::Model> {
        let mut game = game.into_active_model();

        game.opponent_user_id
            .set_if_not_equals(self.opponent.into());

        Ok(game.update(tx).await?)
    }
}

impl EditGameFormatCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        _: &team_guild::Model,
        game: game::Model,
    ) -> BotResult<game::Model> {
        let mut game = game.into_active_model();

        game.game_format.set_if_not_equals(self.game_format);

        Ok(game.update(tx).await?)
    }
}

impl EditMapsCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        guild: &team_guild::Model,
        mut game: game::Model,
    ) -> BotResult<game::Model> {
        game.maps = Some(self.maps.unwrap_or_default());

        if game.reservation_id.is_some() {
            let api_key = guild.serveme_api_key()?;

            game.edit_reservation(api_key).await?;
        }

        let mut game = game.into_active_model();

        game.maps.reset();

        Ok(game.update(tx).await?)
    }
}

impl EditReservationIdCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        guild: &team_guild::Model,
        mut game: game::Model,
    ) -> BotResult<game::Model> {
        game.reservation_id = self.reservation_id;

        if game.reservation_id.is_some() {
            let api_key = guild.serveme_api_key()?;

            game.edit_reservation(api_key).await?;
        }

        let mut game = game.into_active_model();

        game.reservation_id.reset();
        game.server_ip_and_port.set_if_not_equals(None);
        game.server_password.set_if_not_equals(None);

        Ok(game.update(tx).await?)
    }
}

impl EditConnectInfoCommand {
    pub async fn run(
        self,
        tx: &DatabaseTransaction,
        _: &team_guild::Model,
        game: game::Model,
    ) -> BotResult<game::Model> {
        let mut game = game.into_active_model();

        game.reservation_id.set_if_not_equals(None);

        let (ip_and_port, password) = self
            .connect_info
            .map(|connect_info| (connect_info.ip_and_port, connect_info.password))
            .unzip();

        game.server_ip_and_port.set_if_not_equals(ip_and_port);
        game.server_password.set_if_not_equals(password);

        Ok(game.update(tx).await?)
    }
}

impl EditCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::DateTime(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Opponent(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::GameFormat(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::Maps(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::ReservationId(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::ConnectInfo(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}

macro_rules! impl_autocomplete_scrim {
    ($($name:ident),*) => {
        paste! {
            $(
                impl [<Edit $name CommandAutocomplete>] {
                    pub async fn autocomplete(
                        self,
                        bot: &Bot,
                        ctx: &Context,
                        interaction: &CommandInteraction,
                    ) -> BotResult {
                        let Self::Scrim { scrim, .. } = self;

                        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                        guild.
                            autocomplete_games(
                                ctx,
                                interaction,
                                tx,
                                &scrim,
                            )
                            .await
                    }
                }
            )*
        }
    };
}

impl_autocomplete_scrim!(Opponent, GameFormat, ConnectInfo);

impl EditDateTimeCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Scrim { scrim, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild.autocomplete_games(ctx, interaction, tx, &scrim).await
            }
            Self::DateTime { date_time, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_times(ctx, interaction, tx, &date_time)
                    .await
            }
        }
    }
}

impl EditMapsCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Scrim { scrim, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild.autocomplete_games(ctx, interaction, tx, &scrim).await
            }
            Self::Maps { maps, scrim, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                let game_format = if let Some(datetime) = scrim.into_value() {
                    Some(
                        game::Entity::find_by_id((guild.id, datetime))
                            .select_only()
                            .column(game::Column::GameFormat)
                            .into_tuple::<GameFormat>()
                            .one(&tx)
                            .await?
                            .ok_or(BotError::GameNotFound)?,
                    )
                } else {
                    None
                };

                guild
                    .autocomplete_maps(ctx, interaction, game_format, &maps)
                    .await
            }
        }
    }
}

impl EditReservationIdCommandAutocomplete {
    pub async fn autocomplete(
        self,
        bot: &Bot,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> BotResult {
        match self {
            Self::Scrim { scrim, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild.autocomplete_games(ctx, interaction, tx, &scrim).await
            }
            Self::ReservationId { reservation_id, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_reservations(ctx, interaction, tx, &reservation_id)
                    .await
            }
        }
    }
}
