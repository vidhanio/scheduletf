use paste::paste;
use sea_orm::{ActiveModelTrait, IntoActiveModel};
use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use serenity_commands::{SubCommand, SubCommandGroup};
use time::OffsetDateTime;

use crate::{
    Bot, BotResult,
    entities::{
        ConnectInfo, ReservationId,
        game::{self, Game, GameServer, Match},
        team_guild,
    },
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
                    /// The match to edit.
                    #[command(autocomplete)]
                    match_: OffsetDateTime,

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
                            Self::$name(cmd) => cmd.match_,
                        )*
                    };

                    let match_ = guild.get_game::<Match>(&tx, datetime).await?;

                    let game = match self {
                        $(
                            Self::$name(cmd) => {
                                cmd.run(&guild, match_).await?
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
                                    success_embed("Match updated."),
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
    "reservation ID of the server"
    ReservationId {
        reservation_id: Option<ReservationId>,
    },

    "external connect info, if they are hosting"
    ConnectInfo {
        connect_info: Option<ConnectInfo>,
    },
}

impl EditReservationIdCommand {
    pub async fn run(
        self,
        guild: &team_guild::Model,
        mut match_: Game<Match>,
    ) -> BotResult<game::ActiveModel> {
        if let Some(reservation_id) = self.reservation_id {
            match_.server = GameServer::Hosted(reservation_id);
        } else if match_.server.is_hosted() {
            match_.server = GameServer::Undecided;
        }

        if match_.server.is_hosted() {
            let api_key = guild.serveme_api_key()?;

            match_.edit_reservation(api_key).await?;
        }

        let mut active_model = match_.into_active_model();
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
        mut match_: Game<Match>,
    ) -> BotResult<game::ActiveModel> {
        if let Some(connect_info) = self.connect_info {
            match_.server = GameServer::Joined(connect_info);
        } else if match_.server.is_joined() {
            match_.server = GameServer::Undecided;
        }

        let mut active_model = match_.into_active_model();
        active_model.reset(game::Column::ReservationId);

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
            Self::ReservationId(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
            Self::ConnectInfo(cmd) => cmd.autocomplete(bot, ctx, interaction).await,
        }
    }
}

macro_rules! impl_autocomplete_match {
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
                        let Self::Match { match_, .. } = self;

                        let (guild, tx) = bot.get_guild_tx(interaction.guild_id).await?;

                        guild.autocomplete_games::<Match>(
                                ctx,
                                interaction,
                                tx,
                                &match_,
                            )
                            .await
                    }
                }
            )*
        }
    };
}

impl_autocomplete_match!(ReservationId, ConnectInfo);
