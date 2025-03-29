use sea_orm_migration::{prelude::*, schema::*};

use crate::m20240918_184436_create_team_guild::TeamGuild;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TeamGuild::Table)
                    .add_column(string_null(ScrimDivision))
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(TeamGuild::Table)
                    .drop_column(ScrimDivision)
                    .take(),
            )
            .await
    }
}

#[derive(DeriveIden)]
pub struct ScrimDivision;
