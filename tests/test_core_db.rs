//! Tests for the reads and writes of the reading history in `core::db`.
//!
//! The play-throughs are in `test_core_games.rs` and the flagged pages in
//! `test_inspection.rs`; the book is read off disk and belongs to
//! `test_story.rs`.

#[path = "common/mod.rs"]
mod common;

use chrono::Duration;
use common::{at, fake_db, BOOK};
use haute_tension::core::db::{get_oldest_page, last_pages, record_page_view, MAX_PAGE_HISTORY};
use haute_tension::core::models::hybrid_document::{Field, HybridDocument};
use haute_tension::core::models::page_view::PageView;
use haute_tension::row;

const OTHER_BOOK: &str = "pretre_jean/autre_livre";

/// Recording page views and reading the bounded history back.
mod page_history {
    use super::*;

    #[tokio::test]
    async fn a_view_is_recorded() {
        let db = fake_db().await;

        record_page_view(&db.db(), BOOK, "1", None)
            .await
            .expect("the database answers");

        let stored = db.table("page_views").one().await;
        assert_eq!(stored["book"], Field::Str(BOOK.to_string()));
        assert_eq!(stored["page"], Field::Str("1".to_string()));
    }

    #[tokio::test]
    async fn history_is_oldest_first() {
        let db = fake_db().await;
        for number in ["1", "2", "3"] {
            record_page_view(&db.db(), BOOK, number, None)
                .await
                .expect("the database answers");
        }

        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            ["1", "2", "3"]
        );
    }

    #[tokio::test]
    async fn history_keeps_only_the_most_recent_pages() {
        let db = fake_db().await;
        for number in 1..12 {
            record_page_view(&db.db(), BOOK, &number.to_string(), None)
                .await
                .expect("the database answers");
        }

        let expected: Vec<String> = (2..12).map(|number| number.to_string()).collect();
        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            expected
        );
    }

    #[tokio::test]
    async fn history_limit_is_adjustable() {
        let db = fake_db().await;
        for number in ["1", "2", "3"] {
            record_page_view(&db.db(), BOOK, number, None)
                .await
                .expect("the database answers");
        }

        assert_eq!(
            last_pages(&db.db(), BOOK, 2)
                .await
                .expect("the database answers"),
            ["2", "3"]
        );
    }

    #[tokio::test]
    async fn history_is_scoped_to_its_book() {
        let db = fake_db().await;
        record_page_view(&db.db(), BOOK, "1", None)
            .await
            .expect("the database answers");
        record_page_view(&db.db(), OTHER_BOOK, "9", None)
            .await
            .expect("the database answers");

        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            ["1"]
        );
    }

    /// Asking again for the page one is on adds nothing.
    #[tokio::test]
    async fn a_reload_is_not_a_move() {
        let db = fake_db().await;

        assert!(record_page_view(&db.db(), BOOK, "1", None)
            .await
            .expect("the database answers"));
        assert!(!record_page_view(&db.db(), BOOK, "1", None)
            .await
            .expect("the database answers"));

        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            ["1"]
        );
    }

    /// Only consecutive repeats are dropped: a loop is worth seeing.
    #[tokio::test]
    async fn coming_back_to_a_page_is_a_move() {
        let db = fake_db().await;
        for number in ["1", "2", "1"] {
            record_page_view(&db.db(), BOOK, number, None)
                .await
                .expect("the database answers");
        }

        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            ["1", "2", "1"]
        );
    }

    #[tokio::test]
    async fn a_reload_of_another_book_is_still_a_move() {
        let db = fake_db().await;
        record_page_view(&db.db(), BOOK, "1", None)
            .await
            .expect("the database answers");

        assert!(record_page_view(&db.db(), OTHER_BOOK, "1", None)
            .await
            .expect("the database answers"));
    }

    #[tokio::test]
    async fn views_sharing_a_timestamp_keep_insertion_order() {
        let db = fake_db().await;
        let stamp = at(2026, 9, 9, 12, 0);
        db.table("page_views")
            .seed(
                ["1", "2", "3"]
                    .iter()
                    .map(|number| row! { "book" => BOOK, "page" => *number, "viewed_at" => stamp })
                    .collect(),
            )
            .await;

        assert_eq!(
            last_pages(&db.db(), BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the database answers"),
            ["1", "2", "3"]
        );
    }

    #[tokio::test]
    async fn oldest_page_is_the_first_retained_entry() {
        let db = fake_db().await;
        let base = at(2026, 9, 9, 12, 0);
        db.table("page_views")
            .seed(vec![
                row! { "book" => BOOK, "page" => "4", "viewed_at" => base },
                row! { "book" => BOOK, "page" => "7", "viewed_at" => base + Duration::minutes(1) },
            ])
            .await;

        assert_eq!(
            get_oldest_page(&db.db(), BOOK)
                .await
                .expect("the database answers"),
            Some("4".to_string())
        );
    }

    #[tokio::test]
    async fn no_history_has_no_oldest_page() {
        let db = fake_db().await;

        assert_eq!(
            get_oldest_page(&db.db(), BOOK)
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// What a stored visit prints as, for a message about a run.
mod the_stored_visit {
    use super::*;

    #[test]
    fn it_names_what_was_read() {
        let view = PageView::from_dict(&row! {
            "book" => BOOK,
            "page" => "22",
            "viewed_at" => "now",
        });

        assert_eq!(view.to_string(), format!("{BOOK} p.22 @ now"));
    }
}
