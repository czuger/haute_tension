//! A page a reader flagged as having a problem, and what was said about it.
//!
//! One row per page and per book, whatever the number of reports: flagging a
//! page that is already flagged appends to its comments rather than opening a
//! second file on it. `core::db::flag_page` is the find-or-create; the unique
//! constraint on `(book, path)` is what keeps two reports racing each other from
//! making two.
//!
//! The id is an integer the database counts up. It travels in a URL, and nothing
//! restricts those URLs anyway: the application has no accounts.
//!
//! Real columns: the id, the `(book, path)` the file is found by, the `status`
//! the list filters on, and when the file was opened and last changed —
//! `updated_at` moves with every comment and every change of state, and is what
//! the list is sorted by. In the blob:
//!
//! ```text
//! page_title      what the page's tab said at the time, for the list
//! comments        [{text, created_at}], oldest first, dated as ISO 8601 text
//! legacy_id       the uuid a file opened before migration 001 was known by
//! ```

use std::fmt;

use crate::core::models::hybrid_document::{Columns, Field, HybridDocument, RowDict};

pub const OPEN: &str = "open";
pub const RESOLVED: &str = "resolved";
pub const STATUSES: [&str; 2] = [OPEN, RESOLVED];

/// One page, flagged for inspection, with everything said about it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PageInspection {
    columns: Columns,
    data: RowDict,
}

impl PageInspection {
    /// The id the database counted up.
    pub fn id(&self) -> Option<i64> {
        self.columns.int("id")
    }

    /// The page as the browser had it, `"/book/22"`.
    pub fn path(&self) -> Option<String> {
        self.columns.text("path")
    }

    /// The columns, for the writer that binds them.
    pub fn columns(&self) -> &Columns {
        &self.columns
    }
}

impl HybridDocument for PageInspection {
    const TABLE: &'static str = "page_inspections";

    fn indexed_columns() -> &'static [&'static str] {
        &["id", "book", "path", "status", "created_at", "updated_at"]
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

impl fmt::Display for PageInspection {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let comments = self
            .data
            .get("comments")
            .and_then(Field::as_list)
            .map_or(0, <[Field]>::len);
        write!(
            formatter,
            "{} {} ({}, {comments} comments)",
            self.columns.get("book"),
            self.columns.get("path"),
            self.columns.get("status"),
        )
    }
}

/// The `page_inspections` table, as SeaORM reads and writes it.
pub mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "page_inspections")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub book: Option<String>,
        pub path: Option<String>,
        pub status: Option<String>,
        pub created_at: Option<String>,
        pub updated_at: Option<String>,
        #[sea_orm(column_type = "Json", nullable)]
        pub data: Option<Json>,
    }

    #[derive(Copy, Clone, Debug, EnumIter)]
    pub enum Relation {}

    impl RelationTrait for Relation {
        fn def(&self) -> RelationDef {
            unreachable!("a flagged page has no relationship")
        }
    }

    impl ActiveModelBehavior for ActiveModel {}
}
