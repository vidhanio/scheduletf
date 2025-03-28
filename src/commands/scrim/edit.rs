use paste::paste;
use sea_orm::{ActiveModelTrait, EntityTrait, IntoActiveModel, QuerySelect};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse, UserId};
use serenity_commands::{SubCommand, SubCommandGroup};
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        ConnectInfo, GameFormat, MapList, ReservationId,
        game::{self, Game, GameServer, Scrim},
        team_guild,
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

                    let scrim = guild.get_game::<Scrim>(&tx, datetime).await?;

                    let game = match self {
                        $(
                            Self::$name(cmd) => {
                                cmd.run(&guild, scrim).await?
                            }
                        )*
                    }
                    .update(&tx)
                    .await?;

                    let embed = Game::try_from(game)?.embed(&guild).await?;

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
        opponent: Option<UserId>,
    },

    "game format of the scrim"
    GameFormat {
        game_format: GameFormat,
    },

    "maps to be played"
    Maps {
        #[command(autocomplete)]
        maps: Option<MapList>,
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
    #[allow(clippy::unused_async)]
    pub async fn run(
        self,
        guild: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        scrim.timestamp = self.date_time;

        if scrim.server.is_hosted() {
            let api_key = guild.serveme_api_key()?;

            scrim.edit_reservation(api_key).await?;
        }

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::Timestamp);

        Ok(active_model)
    }
}

impl EditOpponentCommand {
    #[allow(clippy::unused_async)]
    pub async fn run(
        self,
        _: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        scrim.details.opponent_user_id = self.opponent.map(Into::into);

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::OpponentUserId);

        Ok(active_model)
    }
}

impl EditGameFormatCommand {
    #[allow(clippy::unused_async)]
    pub async fn run(
        self,
        guild: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        scrim.details.game_format = self.game_format;

        if scrim.server.is_hosted() {
            let api_key = guild.serveme_api_key()?;

            scrim.edit_reservation(api_key).await?;
        }

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::GameFormat);

        Ok(active_model)
    }
}

impl EditMapsCommand {
    pub async fn run(
        self,
        guild: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        scrim.details.maps = self.maps.unwrap_or_default();

        if scrim.server.is_hosted() {
            let api_key = guild.serveme_api_key()?;

            scrim.edit_reservation(api_key).await?;
        }

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::Maps);

        Ok(active_model)
    }
}

impl EditReservationIdCommand {
    pub async fn run(
        self,
        guild: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        if let Some(reservation_id) = self.reservation_id {
            scrim.server = GameServer::Hosted(reservation_id);
        } else if scrim.server.is_hosted() {
            scrim.server = GameServer::Undecided;
        }

        if scrim.server.is_hosted() {
            let api_key = guild.serveme_api_key()?;

            scrim.edit_reservation(api_key).await?;
        }

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::ReservationId);
        active_model.reset(game::Column::ConnectInfo);

        Ok(active_model)
    }
}

impl EditConnectInfoCommand {
    #[allow(clippy::unused_async)]
    pub async fn run(
        self,
        _: &team_guild::Model,
        mut scrim: Game<Scrim>,
    ) -> BotResult<game::ActiveModel> {
        if let Some(connect_info) = self.connect_info {
            scrim.server = GameServer::Joined(connect_info);
        } else if scrim.server.is_joined() {
            scrim.server = GameServer::Undecided;
        }

        let mut active_model = scrim.into_active_model();
        active_model.reset(game::Column::ReservationId);
        active_model.reset(game::Column::ConnectInfo);

        Ok(active_model)
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

                        guild.autocomplete_games::<Scrim>(
                                ctx,
                                interaction,
                                tx,
                                None,
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

                guild
                    .autocomplete_games::<Scrim>(ctx, interaction, tx, None, &scrim)
                    .await
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

                guild
                    .autocomplete_games::<Scrim>(ctx, interaction, tx, None, &scrim)
                    .await
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

                guild
                    .autocomplete_games::<Scrim>(ctx, interaction, tx, None, &scrim)
                    .await
            }
            Self::ReservationId { reservation_id, .. } => {
                let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                guild
                    .autocomplete_reservations::<Scrim>(
                        ctx,
                        interaction,
                        tx,
                        |r| !r.status.is_ended(),
                        &reservation_id,
                    )
                    .await
            }
        }
    }
}
