//! What a reader is shown when the database cannot answer.
//!
//! Ported from `tests/test_errors.py`. Every test here takes the log lock: the
//! capture belongs to the process, so a test reading it must be the only one
//! writing into it.

// The environment and the log belong to the process, so the lock that
// serialises the tests reading them is held for the length of each one.
#![allow(clippy::await_holding_lock)]

#[path = "common/mod.rs"]
mod common;

use common::{
    books_path, books_path_with_fights, dead_database_client, log_lock, no_database_client,
    TestClient,
};
use haute_tension::application::models::game::SESSION_KEY;
use haute_tension::core::logs::general_log::{start_capture, take_capture, Record};

/// Every read and write fails the way a file that cannot be opened looks.
fn dead_database() -> TestClient {
    dead_database_client(books_path())
}

/// Nothing configured to reach in the first place.
fn no_database() -> TestClient {
    no_database_client(books_path())
}

/// A client with fights, whose database refuses everything.
fn dead_fighting_database() -> TestClient {
    dead_database_client(books_path_with_fights())
}

/// The whole log as one text, the way `caplog.text` reads.
fn text_of(records: &[Record]) -> String {
    records
        .iter()
        .map(Record::text)
        .collect::<Vec<String>>()
        .join("\n")
}

/// The unescaped page, the way a reader sees it.
fn unescaped(body: &str) -> String {
    body.replace("&#39;", "'").replace("&#34;", "\"")
}

/// A page that cannot be built is refused, not served half-built.
mod the_pages_that_need_it {
    use super::*;

    #[tokio::test]
    async fn the_reader_is_told() {
        let client = dead_database();

        let response = client.get("/book/1").await;

        assert_eq!(response.status, 503);
        assert!(response.text().contains("ne répond pas"));
    }

    /// Half a page, quietly missing its trail, is what this replaces.
    #[tokio::test]
    async fn the_story_text_is_not_served() {
        let client = dead_database();

        let body = client.get("/book/1").await.text();

        assert!(!body.contains("Le début de l&#39;aventure."));
        assert!(!body.contains(r#"class="trail""#));
    }

    #[tokio::test]
    async fn no_database_configured_reads_the_same() {
        let client = no_database();

        let response = client.get("/book/1").await;

        assert_eq!(response.status, 503);
        assert!(response.text().contains("ne répond pas"));
    }

    #[tokio::test]
    async fn the_page_points_at_the_configuration() {
        let client = no_database();

        assert!(client.get("/book/1").await.text().contains("DATABASE_DIR"));
    }

    #[tokio::test]
    async fn rolling_up_a_hero_is_refused() {
        let client = no_database_client(books_path_with_fights());

        assert_eq!(client.post("/game/new").await.status, 503);
    }

    /// The hero is rolled on a database that answers, and the session cookie
    /// carries him to one that does not: an assault needs the database, and a
    /// reader who has no hero at all never reaches for it.
    #[tokio::test]
    async fn an_assault_is_refused() {
        let (_db, living) = common::hero().await;
        let game_id = living
            .session()
            .get_id(SESSION_KEY)
            .expect("the hero was rolled up");
        let client = dead_fighting_database();
        client.set_session(SESSION_KEY, serde_json::Value::from(game_id));

        assert_eq!(client.post("/combat/1/assault").await.status, 503);
    }
}

/// `/data/<number>` is parsed, not read.
mod the_api_answers_json {
    use super::*;

    #[tokio::test]
    async fn the_refusal_is_json() {
        let client = dead_database();

        let response = client.get("/data/1").await;

        assert_eq!(response.status, 503);
        assert_eq!(response.mimetype(), "application/json");
    }

    #[tokio::test]
    async fn it_says_what_is_wrong() {
        let client = dead_database();

        assert_eq!(
            client.get("/data/1").await.json(),
            serde_json::json!({
                "success": false,
                "message": "la base de données est injoignable",
            })
        );
    }

    #[tokio::test]
    async fn it_is_not_an_html_page() {
        let client = dead_database();

        assert!(!client.get("/data/1").await.text().contains("<!doctype html>"));
    }
}

/// What needs no database still works when there is none.
mod the_pages_that_do_not {
    use super::*;

    #[tokio::test]
    async fn the_landing_page_is_served() {
        let client = dead_database();

        let response = client.get("/").await;

        assert_eq!(response.status, 200);
        assert!(unescaped(&response.text()).contains("La Forteresse d'Alamuth"));
    }

    /// The book is in memory, so a missing page is known without a query.
    #[tokio::test]
    async fn an_unknown_page_is_still_a_404() {
        let client = dead_database();

        assert_eq!(client.get("/book/999").await.status, 404);
    }
}

/// A refusal leaves the reason behind it.
mod the_log {
    use super::*;

    #[tokio::test]
    async fn the_failure_is_written_with_its_trouble() {
        let _lock = log_lock();
        let client = dead_database();
        start_capture();

        client.get("/book/1").await;

        let written = text_of(&take_capture());
        assert!(written.contains("The database could not answer"));
        assert!(written.contains("OperationalError"));
    }

    #[tokio::test]
    async fn the_line_names_the_request() {
        let _lock = log_lock();
        let client = dead_database();
        start_capture();

        client.get("/book/1").await;

        let written = text_of(&take_capture());
        assert!(written.contains("path='/book/1'"));
        assert!(written.contains("endpoint='web.read_page'"));
    }
}
