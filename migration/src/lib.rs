pub use sea_orm_migration::prelude::*;

mod m20240918_184436_create_team_guild;
mod m20240918_185310_create_game;
mod m20250329_023624_add_lfs_div_column;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240918_184436_create_team_guild::Migration),
            Box::new(m20240918_185310_create_game::Migration),
            Box::new(m20250329_023624_add_lfs_div_column::Migration),
        ]
    }
}
