//! One play-through: the hero, and the fight he is in the middle of.
//!
//! The second thing this application stores, and the first that is *state*
//! rather than a trace. The book stays on disk and the reading history stays a
//! log; a game is the row that is read, changed and written back.
//!
//! The id is an integer the database counts up, and never reuses. It travels in
//! the session cookie, whose signature is what keeps a reader from writing
//! another hero's number into it; never reusing one is what keeps an old cookie
//! from ever naming somebody else's hero.
//!
//! Real columns: the id; the book every query is scoped to; the hero's Force,
//! Vie and purse, which may never go below zero and which the database holds to
//! that; when he died, which the memorial filters and sorts on; and when the row
//! was created and last changed. His bag is rows of their own, [`Item`].
//! **Everything else is in the blob**:
//!
//! ```text
//! mode            "normal" or "easy" — which set of rules rolled him up
//! force_dice, vie_dice        the throws that made him, for the sheet
//! gold_dice
//! pending         [{element, label, amount, condition, note, sign, page}]
//! combat          null while he is reading, or {page, fight_type, status,
//!                 on_victory, on_defeat, on_flee, has_special_rules,
//!                 enemies: [...], assaults: [...]}
//! died_on_page, died_of       set with `died_at`, see below
//! legacy_id       the uuid a game created before migration 001 was known by,
//!                 kept so an old log line can still be traced to it
//! ```
//!
//! `died_at` is unset while he lives, which is what tells a game apart from an
//! epitaph: a hero abandoned in good health is simply left behind, and does not
//! join the fallen.

use std::fmt;

use crate::core::models::hybrid_document::{Columns, Field, HybridDocument, RowDict};
use crate::core::models::item::Item;

/// A hero, rolled once, and the fight he is in the middle of.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    columns: Columns,
    data: RowDict,
    pub items: Vec<Item>,
}

impl Game {
    /// The id the database counted up, or nothing before it is written.
    pub fn id(&self) -> Option<i64> {
        self.columns.int("id")
    }

    /// The book this play-through is of.
    pub fn book(&self) -> Option<String> {
        self.columns.text("book")
    }

    /// Whether this hero has already been laid to rest.
    pub fn is_dead(&self) -> bool {
        !self.columns.get("died_at").is_null()
    }

    /// The columns, for the writer that binds them.
    pub fn columns(&self) -> &Columns {
        &self.columns
    }
}

impl HybridDocument for Game {
    const TABLE: &'static str = "games";

    fn indexed_columns() -> &'static [&'static str] {
        &[
            "id",
            "book",
            "force",
            "vie_max",
            "vie_actuelle",
            "gold",
            "died_at",
            "created_at",
            "updated_at",
        ]
    }

    fn child_tables() -> &'static [(&'static str, &'static str)] {
        &[("items", Item::TABLE)]
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

    fn children(&self, relationship: &str) -> Vec<RowDict> {
        match relationship {
            "items" => self.items.iter().map(Item::to_dict).collect(),
            _ => Vec::new(),
        }
    }

    /// The child rows a list of child dicts describes, reusing those it names.
    ///
    /// The existing child an entry's `id` names is updated, an entry without one
    /// is a new row, and a child no entry names is left out — which `core::db`
    /// turns into a delete.
    fn set_children(&mut self, relationship: &str, entries: &[RowDict]) {
        if relationship != "items" {
            return;
        }
        self.items = entries
            .iter()
            .map(|entry| {
                let named = entry.get("id").and_then(Field::as_int);
                match named.and_then(|id| self.items.iter().find(|item| item.id() == Some(id))) {
                    Some(existing) => {
                        let mut child = existing.clone();
                        child.update_from_dict(entry);
                        child
                    }
                    None => Item::from_dict(entry),
                }
            })
            .collect();
    }
}

impl fmt::Display for Game {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state = if self.is_dead() {
            "mort".to_string()
        } else {
            format!(
                "Vie {}/{}",
                self.columns.get("vie_actuelle"),
                self.columns.get("vie_max")
            )
        };
        let mode = self.data.get("mode").cloned().unwrap_or(Field::Null);
        write!(
            formatter,
            "{} ({mode}) — Force {}, {state}",
            self.columns.get("book"),
            self.columns.get("force"),
        )
    }
}

/// The `games` table, as SeaORM reads and writes it.
pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "games")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub book: Option<String>,
        pub force: Option<i64>,
        pub vie_max: Option<i64>,
        pub vie_actuelle: Option<i64>,
        pub gold: Option<i64>,
        pub died_at: Option<String>,
        pub created_at: Option<String>,
        pub updated_at: Option<String>,
        #[sea_orm(column_type = "Json", nullable)]
        pub data: Option<Json>,
    }

    #[derive(Copy, Clone, Debug, EnumIter)]
    pub enum Relation {
        Items,
    }

    impl RelationTrait for Relation {
        fn def(&self) -> RelationDef {
            match self {
                Self::Items => Entity::has_many(super::super::item::entity::Entity).into(),
            }
        }
    }

    impl Related<super::super::item::entity::Entity> for Entity {
        fn to() -> RelationDef {
            Relation::Items.def()
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
