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
                    .col(integer_null(Game::ReservationId))
                    .col(string_null(Game::ConnectInfo))
                    .col(big_integer_null(Game::OpponentUserId))
                    .col(small_integer_null(Game::GameFormat))
                    .col(array_null(Game::Maps, ColumnType::string(None)))
                    .col(integer_null(Game::RglMatchId))
                    .primary_key(Index::create().col(Game::GuildId).col(Game::Timestamp))
                    .foreign_key(
                        ForeignKeyCreateStatement::new()
                            .from(Game::Table, Game::GuildId)
                            .to(TeamGuild::Table, TeamGuild::Id),
                    )
                    .check(
                        (Expr::col(Game::ReservationId).is_null())
                            .or(Expr::col(Game::ConnectInfo).is_null()),
                    )
                    .check(
                        // scrim
                        ((Expr::col(Game::GameFormat).is_not_null())
                            .and(Expr::col(Game::Maps).is_not_null())
                            .and(Expr::col(Game::RglMatchId).is_null()))
                        // match
                        .or((Expr::col(Game::OpponentUserId).is_null())
                            .and(Expr::col(Game::GameFormat).is_null())
                            .and(Expr::col(Game::Maps).is_null())
                            .and(Expr::col(Game::RglMatchId).is_not_null())),
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
    ReservationId,
    ConnectInfo,

    OpponentUserId,
    GameFormat,
    Maps,

    RglMatchId,
}
