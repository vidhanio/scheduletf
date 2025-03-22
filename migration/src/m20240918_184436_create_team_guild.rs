use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(TeamGuild::Table)
                    .col(big_integer(TeamGuild::Id).primary_key())
                    .col(integer_null(TeamGuild::RglTeamId))
                    .col(small_integer_null(TeamGuild::GameFormat))
                    .col(big_integer_null(TeamGuild::ScheduleChannelId))
                    .col(big_integer_null(TeamGuild::ScheduleMessageId))
                    .col(string_len_null(TeamGuild::ServemeApiKey, 32))
                    .take(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(TeamGuild::Table).take())
            .await
    }
}

#[derive(DeriveIden)]
pub enum TeamGuild {
    Table,

    Id,
    RglTeamId,
    GameFormat,
    ScheduleChannelId,
    ScheduleMessageId,
    ServemeApiKey,
}
