//! The book itself, read off disk.
//!
//! The book is static — 668 pages that only change when the import pipeline is
//! re-run — so it is read once at startup and held in memory rather than stored.
//! Nothing here touches the database: `core::db` keeps the one thing that does
//! change, which is what has been read.

use std::path::Path;

use indexmap::IndexMap;
use serde_json::Value;

use crate::application::models::story_page::{StoryData, StoryPage};

/// The file a book directory is read from. The pipeline writes several JSON
/// files there over a book's life; this is the one the application serves.
pub const PAGES_FILE: &str = "pages.json";

/// Why a book could not be read.
#[derive(Debug, thiserror::Error)]
pub enum StoryError {
    #[error("the book has no {PAGES_FILE}: {0}")]
    Missing(#[source] std::io::Error),
    #[error("the book is not valid JSON: {0}")]
    Malformed(#[source] serde_json::Error),
    #[error("{0}")]
    Invalid(String),
}

/// Load and index all numbered pages from a book directory.
///
/// # Arguments
///
/// * `book_path` - Directory containing the book's `pages.json` file.
///
/// # Returns
///
/// Story pages indexed by their page number.
///
/// # Errors
///
/// If the pages file does not exist, holds invalid JSON, is malformed, or
/// repeats a page.
pub fn load_story(book_path: &Path) -> Result<StoryData, StoryError> {
    let pages_path = book_path.join(PAGES_FILE);
    let serialized_pages = std::fs::read_to_string(&pages_path).map_err(StoryError::Missing)?;
    let pages: Value = serde_json::from_str(&serialized_pages).map_err(StoryError::Malformed)?;
    let Value::Array(pages) = pages else {
        return Err(StoryError::Invalid("Book pages must be a list.".to_string()));
    };

    let mut story_data: StoryData = IndexMap::new();
    for page in pages {
        let Value::Object(page) = page else {
            return Err(StoryError::Invalid(
                "Each book page must be an object.".to_string(),
            ));
        };
        let Some(Value::String(page_number)) = page.get("page") else {
            return Err(StoryError::Invalid(
                "Each book page must have a string page number.".to_string(),
            ));
        };
        let page_number = page_number.clone();
        if story_data.contains_key(&page_number) {
            return Err(StoryError::Invalid(format!(
                "Duplicate page number: {page_number}"
            )));
        }
        story_data.insert(page_number, StoryPage::new(page));
    }
    Ok(story_data)
}
