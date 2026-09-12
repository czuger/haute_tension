//! One line of a hero's bag: a kind of thing, and how many of it he carries.
//!
//! Its own table rather than a list inside the game's row. **One row per line of
//! the bag**, as the sheet shows it — "4 rations" is one row with a count of
//! four — and never one row for the whole bag.
//!
//! Real columns: the id, the game the line belongs to, and when it was first
//! carried and last changed. The rest is the blob:
//!
//! ```text
//! element     the book's name for the thing: "sword", "ration", "key"
//! label       what the reader is shown, in French: "épée"
//! count       how many he carries; at zero the line leaves the bag, and the
//!             row is deleted
//! ```
//!
//! The rows are written through their game: a game owns them, in the order they
//! were first carried.

use std::fmt;

use crate::core::models::hybrid_document::{Columns, Field, HybridDocument, RowDict};

/// One line of a hero's bag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Item {
    columns: Columns,
    data: RowDict,
}

impl Item {
    /// The id the database counted up, or nothing before it is written.
    pub fn id(&self) -> Option<i64> {
        self.columns.int("id")
    }

    /// The game this line belongs to.
    pub fn game_id(&self) -> Option<i64> {
        self.columns.int("game_id")
    }

    /// The columns, for the writer that binds them.
    pub fn columns(&self) -> &Columns {
        &self.columns
    }
}

impl HybridDocument for Item {
    const TABLE: &'static str = "items";

    fn indexed_columns() -> &'static [&'static str] {
        &["id", "game_id", "created_at", "updated_at"]
    }

    fn empty() -> Self {
        Self::default()
    }

    fn column(&self, name: &str) -> Field {
        self.columns.get(name)
    }

    fn set_column(&mut self, name: &str, value: &Field) {
        self.columns.set(name, value);
    }

    fn data(&self) -> &RowDict {
        &self.data
    }

    fn set_data(&mut self, data: RowDict) {
        self.data = data;
    }
}

impl fmt::Display for Item {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = self.data.get("label").cloned().unwrap_or(Field::Null);
        let count = self.data.get("count").cloned().unwrap_or(Field::Null);
        write!(formatter, "{label} ×{count}")
    }
}

/// The `items` table, as SeaORM reads and writes it.
pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "items")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub game_id: Option<i64>,
        pub created_at: Option<String>,
        pub updated_at: Option<String>,
        #[sea_orm(column_type = "Json", nullable)]
        pub data: Option<Json>,
    }

    #[derive(Copy, Clone, Debug, EnumIter)]
    pub enum Relation {
        Game,
    }

    impl RelationTrait for Relation {
        fn def(&self) -> RelationDef {
            match self {
                Self::Game => Entity::belongs_to(super::super::game::entity::Entity)
                    .from(Column::GameId)
                    .to(super::super::game::entity::Column::Id)
                    .into(),
            }
        }
    }

    impl Related<super::super::game::entity::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Game.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
