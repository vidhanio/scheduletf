use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240918_184436_create_team_guild::TeamGuild;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Game::Table)
                    .col(big_integer(Game::GuildId))
                    .col(timestamp_with_time_zone(Game::Timestamp))
                    .col(small_integer(Game::GameFormat))
                    .col(big_integer(Game::OpponentUserId))
                    .col(integer_null(Game::ReservationId))
                    .col(string_null(Game::ServerIpAndPort))
                    .col(string_null(Game::ServerPassword))
                    .col(array_null(Game::Maps, ColumnType::string(None)))
                    .col(integer_null(Game::RglMatchId))
                    .primary_key(Index::create().col(Game::GuildId).col(Game::Timestamp))
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Game::Table, Game::GuildId)
                            .to(TeamGuild::Table, TeamGuild::Id),
                    )
                    .check(
                        // hosted
                        ((Expr::col(Game::ReservationId).is_not_null())
                            .and(Expr::col(Game::ServerIpAndPort).is_null())
                            .and(Expr::col(Game::ServerPassword).is_null()))
                        // joined
                        .or((Expr::col(Game::ReservationId).is_null())
                            .and(Expr::col(Game::ServerIpAndPort).is_not_null())
                            .and(Expr::col(Game::ServerPassword).is_not_null()))
                        // undecided
                        .or((Expr::col(Game::ReservationId).is_null())
                            .and(Expr::col(Game::ServerIpAndPort).is_null())
                            .and(Expr::col(Game::ServerPassword).is_null())),
                    )
                    .check(
                        // official
                        ((Expr::col(Game::Maps).is_null())
                            .and(Expr::col(Game::RglMatchId).is_not_null()))
                        // scrim
                        .or((Expr::col(Game::Maps).is_not_null())
                            .and(Expr::col(Game::RglMatchId).is_null())),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Game::Table).take())
            .await
    }
}

#[derive(DeriveIden)]
#[allow(clippy::enum_variant_names)]
pub enum Game {
    Table,

    GuildId,
    Timestamp,
    GameFormat,
    OpponentUserId,
    ReservationId,
    ServerIpAndPort,
    ServerPassword,
    Maps,
    RglMatchId,
}
