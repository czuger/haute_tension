//! Flagging a page for inspection, and reviewing the pages flagged.

#[path = "common/mod.rs"]
mod common;

use common::{
    at, books_path, client, client_for, fake_db, hero, no_database_client, TestClient, BOOK,
};
use haute_tension::core::db::{
    find_inspection, flag_page, flagged_pages, set_inspection_status, StatusError,
};
use haute_tension::core::models::hybrid_document::{Field, HybridDocument};
use haute_tension::core::models::page_inspection::PageInspection;
use haute_tension::row;
use regex::Regex;

const OTHER_BOOK: &str = "pretre_jean/autre_livre";

/// The header a script sends, and a browser does not.
const AS_SCRIPT: [(&str, &str); 1] = [("accept", "application/json")];

/// Send the dialog's form the way the browser would.
async fn report(client: &TestClient, path: &str, comment: &str) -> common::TestResponse {
    client
        .post_form(
            "/flag-page",
            &[("path", path), ("title", "Page 1 · Titre"), ("comment", comment)],
        )
        .await
}

/// Send the dialog's form the way the page's script would.
async fn report_as_script(
    client: &TestClient,
    path: &str,
    comment: &str,
) -> common::TestResponse {
    client
        .post_form_with(
            "/flag-page",
            &[("path", path), ("title", "Page 1 · Titre"), ("comment", comment)],
            &AS_SCRIPT,
        )
        .await
}

/// One file per page, whatever the number of reports.
mod flagging_in_the_database {
    use super::*;

    #[tokio::test]
    async fn the_first_report_opens_a_file() {
        let db = fake_db().await;

        let filed = flag_page(
            &db.db(),
            BOOK,
            "/book/22",
            Some("Page 22"),
            "Le choix mène au 23.",
        )
        .await
        .expect("the database answers");

        assert_eq!(filed.path, "/book/22");
        assert_eq!(filed.status, "open");
        let texts: Vec<&str> = filed
            .comments
            .iter()
            .map(|comment| comment.text.as_str())
            .collect();
        assert_eq!(texts, ["Le choix mène au 23."]);
        assert_eq!(db.table("page_inspections").docs().await.len(), 1);
    }

    #[tokio::test]
    async fn a_second_report_adds_to_it() {
        let db = fake_db().await;
        flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Première remarque.")
            .await
            .expect("the database answers");

        let filed = flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Seconde remarque.")
            .await
            .expect("the database answers");

        let texts: Vec<&str> = filed
            .comments
            .iter()
            .map(|comment| comment.text.as_str())
            .collect();
        assert_eq!(texts, ["Première remarque.", "Seconde remarque."]);
        assert_eq!(db.table("page_inspections").docs().await.len(), 1);
    }

    #[tokio::test]
    async fn the_title_is_kept_and_not_blanked_by_a_later_report() {
        let db = fake_db().await;
        flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Une.")
            .await
            .expect("the database answers");

        let filed = flag_page(&db.db(), BOOK, "/book/22", None, "Deux.")
            .await
            .expect("the database answers");

        assert_eq!(filed.page_title.as_deref(), Some("Page 22"));
    }

    #[tokio::test]
    async fn a_resolved_page_flagged_again_is_reopened() {
        let db = fake_db().await;
        let filed = flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Une.")
            .await
            .expect("the database answers");
        set_inspection_status(&db.db(), filed.id, "resolved")
            .await
            .expect("the database answers");

        let again = flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Encore.")
            .await
            .expect("the database answers");

        assert_eq!(again.status, "open");
    }

    #[tokio::test]
    async fn each_comment_is_dated() {
        let db = fake_db().await;

        let filed = flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Une.")
            .await
            .expect("the database answers");

        assert!(filed.comments[0].created_at.starts_with("20"));
        assert_eq!(filed.created_at, filed.updated_at);
    }

    #[tokio::test]
    async fn the_row_says_what_it_is() {
        let db = fake_db().await;
        flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Une.")
            .await
            .expect("the database answers");

        let filed = db.table("page_inspections").one().await;

        assert_eq!(
            PageInspection::from_dict(&filed).to_string(),
            format!("{BOOK} /book/22 (open, 1 comments)")
        );
    }

    #[tokio::test]
    async fn pages_of_another_book_are_kept_apart() {
        let db = fake_db().await;
        flag_page(&db.db(), BOOK, "/book/22", Some("Page 22"), "Ici.")
            .await
            .expect("the database answers");
        flag_page(&db.db(), OTHER_BOOK, "/book/22", Some("Page 22"), "Ailleurs.")
            .await
            .expect("the database answers");

        assert_eq!(db.table("page_inspections").docs().await.len(), 2);
        let texts: Vec<String> = flagged_pages(&db.db(), BOOK, None)
            .await
            .expect("the database answers")
            .iter()
            .map(|filed| filed.comments[0].text.clone())
            .collect();
        assert_eq!(texts, ["Ici."]);
    }
}

/// What the list gives back.
mod the_list {
    use super::*;

    #[tokio::test]
    async fn the_most_recently_commented_comes_first() {
        let db = fake_db().await;
        let first = flag_page(&db.db(), BOOK, "/book/1", Some("Page 1"), "Une.")
            .await
            .expect("the database answers");
        let second = flag_page(&db.db(), BOOK, "/book/2", Some("Page 2"), "Deux.")
            .await
            .expect("the database answers");

        let touched: Vec<_> = db
            .table("page_inspections")
            .docs()
            .await
            .into_iter()
            .map(|mut doc| {
                let when = if doc["path"] == Field::Str("/book/1".to_string()) {
                    at(2026, 9, 10, 12, 0)
                } else {
                    at(2026, 9, 10, 11, 0)
                };
                doc.insert("updated_at".to_string(), Field::Instant(when));
                doc
            })
            .collect();
        db.table("page_inspections").seed(touched).await;

        let ids: Vec<i64> = flagged_pages(&db.db(), BOOK, None)
            .await
            .expect("the database answers")
            .iter()
            .map(|filed| filed.id)
            .collect();
        assert_eq!(ids, [first.id, second.id]);
    }

    #[tokio::test]
    async fn it_can_be_narrowed_to_one_status() {
        let db = fake_db().await;
        flag_page(&db.db(), BOOK, "/book/1", Some("Page 1"), "Une.")
            .await
            .expect("the database answers");
        let done = flag_page(&db.db(), BOOK, "/book/2", Some("Page 2"), "Deux.")
            .await
            .expect("the database answers");
        set_inspection_status(&db.db(), done.id, "resolved")
            .await
            .expect("the database answers");

        let open: Vec<String> = flagged_pages(&db.db(), BOOK, Some("open"))
            .await
            .expect("the database answers")
            .iter()
            .map(|filed| filed.path.clone())
            .collect();
        let resolved: Vec<String> = flagged_pages(&db.db(), BOOK, Some("resolved"))
            .await
            .expect("the database answers")
            .iter()
            .map(|filed| filed.path.clone())
            .collect();

        assert_eq!(open, ["/book/1"]);
        assert_eq!(resolved, ["/book/2"]);
        assert_eq!(
            flagged_pages(&db.db(), BOOK, None)
                .await
                .expect("the database answers")
                .len(),
            2
        );
    }

    #[tokio::test]
    async fn an_unknown_file_is_none() {
        let db = fake_db().await;

        assert_eq!(
            find_inspection(&db.db(), Some(999))
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            find_inspection(&db.db(), None)
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            set_inspection_status(&db.db(), 999, "resolved")
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn an_unknown_status_is_refused() {
        let db = fake_db().await;
        let filed = flag_page(&db.db(), BOOK, "/book/1", Some("Page 1"), "Une.")
            .await
            .expect("the database answers");

        let trouble = set_inspection_status(&db.db(), filed.id, "lost")
            .await
            .expect_err("lost is not a status");

        assert!(matches!(trouble, StatusError::Unknown(_)));
    }

    /// The unique constraint is the safety net under the find-or-create.
    #[tokio::test]
    async fn two_files_on_one_page_cannot_coexist() {
        let db = fake_db().await;
        let now = at(2026, 9, 10, 12, 0);
        let twins = (1..=2)
            .map(|number| {
                row! {
                    "id" => number as i64,
                    "book" => BOOK,
                    "path" => "/book/1",
                    "status" => "open",
                    "created_at" => now,
                    "updated_at" => now,
                }
            })
            .collect();

        let trouble = db
            .table("page_inspections")
            .set_docs(twins)
            .await
            .expect_err("two files on one page are refused");

        assert!(trouble.to_string().contains("UNIQUE constraint failed"));
    }
}

/// The menu entry and the form behind it, on every page.
mod the_dialog {
    use super::*;

    #[tokio::test]
    async fn every_page_carries_the_entry_and_the_form() {
        let (_db, client) = hero().await;

        for path in ["/", "/book/1", "/game", "/heroes", "/flagged-pages"] {
            let body = client.get(path).await.text();

            assert!(body.contains(r##"href="#signaler">Signaler<"##), "on {path}");
            assert!(body.contains(r#"action="/flag-page""#), "on {path}");
            assert!(
                body.contains(&format!(r#"name="path" value="{path}""#)),
                "on {path}"
            );
        }
    }

    #[tokio::test]
    async fn the_form_carries_the_page_title() {
        let (_db, client) = client().await;

        let body = client.get("/book/1").await.text();

        assert!(Regex::new(r#"name="title" value="Page 1 · [^"]+""#)
            .expect("the pattern is written here")
            .is_match(&body));
    }

    #[tokio::test]
    async fn the_landing_page_needs_no_database_for_it() {
        let client = no_database_client(books_path());

        assert!(client.get("/").await.text().contains("Signaler"));
    }

    #[tokio::test]
    async fn the_dialog_links_to_the_review_list() {
        let (_db, client) = client().await;

        assert!(client
            .get("/")
            .await
            .text()
            .contains(r#"href="/flagged-pages""#));
    }
}

/// Without a script: a flash message and back to the page.
mod reporting_as_a_form {
    use super::*;

    #[tokio::test]
    async fn the_report_is_filed_and_the_reader_sent_back() {
        let (db, client) = client().await;

        let response = report(&client, "/book/1", "Une coquille.").await;

        assert_eq!(response.status, 302);
        assert_eq!(response.location(), "/book/1");
        let filed = db.table("page_inspections").one().await;
        assert_eq!(filed["path"], Field::Str("/book/1".to_string()));
        assert_eq!(
            filed["page_title"],
            Field::Str("Page 1 · Titre".to_string())
        );
        let comments = filed["comments"].as_list().expect("a file has comments");
        assert_eq!(
            comments[0].as_dict().expect("one comment")["text"],
            Field::Str("Une coquille.".to_string())
        );
    }

    #[tokio::test]
    async fn the_page_then_says_thank_you() {
        let (_db, client) = client().await;
        let response = report(&client, "/book/1", "Une coquille.").await;
        let body = client.get(&response.location()).await.text();

        assert!(body.contains("la page est signalée"));
        assert!(body.contains(r#"class="toast success""#));
    }

    #[tokio::test]
    async fn the_thanks_is_said_once() {
        let (_db, client) = client().await;
        report(&client, "/book/1", "Une coquille.").await;
        client.get("/book/1").await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains("la page est signalée"));
    }

    #[tokio::test]
    async fn a_blank_remark_is_refused() {
        let (db, client) = client().await;

        let response = report(&client, "/book/1", "   ").await;

        assert_eq!(response.status, 302);
        assert!(db.table("page_inspections").docs().await.is_empty());
        assert!(client
            .get("/book/1")
            .await
            .text()
            .contains("Dites en quelques mots"));
    }

    #[tokio::test]
    async fn a_second_report_joins_the_first() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Une.").await;
        report(&client, "/book/1", "Deux.").await;

        let filed = db.table("page_inspections").one().await;
        let texts: Vec<String> = filed["comments"]
            .as_list()
            .expect("a file has comments")
            .iter()
            .filter_map(Field::as_dict)
            .map(|comment| comment["text"].to_string())
            .collect();
        assert_eq!(texts, ["Une.", "Deux."]);
    }

    #[tokio::test]
    async fn only_a_local_path_is_filed() {
        for path in ["https://elsewhere.example/x", "//evil", "book/1", ""] {
            let (db, client) = client().await;

            let response = report(&client, path, "Une coquille.").await;

            assert_eq!(response.location(), "/", "for {path}");
            assert_eq!(
                db.table("page_inspections").one().await["path"],
                Field::Str("/".to_string()),
                "for {path}"
            );
        }
    }
}

/// With the page's script: JSON, and the reader stays where he is.
mod reporting_as_a_script {
    use super::*;

    #[tokio::test]
    async fn the_answer_is_json() {
        let (db, client) = client().await;

        let response = report_as_script(&client, "/book/1", "Une coquille.").await;

        assert_eq!(response.status, 200);
        assert_eq!(
            response.json(),
            serde_json::json!({
                "success": true,
                "message": "Merci, la page est signalée pour inspection.",
                "comments": 1,
            })
        );
        assert_eq!(db.table("page_inspections").docs().await.len(), 1);
    }

    #[tokio::test]
    async fn the_comment_count_grows() {
        let (_db, client) = client().await;
        report_as_script(&client, "/book/1", "Une.").await;

        assert_eq!(
            report_as_script(&client, "/book/1", "Deux.").await.json()["comments"],
            2
        );
    }

    #[tokio::test]
    async fn a_blank_remark_is_a_400() {
        let (db, client) = client().await;

        let response = report_as_script(&client, "/book/1", "").await;

        assert_eq!(response.status, 400);
        assert_eq!(response.json()["success"], false);
        assert!(db.table("page_inspections").docs().await.is_empty());
    }

    #[tokio::test]
    async fn no_flash_is_left_behind() {
        let (_db, client) = client().await;
        report_as_script(&client, "/book/1", "Une coquille.").await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains(r#"class="toast success""#));
    }

    #[tokio::test]
    async fn an_unreachable_database_answers_json() {
        let client = no_database_client(books_path());

        let response = report_as_script(&client, "/book/1", "Une coquille.").await;

        assert_eq!(response.status, 503);
        assert_eq!(response.json()["success"], false);
    }

    #[tokio::test]
    async fn an_unreachable_database_answers_a_page_to_a_form() {
        let client = no_database_client(books_path());

        let response = report(&client, "/book/1", "Une coquille.").await;

        assert_eq!(response.status, 503);
        assert!(response.content_type().starts_with("text/html"));
    }
}

/// `/flagged-pages`.
mod the_review_list {
    use super::*;

    #[tokio::test]
    async fn it_says_when_nothing_is_flagged() {
        let (_db, client) = client().await;

        assert!(client
            .get("/flagged-pages")
            .await
            .text()
            .contains("Aucune page n'a été signalée"));
    }

    #[tokio::test]
    async fn a_flagged_page_is_listed() {
        let (_db, client) = client().await;
        report(&client, "/book/1", "Le choix mène au mauvais paragraphe.").await;
        let body = client.get("/flagged-pages").await.text();

        assert!(body.contains("Page 1 · Titre"));
        assert!(body.contains("/book/1"));
        assert!(body.contains("1 remarque"));
        assert!(body.contains("Le choix mène au mauvais paragraphe."));
        assert!(body.contains("À traiter"));
    }

    #[tokio::test]
    async fn it_links_to_the_file() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Une coquille.").await;
        let filed = db.table("page_inspections").one().await;

        assert!(client
            .get("/flagged-pages")
            .await
            .text()
            .contains(&format!(r#"href="/flagged-pages/{}""#, filed["id"])));
    }

    #[tokio::test]
    async fn the_last_remark_is_the_one_shown() {
        let (_db, client) = client().await;
        report(&client, "/book/1", "Première.").await;
        report(&client, "/book/1", "Dernière.").await;
        let body = client.get("/flagged-pages").await.text();

        assert!(body.contains("Dernière."));
        assert!(!body.contains("Première."));
        assert!(body.contains("2 remarques"));
    }

    #[tokio::test]
    async fn it_filters_by_status() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Une.").await;
        report(&client, "/book/2", "Deux.").await;
        let second = db.table("page_inspections").docs().await[1].clone();
        client
            .post(&format!("/flagged-pages/{}/resolve", second["id"]))
            .await;

        assert!(!client
            .get("/flagged-pages?status=open")
            .await
            .text()
            .contains("/book/2"));
        assert!(!client
            .get("/flagged-pages?status=resolved")
            .await
            .text()
            .contains("/book/1"));
    }

    #[tokio::test]
    async fn an_empty_filter_says_which() {
        let (_db, client) = client().await;

        assert!(client
            .get("/flagged-pages?status=open")
            .await
            .text()
            .contains("Aucune page à traiter"));
        assert!(client
            .get("/flagged-pages?status=resolved")
            .await
            .text()
            .contains("Aucune page réglée"));
    }

    #[tokio::test]
    async fn an_unknown_status_shows_everything() {
        let (_db, client) = client().await;
        report(&client, "/book/1", "Une coquille.").await;

        assert!(client
            .get("/flagged-pages?status=lost")
            .await
            .text()
            .contains("/book/1"));
    }
}

/// `/flagged-pages/<id>`: the whole thread, and the verdict.
mod the_file {
    use super::*;

    #[tokio::test]
    async fn every_remark_is_shown() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Première.").await;
        report(&client, "/book/1", "Dernière.").await;
        let filed = db.table("page_inspections").one().await;
        let body = client
            .get(&format!("/flagged-pages/{}", filed["id"]))
            .await
            .text();

        assert!(body.contains("Première."));
        assert!(body.contains("Dernière."));
        assert!(body.contains("2 remarques"));
        assert!(body.contains(r#"href="/book/1""#));
    }

    #[tokio::test]
    async fn an_unknown_file_is_not_found() {
        let (_db, client) = client().await;

        assert_eq!(client.get("/flagged-pages/999").await.status, 404);
        assert_eq!(client.get("/flagged-pages/nope").await.status, 404);
    }

    #[tokio::test]
    async fn it_can_be_resolved() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Une coquille.").await;
        let filed = db.table("page_inspections").one().await;

        let response = client
            .post(&format!("/flagged-pages/{}/resolve", filed["id"]))
            .await;

        assert!(response
            .location()
            .ends_with(&format!("/flagged-pages/{}", filed["id"])));
        assert_eq!(
            db.table("page_inspections").one().await["status"],
            Field::Str("resolved".to_string())
        );
        assert!(client
            .get(&response.location())
            .await
            .text()
            .contains("Rouvrir"));
    }

    #[tokio::test]
    async fn it_can_be_reopened() {
        let (db, client) = client().await;
        report(&client, "/book/1", "Une coquille.").await;
        let filed = db.table("page_inspections").one().await;
        client
            .post(&format!("/flagged-pages/{}/resolve", filed["id"]))
            .await;

        client
            .post(&format!("/flagged-pages/{}/reopen", filed["id"]))
            .await;

        assert_eq!(
            db.table("page_inspections").one().await["status"],
            Field::Str("open".to_string())
        );
        assert!(client
            .get(&format!("/flagged-pages/{}", filed["id"]))
            .await
            .text()
            .contains("Marquer comme réglée"));
    }

    #[tokio::test]
    async fn resolving_an_unknown_file_is_not_found() {
        let (_db, client) = client().await;

        assert_eq!(client.post("/flagged-pages/999/resolve").await.status, 404);
        assert_eq!(client.post("/flagged-pages/999/reopen").await.status, 404);
    }
}

/// Keeps the unused import of `client_for` honest for the fixtures above.
#[tokio::test]
async fn a_client_can_be_built_on_a_book_of_its_own() {
    let db = fake_db().await;
    let client = client_for(db.db(), books_path(), None);

    assert_eq!(client.get("/").await.status, 200);
}
