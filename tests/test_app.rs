//! Tests for the app factory and the development-server entry point.

#[path = "common/mod.rs"]
mod common;

use common::{books_path, client_for, fake_db, BOOK};
use haute_tension::application::factory::{books_path as packaged, create_app, AppConfig};
use haute_tension::core::db::Db;
use haute_tension::core::story::{StoryError, PAGES_FILE};

/// Wiring the app onto a book read off disk.
mod creating_the_app {
    use super::*;

    /// The routes are counted by address: one of each family answers, rather than
    /// falling through.
    #[tokio::test]
    async fn every_family_of_routes_is_mounted() {
        let db = fake_db().await;
        let client = client_for(db.db(), books_path(), None);

        for (family, path) in [
            ("web", "/"),
            ("api", "/data/1"),
            ("game", "/game/new"),
            ("inspection", "/flagged-pages"),
        ] {
            let response = client.get(path).await;

            assert_eq!(response.status, 200, "the {family} routes answer {path}");
        }
    }

    #[test]
    fn the_session_cookie_is_signed() {
        assert!(!AppConfig::default().secret.is_empty());
    }

    /// The book is on disk, so building the app reaches for no database.
    #[test]
    fn the_app_is_built_without_a_database() {
        let books = books_path();

        assert!(create_app(AppConfig {
            db: Db::Unavailable("none".to_string()),
            ..AppConfig::for_book(BOOK, books.path())
        })
        .is_ok());
    }

    /// Nothing is served half-built: the reader is told instead.
    #[tokio::test]
    async fn a_page_needing_the_database_refuses_without_one() {
        let client = common::no_database_client(books_path());

        assert_eq!(client.get("/book/1").await.status, 503);
    }

    /// It shows the book and a link, and reads nothing.
    #[tokio::test]
    async fn the_landing_page_needs_no_database() {
        let client = common::no_database_client(books_path());

        assert_eq!(client.get("/").await.status, 200);
    }

    #[tokio::test]
    async fn the_book_is_read_once_at_startup() {
        let db = fake_db().await;
        let books = books_path();
        let pages = books.path().join(BOOK).join(PAGES_FILE);
        let client = client_for(db.db(), books, None);
        std::fs::write(&pages, "[]").expect("the book file is written");

        assert_eq!(client.get("/book/1").await.status, 200);
    }

    #[test]
    fn a_missing_book_is_reported() {
        let empty = tempfile::tempdir().expect("a temporary directory is always made");

        assert!(matches!(
            create_app(AppConfig::for_book(BOOK, empty.path())),
            Err(StoryError::Missing(_))
        ));
    }

    #[tokio::test]
    async fn the_packaged_book_is_the_default() {
        let db = fake_db().await;
        let router = create_app(AppConfig {
            db: db.db(),
            secret: common::TEST_SECRET.to_vec(),
            ..AppConfig::default()
        })
        .expect("the packaged book is on disk");

        let response = tower::ServiceExt::oneshot(
            router,
            axum::http::Request::builder()
                .uri("/book/1")
                .body(axum::body::Body::empty())
                .expect("the request is built"),
        )
        .await
        .expect("the router answers");

        assert_eq!(response.status(), 200);
        assert!(packaged().join(BOOK).join(PAGES_FILE).exists());
    }
}
