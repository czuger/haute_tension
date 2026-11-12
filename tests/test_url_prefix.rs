//! The site served under a sub-path, as a proxy in front of it serves it.
//!
//! The proxy strips the sub-path before the request arrives, so the routes answer
//! their own addresses here exactly as they do at the root — what changes is
//! everything written back out: every link, every redirect and the one cookie.

#[path = "common/mod.rs"]
mod common;

use std::sync::Arc;

use common::{books_path, books_path_with_fights, client, client_under, fake_db, seeded, FakeDb, TestClient};
use haute_tension::core::db::Db;

/// The sub-path this site is served under in these tests.
const PREFIX: &str = "/haute-tension";

/// A client under the sub-path, on a book with fights and a hero already rolled.
async fn hero_under_the_prefix() -> (FakeDb, TestClient) {
    let db = fake_db().await;
    let client = client_under(
        PREFIX,
        db.db(),
        books_path_with_fights(),
        Some(Arc::new(seeded(1))),
    );
    client.post("/game/new").await;
    (db, client)
}

/// What a reader clicks: the menu, the story, the forms.
mod the_links {
    use super::*;

    #[tokio::test]
    async fn the_menu_is_built_under_the_prefix() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let body = client.get("/").await.text();

        for address in ["/", "/game/new", "/game/resume", "/game", "/heroes"] {
            assert!(
                body.contains(&format!("href=\"{PREFIX}{address}\"")),
                "the menu links to {PREFIX}{address}"
            );
        }
    }

    #[tokio::test]
    async fn the_opening_page_is_linked_under_the_prefix() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let body = client.get("/").await.text();

        assert!(body.contains(&format!("href=\"{PREFIX}/book/1\"")));
    }

    #[tokio::test]
    async fn the_story_is_built_under_the_prefix() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let body = client.get("/book/1").await.text();

        assert!(body.contains(&format!("href=\"{PREFIX}/book/2\"")));
        assert!(body.contains(&format!("href=\"{PREFIX}/game\"")));
    }

    /// A choice is a form once there is a hero to spend his rations.
    #[tokio::test]
    async fn a_choice_is_posted_under_the_prefix() {
        let (_db, client) = hero_under_the_prefix().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains(&format!("action=\"{PREFIX}/book/1/choice/0\"")));
    }

    #[tokio::test]
    async fn the_breadcrumb_leads_back_inside_the_site() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);
        client.get("/book/1").await;

        let body = client.get("/book/2").await.text();

        assert!(body.contains(&format!("href=\"{PREFIX}/book/1\"")));
    }

    #[tokio::test]
    async fn a_fight_is_fought_under_the_prefix() {
        let (_db, client) = hero_under_the_prefix().await;

        let body = client.get("/combat/2").await.text();

        assert!(body.contains(&format!("action=\"{PREFIX}/combat/2/assault\"")));
    }

    #[tokio::test]
    async fn nothing_is_prefixed_when_the_site_is_at_the_root() {
        let (_db, client) = client().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains("href=\"/game\""));
        assert!(body.contains("href=\"/book/2\""));
        assert!(!body.contains(PREFIX));
    }
}

/// Where a form sends the reader once it has done its work.
mod the_redirects {
    use super::*;

    #[tokio::test]
    async fn a_new_hero_lands_on_his_sheet_inside_the_site() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let response = client.post_form("/game/new", &[("mode", "normal")]).await;

        assert_eq!(response.status, 302);
        assert_eq!(response.location(), format!("{PREFIX}/game?rolled=1"));
    }

    #[tokio::test]
    async fn a_choice_followed_leads_to_its_page_inside_the_site() {
        let (_db, client) = hero_under_the_prefix().await;
        client.get("/book/1").await;

        let response = client.post("/book/1/choice/0").await;

        assert_eq!(response.status, 302);
        assert!(
            response.location().starts_with(&format!("{PREFIX}/book/")),
            "a choice leads under the prefix, got {}",
            response.location()
        );
    }

    #[tokio::test]
    async fn a_redirect_is_bare_when_the_site_is_at_the_root() {
        let (_db, client) = client().await;

        let response = client.post_form("/game/new", &[("mode", "normal")]).await;

        assert_eq!(response.location(), "/game?rolled=1");
    }
}

/// The one cookie, and how wide it is sent.
mod the_session_cookie {
    use super::*;

    #[tokio::test]
    async fn it_is_sent_back_on_the_sub_path_only() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let cookie = client
            .post_form("/game/new", &[("mode", "normal")])
            .await
            .header("set-cookie")
            .expect("a new hero is written into the session");

        assert!(cookie.contains("haute_tension_session="));
        assert!(
            cookie.contains(&format!("Path={PREFIX}/")),
            "the cookie is scoped to the site, got {cookie}"
        );
    }

    #[tokio::test]
    async fn it_is_sent_back_on_everything_at_the_root() {
        let (_db, client) = client().await;

        let cookie = client
            .post_form("/game/new", &[("mode", "normal")])
            .await
            .header("set-cookie")
            .expect("a new hero is written into the session");

        assert!(cookie.contains("Path=/;"));
    }
}

/// A page flagged from behind the proxy: the link carries the prefix, the report
/// does not — the address filed against is the application's own.
mod flagging_a_page {
    use super::*;

    #[tokio::test]
    async fn the_dialog_posts_under_the_prefix_and_names_the_bare_address() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let body = client.get("/book/1").await.text();

        assert!(body.contains(&format!("action=\"{PREFIX}/flag-page\"")));
        assert!(body.contains(r#"value="/book/1""#));
    }

    #[tokio::test]
    async fn the_flagged_list_links_back_under_the_prefix() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);
        client
            .post_form(
                "/flag-page",
                &[
                    ("path", "/book/1"),
                    ("title", "Paragraphe 1"),
                    ("comment", "Le texte est coupé."),
                ],
            )
            .await;

        let filed = db.table("page_inspections").one().await;
        let body = client.get("/flagged-pages").await.text();

        assert_eq!(filed["path"].to_string(), "/book/1");
        assert!(body.contains(&format!("href=\"{PREFIX}/flagged-pages/1\"")));
    }
}

/// The page a reader is shown when the database cannot answer carries the menu,
/// which has to lead back into the site like any other.
mod the_database_page {
    use super::*;

    #[tokio::test]
    async fn its_menu_stays_inside_the_site() {
        let client = client_under(
            PREFIX,
            Db::Failing("unable to open database file".to_string()),
            books_path(),
            None,
        );

        let response = client.get("/book/1").await;

        assert_eq!(response.status, 503);
        assert!(response.text().contains(&format!("href=\"{PREFIX}/heroes\"")));
    }
}

/// The addresses the routes answer never carry the sub-path: the proxy strips it,
/// and a request that still has it matched no route at all.
mod the_addresses_answered {
    use super::*;

    #[tokio::test]
    async fn a_request_carrying_the_prefix_is_not_found() {
        let db = fake_db().await;
        let client = client_under(PREFIX, db.db(), books_path(), None);

        let response = client.get(&format!("{PREFIX}/book/1")).await;

        assert_eq!(response.status, 404);
    }
}
