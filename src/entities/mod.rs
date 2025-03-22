pub mod game;
pub mod team_guild;

macro_rules! discord_id {
    (? $Id:ident($DiscordId:ident)) => {
        discord_id!($Id($DiscordId));

        impl sea_orm::sea_query::Nullable for $Id {
            fn null() -> Value {
                <i64 as sea_orm::sea_query::Nullable>::null()
            }
        }
    };
    ($Id:ident($DiscordId:ident)) => {
        #[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
        pub struct $Id(pub $DiscordId);

        impl std::ops::Deref for $Id {
            type Target = $DiscordId;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<i64> for $Id {
            fn from(source: i64) -> Self {
                $Id($DiscordId::new(source as _))
            }
        }

        impl From<$Id> for i64 {
            fn from(source: $Id) -> Self {
                source.get() as _
            }
        }

        impl From<$DiscordId> for $Id {
            fn from(source: $DiscordId) -> Self {
                $Id(source)
            }
        }

        impl From<$Id> for $DiscordId {
            fn from(source: $Id) -> Self {
                source.0
            }
        }

        impl From<$Id> for Value {
            fn from(source: $Id) -> Self {
                i64::from(source).into()
            }
        }

        impl sea_orm::TryGetable for $Id {
            fn try_get_by<I: sea_orm::ColIdx>(
                res: &QueryResult,
                idx: I,
            ) -> Result<Self, sea_orm::TryGetError> {
                <i64 as sea_orm::TryGetable>::try_get_by(res, idx).map($Id::from)
            }
        }

        impl sea_orm::sea_query::ValueType for $Id {
            fn try_from(v: Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
                <i64 as sea_orm::sea_query::ValueType>::try_from(v).map($Id::from)
            }

            fn type_name() -> String {
                stringify!($Id).to_owned()
            }

            fn array_type() -> sea_orm::sea_query::ArrayType {
                sea_orm::sea_query::ArrayType::BigInt
            }

            fn column_type() -> sea_orm::sea_query::ColumnType {
                sea_orm::sea_query::ColumnType::BigInteger
            }
        }

        impl std::fmt::Display for $Id {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };
}

use discord_id;
