//! Tests for reading and indexing a book's pages.

use haute_tension::core::story::{load_story, StoryError, PAGES_FILE};
use serde_json::Value;

/// An isolated book directory for one test.
struct Book(tempfile::TempDir);

impl Book {
    /// Open a book directory of this test's own.
    fn new() -> Self {
        Self(tempfile::tempdir().expect("a temporary directory is always made"))
    }

    /// The directory the book is read from.
    fn path(&self) -> &std::path::Path {
        self.0.path()
    }

    /// Persist merged-pages content for the book under test.
    fn write_pages(&self, pages: Value) {
        std::fs::write(
            self.path().join(PAGES_FILE),
            serde_json::to_string(&pages).expect("the pages are always written"),
        )
        .expect("the pages file is always written");
    }

    /// Persist text that is not valid JSON.
    fn write_text(&self, text: &str) {
        std::fs::write(self.path().join(PAGES_FILE), text)
            .expect("the pages file is always written");
    }
}

/// Return every page keyed by its own page number.
#[test]
fn pages_are_indexed_by_page_number() {
    let book = Book::new();
    book.write_pages(serde_json::json!([
        {"page": "1", "text": ["Un."]},
        {"page": "2", "text": ["Deux."]},
    ]));

    let story_data = load_story(book.path()).expect("the book reads");

    let mut numbers: Vec<&String> = story_data.keys().collect();
    numbers.sort();
    assert_eq!(numbers, ["1", "2"]);
    assert_eq!(story_data["2"].text(), ["Deux."]);
}

/// Report when the merged-pages file does not exist.
#[test]
fn missing_book_file_is_reported() {
    let book = Book::new();

    assert!(matches!(
        load_story(book.path()),
        Err(StoryError::Missing(_))
    ));
}

/// Report when the merged-pages file is not valid JSON.
#[test]
fn invalid_json_is_reported() {
    let book = Book::new();
    book.write_text("{");

    assert!(matches!(
        load_story(book.path()),
        Err(StoryError::Malformed(_))
    ));
}

/// Report when the merged-pages document is not a list.
#[test]
fn non_list_book_is_rejected() {
    let book = Book::new();
    book.write_pages(serde_json::json!({"page": "1"}));

    assert!(matches!(
        load_story(book.path()),
        Err(StoryError::Invalid(_))
    ));
}

/// Report when an entry in the merged-pages list is not an object.
#[test]
fn non_object_page_is_rejected() {
    let book = Book::new();
    book.write_pages(serde_json::json!(["1"]));

    assert!(matches!(
        load_story(book.path()),
        Err(StoryError::Invalid(_))
    ));
}

/// Report when a page number is missing or not a string.
#[test]
fn page_without_string_number_is_rejected() {
    let book = Book::new();
    book.write_pages(serde_json::json!([{"page": 1}]));

    assert!(matches!(
        load_story(book.path()),
        Err(StoryError::Invalid(_))
    ));
}

/// Report when two pages share the same page number.
#[test]
fn duplicate_page_number_is_rejected() {
    let book = Book::new();
    book.write_pages(serde_json::json!([{"page": "1"}, {"page": "1"}]));

    let trouble = load_story(book.path()).expect_err("a repeated page is refused");

    assert!(trouble.to_string().contains("Duplicate page number: 1"));
}

/// Return an empty index for a book without any page.
#[test]
fn empty_book_loads_no_pages() {
    let book = Book::new();
    book.write_pages(serde_json::json!([]));

    assert!(load_story(book.path()).expect("the book reads").is_empty());
}
