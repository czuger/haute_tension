//! The shape of one page of the book, as the routes and templates read it.
//!
//! The book is read off disk and never written, so a page is the JSON object the
//! import pipeline wrote plus the readers the templates need. The optional parts
//! — a fight, the gains and losses on a choice — are asked for rather than
//! declared, because the pipeline leaves them out of the pages that have none.

use indexmap::IndexMap;
use serde_json::{Map, Value};

/// Every page of one book, indexed by its page number.
pub type StoryData = IndexMap<String, StoryPage>;

/// A numbered story page with its choices and, sometimes, a fight.
#[derive(Clone, Debug, PartialEq)]
pub struct StoryPage(Map<String, Value>);

impl StoryPage {
    /// Wrap the object the pipeline wrote.
    pub fn new(page: Map<String, Value>) -> Self {
        Self(page)
    }

    /// The page as the pipeline wrote it, for `/data/<number>`.
    pub fn as_object(&self) -> &Map<String, Value> {
        &self.0
    }

    /// The page number, as text.
    pub fn page(&self) -> &str {
        self.0.get("page").and_then(Value::as_str).unwrap_or("")
    }

    /// The narrative, one entry per paragraph.
    pub fn text(&self) -> Vec<&str> {
        self.0
            .get("text")
            .and_then(Value::as_array)
            .map(|lines| lines.iter().filter_map(Value::as_str).collect())
            .unwrap_or_default()
    }

    /// Every choice the page offers, in the order it prints them.
    pub fn choices(&self) -> Vec<&Map<String, Value>> {
        self.0
            .get("choices")
            .and_then(Value::as_array)
            .map(|choices| choices.iter().filter_map(Value::as_object).collect())
            .unwrap_or_default()
    }

    /// The page's fight, when the pipeline found one.
    pub fn fight(&self) -> Option<&Map<String, Value>> {
        self.0.get("fight").and_then(Value::as_object)
    }

    /// Whether the page holds something that reads as a fight at all.
    ///
    /// Truthy rather than usable: the parser does emit fights whose adversaries
    /// cannot be fielded, and arming one is what finds that out.
    pub fn has_fight(&self) -> bool {
        match self.0.get("fight") {
            None | Some(Value::Null) => false,
            Some(Value::Object(fight)) => !fight.is_empty(),
            Some(Value::Array(entries)) => !entries.is_empty(),
            Some(Value::Bool(carried)) => *carried,
            Some(Value::String(text)) => !text.is_empty(),
            Some(_) => true,
        }
    }

    /// Whether the page's fight names at least one adversary.
    pub fn fight_has_enemies(&self) -> bool {
        self.fight()
            .and_then(|fight| fight.get("enemies"))
            .and_then(Value::as_array)
            .is_some_and(|enemies| !enemies.is_empty())
    }
}
