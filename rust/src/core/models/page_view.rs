//! The reading history: which page was asked for, and when.
//!
//! One row per visit. The history is bounded when it is read rather than when it
//! is written (see `core::db::last_pages`), so a visit is only ever an insert.
//!
//! The id is an integer the database counts up: a visit has no id of its own and
//! nothing ever looks one up by id. `book`, `game_id` and `viewed_at` are real
//! columns because every read filters on one of the first two and sorts on the
//! third; the page number itself is only ever read back, so it sits in the blob:
//!
//! ```text
//! page    the page number asked for, whether or not the book has it
//! ```
//!
//! A visit is written once and never changed, so it carries no `created_at` or
//! `updated_at`: `viewed_at` is when it was written.

use std::fmt;

use crate::core::models::hybrid_document::{Columns, Field, HybridDocument, RowDict};

/// One page, asked for once.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageView {
    columns: Columns,
    data: RowDict,
}

impl PageView {
    /// The id the database counted up.
    pub fn id(&self) -> Option<i64> {
        self.columns.int("id")
    }

    /// The page number asked for, out of the blob.
    pub fn page(&self) -> String {
        self.data
            .get("page")
            .cloned()
            .unwrap_or(Field::Null)
            .to_string()
    }

    /// The columns, for the writer that binds them.
    pub fn columns(&self) -> &Columns {
        &self.columns
    }
}

impl HybridDocument for PageView {
    const TABLE: &'static str = "page_views";

    fn indexed_columns() -> &'static [&'static str] {
        &["id", "book", "game_id", "viewed_at"]
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

impl fmt::Display for PageView {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{} p.{} @ {}",
            self.columns.get("book"),
            self.page(),
            self.columns.get("viewed_at"),
        )
    }
}

/// The `page_views` table, as SeaORM reads and writes it.
pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "page_views")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub book: Option<String>,
        pub game_id: Option<i64>,
        pub viewed_at: Option<String>,
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
