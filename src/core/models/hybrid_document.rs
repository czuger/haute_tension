//! What every table shares: a few real columns, and a JSON blob for the rest.
//!
//! The schema is hybrid on purpose. A field is a column of its own when a query
//! filters, sorts or joins on it, or when the database has a rule to keep about
//! it — the id, the book, a timestamp, the hero's characteristics that may never
//! go below zero. Everything else is bundled into one `data` column holding a
//! JSON object, so that a new field on a fight is a key in that object.
//!
//! A row's children — the lines of a game's bag — are rows of a table of their
//! own, but they travel inside the parent's dict, as a list of their dicts under
//! the relationship's name. The trait is the whole contract:
//!
//! - [`HybridDocument::to_dict`] is the row as one flat dict: its columns, its
//!   children, its blob.
//! - [`HybridDocument::from_dict`] is the reverse: keys naming a column become
//!   that column, keys naming a relationship become child rows, the others are
//!   packed into `data`.
//! - [`HybridDocument::update_from_dict`] does the same onto an existing row,
//!   which is how `core::db` writes a loaded and changed state back. A child
//!   dict carrying an `id` updates that child, one without is a new child, and a
//!   child no dict names is dropped — which `core::db` turns into a delete.
//! - [`HybridDocument::to_json`] is `to_dict()` as text, for anything that
//!   answers JSON.
//!
//! The blob is normalised as it is set, so an instant put into it is ISO 8601
//! text at once and not only after a round trip; only a real column keeps the
//! type.

use std::cmp::Ordering;

use chrono::{DateTime, NaiveDate, Utc};
use indexmap::IndexMap;
use serde_json::Value;

use crate::core::models::utc_datetime::{from_text, to_isoformat};

pub const DATA_COLUMN: &str = "data";

/// One value of a row's flat dict.
///
/// A column keeps the type it was given — an instant stays an instant, which is
/// what tells a stored timestamp apart from the ISO text a blob holds — so this
/// is JSON plus the two kinds of date the application puts in a column.
#[derive(Clone, Debug)]
pub enum Field {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Instant(DateTime<Utc>),
    Date(NaiveDate),
    List(Vec<Field>),
    Dict(RowDict),
}

/// One row as a flat dict, the shape `to_dict()` gives and `from_dict()` takes.
pub type RowDict = IndexMap<String, Field>;

/// What [`json_default`] refuses to write.
#[derive(Debug, thiserror::Error)]
#[error("{type_name} cannot be written as JSON")]
pub struct UnwritableValue {
    pub type_name: &'static str,
}

impl Field {
    /// The name of this kind of value, for a message about it.
    pub fn type_name(&self) -> &'static str {
        match self {
            Field::Null => "NoneType",
            Field::Bool(_) => "bool",
            Field::Int(_) => "int",
            Field::Float(_) => "float",
            Field::Str(_) => "str",
            Field::Instant(_) => "datetime",
            Field::Date(_) => "date",
            Field::List(_) => "list",
            Field::Dict(_) => "dict",
        }
    }

    /// This value as JSON, an instant becoming ISO 8601 text.
    pub fn to_json(&self) -> Value {
        match self {
            Field::Null => Value::Null,
            Field::Bool(value) => Value::Bool(*value),
            Field::Int(value) => Value::from(*value),
            Field::Float(value) => serde_json::Number::from_f64(*value)
                .map_or(Value::Null, Value::Number),
            Field::Str(value) => Value::String(value.clone()),
            Field::Instant(_) | Field::Date(_) => Value::String(
                json_default(self).expect("an instant is exactly what the hook writes"),
            ),
            Field::List(values) => Value::Array(values.iter().map(Field::to_json).collect()),
            Field::Dict(values) => Value::Object(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.to_json()))
                    .collect(),
            ),
        }
    }

    /// A JSON value as a field: plain values only, never an instant.
    pub fn from_json(value: &Value) -> Self {
        match value {
            Value::Null => Field::Null,
            Value::Bool(value) => Field::Bool(*value),
            Value::Number(number) => number
                .as_i64()
                .map(Field::Int)
                .unwrap_or_else(|| Field::Float(number.as_f64().unwrap_or_default())),
            Value::String(text) => Field::Str(text.clone()),
            Value::Array(values) => Field::List(values.iter().map(Field::from_json).collect()),
            Value::Object(values) => Field::Dict(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), Field::from_json(value)))
                    .collect(),
            ),
        }
    }

    /// The text of a string field, or nothing for anything else.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Field::Str(text) => Some(text),
            _ => None,
        }
    }

    /// The number of an integer field, or nothing for anything else.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Field::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The instant of a timestamp column, parsing one stored as text.
    pub fn as_instant(&self) -> Option<DateTime<Utc>> {
        match self {
            Field::Instant(instant) => Some(*instant),
            Field::Str(text) => from_text(text),
            _ => None,
        }
    }

    /// The entries of a list field, or nothing for anything else.
    pub fn as_list(&self) -> Option<&[Field]> {
        match self {
            Field::List(values) => Some(values),
            _ => None,
        }
    }

    /// The keys of a dict field, or nothing for anything else.
    pub fn as_dict(&self) -> Option<&RowDict> {
        match self {
            Field::Dict(values) => Some(values),
            _ => None,
        }
    }

    /// Whether this field holds nothing at all.
    pub fn is_null(&self) -> bool {
        matches!(self, Field::Null)
    }
}

impl PartialEq for Field {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Field::Null, Field::Null) => true,
            (Field::Bool(left), Field::Bool(right)) => left == right,
            (Field::Int(left), Field::Int(right)) => left == right,
            (Field::Float(left), Field::Float(right)) => left == right,
            (Field::Int(left), Field::Float(right)) | (Field::Float(right), Field::Int(left)) => {
                *left as f64 == *right
            }
            (Field::Str(left), Field::Str(right)) => left == right,
            (Field::Instant(left), Field::Instant(right)) => left == right,
            (Field::Date(left), Field::Date(right)) => left == right,
            (Field::List(left), Field::List(right)) => left == right,
            (Field::Dict(left), Field::Dict(right)) => left == right,
            _ => false,
        }
    }
}

impl PartialOrd for Field {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (Field::Int(left), Field::Int(right)) => Some(left.cmp(right)),
            (Field::Float(left), Field::Float(right)) => left.partial_cmp(right),
            (Field::Str(left), Field::Str(right)) => Some(left.cmp(right)),
            (Field::Instant(left), Field::Instant(right)) => Some(left.cmp(right)),
            (Field::Date(left), Field::Date(right)) => Some(left.cmp(right)),
            _ => None,
        }
    }
}

impl From<i64> for Field {
    fn from(value: i64) -> Self {
        Field::Int(value)
    }
}

impl From<&str> for Field {
    fn from(value: &str) -> Self {
        Field::Str(value.to_string())
    }
}

impl From<String> for Field {
    fn from(value: String) -> Self {
        Field::Str(value)
    }
}

impl From<bool> for Field {
    fn from(value: bool) -> Self {
        Field::Bool(value)
    }
}

impl From<DateTime<Utc>> for Field {
    fn from(value: DateTime<Utc>) -> Self {
        Field::Instant(value)
    }
}

impl<T: Into<Field>> From<Option<T>> for Field {
    fn from(value: Option<T>) -> Self {
        value.map_or(Field::Null, Into::into)
    }
}

/// Write what JSON does not know how to: an instant, as ISO 8601 text.
///
/// # Returns
///
/// The text to write in its place.
///
/// # Errors
///
/// For anything that is not a date or an instant, which JSON writes itself.
pub fn json_default(value: &Field) -> Result<String, UnwritableValue> {
    match value {
        Field::Instant(instant) => Ok(to_isoformat(*instant)),
        Field::Date(day) => Ok(day.to_string()),
        other => Err(UnwritableValue {
            type_name: other.type_name(),
        }),
    }
}

/// The blob as it reads back: plain JSON values, instants as text, a copy.
pub fn normalised(values: &RowDict) -> RowDict {
    values
        .iter()
        .map(|(key, value)| (key.clone(), Field::from_json(&value.to_json())))
        .collect()
}

/// A row made of indexed columns, child rows, and one JSON object for the rest.
pub trait HybridDocument: Sized {
    /// The table this row belongs to.
    const TABLE: &'static str;

    /// The names of the real columns, `data` excepted.
    fn indexed_columns() -> &'static [&'static str];

    /// The one-to-many relationships whose rows travel inside this row's dict,
    /// each with the table its rows are.
    fn child_tables() -> &'static [(&'static str, &'static str)] {
        &[]
    }

    /// Build an empty row, before any value is written onto it.
    fn empty() -> Self;

    /// Read one column, as the flat dict shows it.
    fn column(&self, name: &str) -> Field;

    /// Write one column from the flat dict.
    fn set_column(&mut self, name: &str, value: &Field);

    /// The blob, everything the columns and the children do not hold.
    fn data(&self) -> &RowDict;

    /// Replace the blob whole.
    fn set_data(&mut self, data: RowDict);

    /// One relationship's children, as their own flat dicts.
    fn children(&self, _relationship: &str) -> Vec<RowDict> {
        Vec::new()
    }

    /// Replace one relationship's children, reusing the rows an `id` names.
    fn set_children(&mut self, _relationship: &str, _entries: &[RowDict]) {}

    /// Sort a flat dict into columns, children and the blob.
    ///
    /// # Returns
    ///
    /// The column values, the child dicts by relationship, then the rest.
    fn split(values: &RowDict) -> (RowDict, IndexMap<String, Vec<RowDict>>, RowDict) {
        let columns = Self::indexed_columns();
        let children: Vec<&str> = Self::child_tables()
            .iter()
            .map(|(name, _)| *name)
            .collect();
        let mut indexed = RowDict::new();
        let mut nested: IndexMap<String, Vec<RowDict>> = IndexMap::new();
        let mut rest = RowDict::new();
        for (key, value) in values {
            if columns.contains(&key.as_str()) {
                indexed.insert(key.clone(), value.clone());
            } else if children.contains(&key.as_str()) {
                nested.insert(
                    key.clone(),
                    value
                        .as_list()
                        .unwrap_or(&[])
                        .iter()
                        .map(|entry| entry.as_dict().cloned().unwrap_or_default())
                        .collect(),
                );
            } else {
                rest.insert(key.clone(), value.clone());
            }
        }
        (indexed, nested, rest)
    }

    /// Build a row, and its children, from one flat dict.
    fn from_dict(values: &RowDict) -> Self {
        let mut row = Self::empty();
        row.update_from_dict(values);
        row
    }

    /// Replace the whole row with one flat dict, children and blob included.
    ///
    /// A column or a relationship left out keeps what it has; the blob is
    /// replaced entirely.
    fn update_from_dict(&mut self, values: &RowDict) {
        let (indexed, nested, rest) = Self::split(values);
        for (key, value) in &indexed {
            self.set_column(key, value);
        }
        for (key, entries) in &nested {
            self.set_children(key, entries);
        }
        self.set_data(normalised(&rest));
    }

    /// The row as one flat dict: its columns, its children, then the blob's keys.
    fn to_dict(&self) -> RowDict {
        let mut values = RowDict::new();
        for name in Self::indexed_columns() {
            values.insert((*name).to_string(), self.column(name));
        }
        for (relationship, _) in Self::child_tables() {
            values.insert(
                (*relationship).to_string(),
                Field::List(
                    self.children(relationship)
                        .into_iter()
                        .map(Field::Dict)
                        .collect(),
                ),
            );
        }
        for (key, value) in self.data() {
            values.insert(key.clone(), value.clone());
        }
        values
    }

    /// The row as JSON text, children included, instants written as ISO 8601.
    fn to_json(&self) -> String {
        serde_json::to_string(&Field::Dict(self.to_dict()).to_json())
            .expect("a normalised dict is always writable")
    }
}

/// Build a flat dict from pairs, for the places that spell one out.
#[macro_export]
macro_rules! row {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut dict = $crate::core::models::hybrid_document::RowDict::new();
        $(dict.insert($key.to_string(), $crate::core::models::hybrid_document::Field::from($value));)*
        dict
    }};
}

impl std::fmt::Display for Field {
    /// A field as a message about a run prints it, the way `str()` did.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Field::Null => formatter.write_str("None"),
            Field::Bool(true) => formatter.write_str("True"),
            Field::Bool(false) => formatter.write_str("False"),
            Field::Int(value) => write!(formatter, "{value}"),
            Field::Float(value) => write!(formatter, "{value}"),
            Field::Str(text) => formatter.write_str(text),
            Field::Instant(instant) => {
                write!(formatter, "{}", instant.format("%Y-%m-%d %H:%M:%S%.f+00:00"))
            }
            Field::Date(day) => write!(formatter, "{day}"),
            Field::List(_) | Field::Dict(_) => {
                write!(formatter, "{}", Field::to_json(self))
            }
        }
    }
}

/// The columns of one row, and the blob beside them.
///
/// Every model is this plus its children: the trait's `column` and `set_column`
/// are a lookup in here, and the typed readers below are what `core::db` uses
/// once it knows which column it is asking for.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Columns(RowDict);

impl Columns {
    /// Read one column, absent reading as nothing.
    pub fn get(&self, name: &str) -> Field {
        self.0.get(name).cloned().unwrap_or(Field::Null)
    }

    /// Write one column.
    pub fn set(&mut self, name: &str, value: &Field) {
        self.0.insert(name.to_string(), value.clone());
    }

    /// One column as a whole number.
    pub fn int(&self, name: &str) -> Option<i64> {
        self.0.get(name).and_then(Field::as_int)
    }

    /// One column as text.
    pub fn text(&self, name: &str) -> Option<String> {
        self.0.get(name).and_then(|value| match value {
            Field::Str(text) => Some(text.clone()),
            Field::Null => None,
            other => Some(other.to_string()),
        })
    }

    /// One column as an instant.
    pub fn instant(&self, name: &str) -> Option<DateTime<Utc>> {
        self.0.get(name).and_then(Field::as_instant)
    }
}
