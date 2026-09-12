//! The site menu: Nouveau, Partie en cours, Feuille, Les tombés.
//!
//! Ported from `tests/test_menu.py`.

#[path = "common/mod.rs"]
mod common;

use common::{at, client, fighting_client, hero, FakeDb};
use haute_tension::application::french_date::french_date;
use haute_tension::application::models::game::SESSION_KEY;
use haute_tension::core::models::hybrid_document::Field;
use regex::Regex;

/// Lay the one stored hero to rest, in place.
async fn bury(db: &FakeDb, page: &str, cause: &str) {
    let mut game = db.table("games").one().await;
    game.insert("vie_actuelle".to_string(), Field::Int(0));
    game.insert(
        "died_at".to_string(),
        Field::Instant(at(2026, 9, 10, 12, 0)),
    );
    game.insert("died_on_page".to_string(), Field::Str(page.to_string()));
    game.insert("died_of".to_string(), Field::Str(cause.to_string()));
    db.table("games").seed(vec![game]).await;
}

/// Four entries, on every page, hero or not.
mod the_menu {
    use super::*;

    #[tokio::test]
    async fn every_page_carries_the_four_entries() {
        let (_db, client) = hero().await;

        for path in ["/", "/book/1", "/game", "/heroes", "/game/new"] {
            let body = client.get(path).await.text();

            assert!(body.contains(r#"href="/game/new">Nouveau<"#), "on {path}");
            assert!(
                body.contains(r#"href="/game/resume">Partie en cours<"#),
                "on {path}"
            );
            assert!(body.contains(r#"href="/game">Feuille<"#), "on {path}");
            assert!(body.contains(r#"href="/heroes">Les tombés<"#), "on {path}");
        }
    }

    #[tokio::test]
    async fn a_reader_without_a_hero_has_the_same_menu() {
        let (_db, client) = client().await;

        let body = client.get("/").await.text();

        assert!(body.contains(r#"href="/game/new">Nouveau<"#));
        assert!(body.contains(r#"href="/game/resume">Partie en cours<"#));
    }
}

/// Offering a hero, and saying what it costs.
mod nouveau {
    use super::*;

    #[tokio::test]
    async fn it_offers_the_dice() {
        let (_db, client) = fighting_client().await;

        let response = client.get("/game/new").await;

        assert_eq!(response.status, 200);
        assert!(response.text().contains("Lancer les dés"));
        assert!(response.text().contains(r#"value="normal""#));
        assert!(response.text().contains(r#"value="easy""#));
    }

    #[tokio::test]
    async fn it_rolls_nothing_by_itself() {
        let (db, client) = fighting_client().await;
        client.get("/game/new").await;

        assert!(db.table("games").docs().await.is_empty());
    }

    #[tokio::test]
    async fn without_a_hero_there_is_nothing_to_warn_about() {
        let (_db, client) = fighting_client().await;

        assert!(!client
            .get("/game/new")
            .await
            .text()
            .contains("déjà en route"));
    }

    #[tokio::test]
    async fn a_living_hero_is_shown_before_being_abandoned() {
        let (db, client) = hero().await;
        let game = db.table("games").one().await;
        let body = client.get("/game/new").await.text();

        assert!(body.contains("Un héros est déjà en route"));
        assert!(body.contains(&format!("Force {}", game["force"])));
        assert!(body.contains(&format!(
            "Vie {}/{}",
            game["vie_actuelle"], game["vie_max"]
        )));
        assert!(body.contains(r#"href="/game/resume""#));
    }

    #[tokio::test]
    async fn a_hero_in_a_fight_is_said_to_be() {
        let (_db, client) = hero().await;
        client.get("/combat/1").await;

        assert!(client
            .get("/game/new")
            .await
            .text()
            .contains("aux prises au paragraphe 1"));
    }

    #[tokio::test]
    async fn the_offer_leaves_the_living_hero_untouched() {
        let (db, client) = hero().await;
        let before = db.table("games").docs().await;
        client.get("/game/new").await;

        assert_eq!(db.table("games").docs().await, before);
    }

    #[tokio::test]
    async fn a_dead_hero_is_told_of_instead() {
        let (db, client) = hero().await;
        bury(&db, "22", "un troll").await;
        let body = client.get("/game/new").await.text();

        assert!(body.contains("Votre héros est mort au paragraphe 22"));
        assert!(!body.contains("déjà en route"));
    }

    #[tokio::test]
    async fn rolling_from_it_replaces_the_hero() {
        let (db, client) = hero().await;
        let first = client.session().get_id(SESSION_KEY);
        client.post_form("/game/new", &[("mode", "easy")]).await;

        assert_ne!(client.session().get_id(SESSION_KEY), first);
        assert_eq!(db.table("games").docs().await.len(), 2);
    }

    #[tokio::test]
    async fn the_abandoned_hero_is_kept_but_not_among_the_fallen() {
        let (db, client) = hero().await;
        client.post("/game/new").await;

        assert_eq!(db.table("games").docs().await.len(), 2);
        assert!(client
            .get("/heroes")
            .await
            .text()
            .contains("Personne n'est encore tombé"));
    }

    #[tokio::test]
    async fn the_sheet_links_to_it_rather_than_rolling() {
        let (_db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(body.contains(r#"href="/game/new">Abandonner et créer un nouveau héros"#));
        assert!(!body.contains(r#"action="/game/new""#));
    }
}

/// Back to wherever the adventure stands.
mod partie_en_cours {
    use super::*;

    #[tokio::test]
    async fn without_a_hero_it_offers_one() {
        let (_db, client) = fighting_client().await;

        let response = client.get("/game/resume").await;

        assert_eq!(response.status, 302);
        assert!(response.location().ends_with("/game/new"));
    }

    #[tokio::test]
    async fn a_hero_who_has_read_nothing_starts_at_the_opening() {
        let (_db, client) = hero().await;

        let response = client.get("/game/resume").await;

        assert!(response.location().ends_with("/book/1"));
    }

    #[tokio::test]
    async fn it_goes_back_to_the_last_page_read() {
        let (_db, client) = hero().await;
        client.get("/book/2").await;
        client.get("/book/4").await;

        assert!(client
            .get("/game/resume")
            .await
            .location()
            .ends_with("/book/4"));
    }

    #[tokio::test]
    async fn it_goes_back_into_an_open_fight() {
        let (_db, client) = hero().await;
        client.get("/book/2").await;
        client.get("/combat/2").await;

        assert!(client
            .get("/game/resume")
            .await
            .location()
            .ends_with("/combat/2"));
    }

    #[tokio::test]
    async fn a_dead_hero_is_sent_to_the_death_page() {
        let (db, client) = hero().await;
        client.get("/book/2").await;
        bury(&db, "22", "un troll").await;

        assert!(client
            .get("/game/resume")
            .await
            .location()
            .ends_with("/game/death"));
    }

    #[tokio::test]
    async fn it_records_no_visit_of_its_own() {
        let (db, client) = hero().await;
        client.get("/book/2").await;
        client.get("/game/resume").await;

        let pages: Vec<String> = db
            .table("page_views")
            .docs()
            .await
            .iter()
            .map(|view| view["page"].to_string())
            .collect();
        assert_eq!(pages, ["2"]);
    }

    /// Another hero's last page is not this one's.
    #[tokio::test]
    async fn the_page_it_lands_on_is_the_heros() {
        let (_db, client) = hero().await;
        client.get("/book/2").await;
        client.post("/game/new").await;

        assert!(client
            .get("/game/resume")
            .await
            .location()
            .ends_with("/book/1"));
    }

    #[tokio::test]
    async fn a_stale_session_offers_a_hero() {
        let (_db, client) = fighting_client().await;
        client.set_session(SESSION_KEY, serde_json::Value::from("gone"));

        let response = client.get("/game/resume").await;

        assert!(response.location().ends_with("/game/new"));
    }
}

/// The sheet reads everything and changes nothing.
mod feuille {
    use super::*;

    #[tokio::test]
    async fn the_changes_waiting_on_the_reader_are_listed() {
        let (_db, client) = hero().await;
        client.post("/book/1/choice/1").await;
        let body = client.get("/game").await.text();

        assert!(body.contains("À vous de voir"));
        assert!(body.contains("1 effet en attente"));
        assert!(body.contains("point de Force"));
        assert!(body.contains("pendant tout le temps où vous les porterez"));
        assert!(body.contains(r#"href="/book/1">paragraphe 1<"#));
    }

    #[tokio::test]
    async fn without_any_the_section_is_absent() {
        let (_db, client) = hero().await;

        assert!(!client.get("/game").await.text().contains("À vous de voir"));
    }

    /// Only the site-wide flag dialog, outside the card, holds a form.
    #[tokio::test]
    async fn the_sheet_posts_nothing() {
        let (_db, client) = hero().await;
        let body = client.get("/game").await.text();

        let card = Regex::new(r#"(?s)<article class="card.*?</article>"#)
            .expect("the pattern is written here")
            .find(&body)
            .expect("the sheet is a card")
            .as_str();

        assert!(!card.contains("<form"));
    }

    #[tokio::test]
    async fn the_sheet_never_changes_the_hero() {
        let (db, client) = hero().await;
        client.post("/book/1/choice/1").await;
        let before = db.table("games").docs().await;

        client.get("/game").await;

        assert_eq!(db.table("games").docs().await, before);
    }
}

/// The memorial, dated.
mod les_tombes {
    use super::*;

    #[tokio::test]
    async fn the_day_he_fell_is_shown() {
        let (db, client) = hero().await;
        bury(&db, "22", "un troll").await;

        assert!(client
            .get("/heroes")
            .await
            .text()
            .contains("le 10 septembre 2026"));
    }

    #[tokio::test]
    async fn without_a_fallen_hero_it_says_so() {
        let (_db, client) = hero().await;

        assert!(client
            .get("/heroes")
            .await
            .text()
            .contains("Personne n'est encore tombé"));
    }
}

/// The filter behind the memorial's dates.
mod the_french_date {
    use super::*;

    #[test]
    fn an_instant_reads_as_a_french_date() {
        assert_eq!(
            french_date(Some("2026-09-10T12:00:00+00:00")),
            "10 septembre 2026"
        );
    }

    #[test]
    fn the_first_of_a_month() {
        assert_eq!(
            french_date(Some("2025-01-01T00:00:00+00:00")),
            "1 janvier 2025"
        );
    }

    #[test]
    fn nothing_reads_as_nothing() {
        assert_eq!(french_date(None), "");
    }
}
