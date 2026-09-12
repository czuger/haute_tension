//! Tests for the landing page, book navigation and the reading trail.

#[path = "common/mod.rs"]
mod common;

use chrono::Duration;
use common::{at, books_path, client, dead_database_client, TestClient, BOOK};
use haute_tension::core::models::hybrid_document::Field;
use haute_tension::row;
use regex::Regex;

/// The page numbers the breadcrumb shows, in order, for one reader page.
async fn trail_of(client: &TestClient, number: &str) -> Option<Vec<String>> {
    let body = client.get(&format!("/book/{number}")).await.text();
    let breadcrumb = Regex::new(r#"(?s)<nav class="trail".*?</nav>"#)
        .expect("the pattern is written here")
        .find(&body)?
        .as_str()
        .to_string();
    Some(
        Regex::new(r">(\d+)<")
            .expect("the pattern is written here")
            .captures_iter(&breadcrumb)
            .map(|found| found[1].to_string())
            .collect(),
    )
}

/// The unescaped page, the way a reader sees it.
fn unescaped(body: &str) -> String {
    body.replace("&#39;", "'")
        .replace("&#34;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

/// A timestamp that orders page views the way they were seeded.
fn stamp(minute: i64) -> chrono::DateTime<chrono::Utc> {
    at(2026, 9, 9, 12, 0) + Duration::minutes(minute)
}

/// The one book on offer.
mod the_landing_page {
    use super::*;

    #[tokio::test]
    async fn the_current_book_is_shown() {
        let (_db, client) = client().await;

        let response = client.get("/").await;

        assert_eq!(response.status, 200);
        assert!(unescaped(&response.text()).contains("La Forteresse d'Alamuth"));
    }

    #[tokio::test]
    async fn it_links_to_the_opening_page() {
        let (_db, client) = client().await;

        assert!(client.get("/").await.text().contains(r#"href="/book/1""#));
    }

    #[tokio::test]
    async fn it_shows_no_trail() {
        let (_db, client) = client().await;
        client.get("/book/1").await;

        assert!(!client.get("/").await.text().contains(r#"class="trail""#));
    }
}

/// Reading a page and moving on from it.
mod the_reader {
    use super::*;

    #[tokio::test]
    async fn choices_come_before_the_story_text() {
        let (_db, client) = client().await;

        let body = client.get("/book/1").await.text();

        let choice = body.find(r#"href="/book/2""#).expect("the choice is shown");
        let story = body
            .find("Le début de l&#39;aventure.")
            .expect("the story is shown");
        assert!(choice < story);
    }

    #[tokio::test]
    async fn the_story_text_is_rendered() {
        let (_db, client) = client().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains("Le début de l&#39;aventure."));
        assert!(body.contains("Deuxième ligne."));
    }

    #[tokio::test]
    async fn a_terminal_page_offers_to_restart() {
        let (_db, client) = client().await;

        let body = client.get("/book/2").await.text();

        assert!(body.contains("Fin de l'aventure"));
        assert!(body.contains(r#"href="/book/1""#));
    }

    #[tokio::test]
    async fn an_unknown_page_returns_not_found() {
        let (_db, client) = client().await;

        assert_eq!(client.get("/book/999").await.status, 404);
    }
}

/// The breadcrumb of the last pages read.
mod the_reading_trail {
    use super::*;

    #[tokio::test]
    async fn reading_a_page_records_it() {
        let (db, client) = client().await;
        client.get("/book/1").await;

        let pages: Vec<String> = db
            .table("page_views")
            .docs()
            .await
            .iter()
            .map(|view| view["page"].to_string())
            .collect();
        assert_eq!(pages, ["1"]);
    }

    #[tokio::test]
    async fn the_trail_ends_on_the_page_being_read() {
        let (_db, client) = client().await;

        assert_eq!(trail_of(&client, "1").await, Some(vec!["1".to_string()]));
    }

    #[tokio::test]
    async fn the_trail_follows_the_reader() {
        let (_db, client) = client().await;
        client.get("/book/1").await;

        assert_eq!(
            trail_of(&client, "2").await,
            Some(vec!["1".to_string(), "2".to_string()])
        );
    }

    #[tokio::test]
    async fn the_current_page_is_marked_and_not_a_link() {
        let (_db, client) = client().await;
        client.get("/book/1").await;
        let body = client.get("/book/2").await.text();
        let breadcrumb = Regex::new(r#"(?s)<nav class="trail".*?</nav>"#)
            .expect("the pattern is written here")
            .find(&body)
            .expect("the trail is shown")
            .as_str()
            .to_string();

        let tightened = Regex::new(r"\s+")
            .expect("the pattern is written here")
            .replace_all(&breadcrumb, "");
        assert!(tightened.contains(r#"aria-current="page">2<"#));
        assert_eq!(breadcrumb.matches(r#"href="/book/1""#).count(), 1);
    }

    #[tokio::test]
    async fn earlier_pages_are_links_back() {
        let (_db, client) = client().await;
        client.get("/book/1").await;

        assert!(client
            .get("/book/2")
            .await
            .text()
            .contains(r#"href="/book/1""#));
    }

    #[tokio::test]
    async fn a_reload_does_not_lengthen_the_trail() {
        let (_db, client) = client().await;
        client.get("/book/1").await;

        assert_eq!(trail_of(&client, "1").await, Some(vec!["1".to_string()]));
        assert_eq!(trail_of(&client, "1").await, Some(vec!["1".to_string()]));
    }

    #[tokio::test]
    async fn a_loop_is_shown() {
        let (_db, client) = client().await;
        client.get("/book/1").await;
        client.get("/book/2").await;

        assert_eq!(
            trail_of(&client, "1").await,
            Some(vec!["1".to_string(), "2".to_string(), "1".to_string()])
        );
    }

    /// Twelve pages read, plus this visit: the ten most recent are shown.
    #[tokio::test]
    async fn the_trail_keeps_only_ten_pages() {
        let (db, client) = client().await;
        db.table("page_views")
            .seed(
                (1..13)
                    .map(|number| {
                        row! {
                            "book" => BOOK,
                            "page" => number.to_string(),
                            "viewed_at" => stamp(number),
                        }
                    })
                    .collect(),
            )
            .await;

        let trail = trail_of(&client, "1").await.expect("the trail is shown");

        assert_eq!(trail.len(), 10);
        assert_eq!(
            trail,
            ["4", "5", "6", "7", "8", "9", "10", "11", "12", "1"]
        );
    }

    #[tokio::test]
    async fn another_book_never_shows_up() {
        let (db, client) = client().await;
        db.table("page_views")
            .seed(vec![row! {
                "book" => "autre/livre",
                "page" => "99",
                "viewed_at" => stamp(1),
            }])
            .await;

        assert_eq!(trail_of(&client, "1").await, Some(vec!["1".to_string()]));
    }

    /// Half a page is worse than none: the reader is told instead.
    #[tokio::test]
    async fn an_unreachable_database_refuses_the_page() {
        let client = dead_database_client(books_path());

        let response = client.get("/book/1").await;

        assert_eq!(response.status, 503);
        assert!(response.text().contains("ne répond pas"));
        assert!(!response.text().contains("Le début de l&#39;aventure."));
    }
}

/// The field the trail is read off, for the one page that has none.
mod the_trail_on_a_page_without_one {
    use super::*;

    #[tokio::test]
    async fn the_landing_page_has_no_breadcrumb_at_all() {
        let (_db, client) = client().await;

        let body = client.get("/").await.text();

        assert!(Regex::new(r#"(?s)<nav class="trail".*?</nav>"#)
            .expect("the pattern is written here")
            .find(&body)
            .is_none());
        assert_eq!(Field::Str(BOOK.to_string()).to_string(), BOOK);
    }
}
