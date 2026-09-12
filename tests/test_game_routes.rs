//! Tests for rolling up a hero and fighting with him through the browser.

#[path = "common/mod.rs"]
mod common;

use common::{fake_db, fighting_client, hero, FakeDb, TestClient};
use haute_tension::application::models::game::SESSION_KEY;
use haute_tension::core::models::hybrid_document::{Field, RowDict};
use haute_tension::row;
use regex::Regex;
use serde_json::Value;

/// Whether the fight on a page is still open, won or lost.
async fn combat_status(client: &TestClient, page: &str) -> &'static str {
    let body = client.get(&format!("/combat/{page}")).await.text();
    if body.contains("Vous l'emportez") {
        return "victory";
    }
    if body.contains("Vous êtes vaincu") {
        return "defeat";
    }
    "ongoing"
}

/// Play assaults until the fight is decided, and say how it went.
async fn fight_to_the_end(client: &TestClient, page: &str) -> &'static str {
    for _ in 0..200 {
        let status = combat_status(client, page).await;
        if status != "ongoing" {
            return status;
        }
        client.post(&format!("/combat/{page}/assault")).await;
    }
    panic!("the fight never ended");
}

/// The one stored game, as it now stands.
async fn stored(db: &FakeDb) -> RowDict {
    db.table("games").one().await
}

/// One field of a combat blob spelled out by a test.
fn combat_blob(entries: RowDict) -> Field {
    Field::Dict(entries)
}

/// Rolling up Prêtre Jean.
mod creating_a_hero {
    use super::*;

    #[tokio::test]
    async fn without_a_game_the_reader_is_offered_one() {
        let (_db, client) = fighting_client().await;

        let response = client.get("/game").await;

        assert_eq!(response.status, 200);
        assert!(response.text().contains("Lancer les dés"));
    }

    #[tokio::test]
    async fn rolling_up_creates_a_game() {
        let (db, client) = fighting_client().await;
        client.post("/game/new").await;

        assert_eq!(db.table("games").docs().await.len(), 1);
    }

    #[tokio::test]
    async fn the_game_id_goes_into_the_session() {
        let (_db, client) = fighting_client().await;
        client.post("/game/new").await;

        assert!(client.session().get_id(SESSION_KEY).is_some());
    }

    #[tokio::test]
    async fn the_sheet_shows_its_working() {
        let (_db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(Regex::new(r"\d \+ \d \(2D6\)\s*\+ 6 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
        assert!(Regex::new(r"\d \+ \d \(2D6\)\s*\+ 18 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
    }

    #[tokio::test]
    async fn the_sheet_shows_force_and_vie() {
        let (db, client) = hero().await;
        let game = stored(&db).await;
        let body = client.get("/game").await.text();

        assert!(body.contains(&format!("<strong>{}</strong>", game["force"])));
        assert!(body.contains(&format!("<strong>{}</strong>", game["vie_actuelle"])));
        assert!(body.contains(&format!("/{}", game["vie_max"])));
    }

    /// The dice are thrown once; every later view loads what was written.
    #[tokio::test]
    async fn reloading_the_sheet_never_re_rolls() {
        let (db, client) = hero().await;
        let first = db.table("games").docs().await;

        for _ in 0..4 {
            client.get("/game").await;
        }

        assert_eq!(db.table("games").docs().await, first);
    }

    #[tokio::test]
    async fn a_stale_session_falls_back_to_rolling_up() {
        let (_db, client) = fighting_client().await;
        client.set_session(SESSION_KEY, Value::from("a-game-that-was-deleted"));

        assert!(client.get("/game").await.text().contains("Lancer les dés"));
    }

    #[tokio::test]
    async fn a_stale_session_is_cleared() {
        let (_db, client) = fighting_client().await;
        client.set_session(SESSION_KEY, Value::from("gone"));
        client.get("/game").await;

        assert!(client.session().get(SESSION_KEY).is_none());
    }

    #[tokio::test]
    async fn both_difficulties_are_offered() {
        let (_db, client) = fighting_client().await;

        let body = client.get("/game").await.text();

        assert!(body.contains("Difficulté normale"));
        assert!(body.contains("Difficulté facile"));
    }

    #[tokio::test]
    async fn each_difficulty_shows_its_dice_and_range() {
        let (_db, client) = fighting_client().await;

        let body = client.get("/game").await.text();

        assert!(body.contains("Force 6 + 2D6"));
        assert!(body.contains("Force 12 + 2D4"));
        assert!(body.contains("Vie 26 + 3D4"));
        assert!(body.contains("29–38"));
    }

    #[tokio::test]
    async fn choosing_the_easy_mode_rolls_an_easy_hero() {
        let (db, client) = fighting_client().await;
        client.post_form("/game/new", &[("mode", "easy")]).await;

        let game = stored(&db).await;
        assert_eq!(game["mode"], Field::Str("easy".to_string()));
        let force = game["force"].as_int().expect("a hero has a Force");
        assert!((14..=20).contains(&force), "force {force}");
        let vie = game["vie_max"].as_int().expect("a hero has a Vie");
        assert!((29..=38).contains(&vie), "vie {vie}");
        assert_eq!(
            game["vie_dice"].as_list().expect("the dice are kept").len(),
            3
        );
    }

    #[tokio::test]
    async fn choosing_nothing_rolls_by_the_book() {
        let (db, client) = fighting_client().await;
        client.post("/game/new").await;

        let game = stored(&db).await;
        assert_eq!(game["mode"], Field::Str("normal".to_string()));
        let force = game["force"].as_int().expect("a hero has a Force");
        assert!((8..=18).contains(&force), "force {force}");
    }

    /// A posted value is whatever was posted; it must still roll a hero.
    #[tokio::test]
    async fn an_unknown_mode_rolls_by_the_book() {
        let (db, client) = fighting_client().await;

        let response = client
            .post_form("/game/new", &[("mode", "invincible")])
            .await;

        assert_eq!(response.status, 302);
        assert_eq!(stored(&db).await["mode"], Field::Str("normal".to_string()));
    }

    #[tokio::test]
    async fn the_easy_sheet_prints_its_own_throw() {
        let (_db, client) = fighting_client().await;
        client.post_form("/game/new", &[("mode", "easy")]).await;
        let body = client.get("/game").await.text();

        assert!(body.contains("Difficulté facile"));
        assert!(Regex::new(r"\d \+ \d \(2D4\)\s*\+ 12 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
        assert!(Regex::new(r"\d \+ \d \+ \d \(3D4\)\s*\+ 26 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
    }

    #[tokio::test]
    async fn a_new_hero_replaces_the_old_one() {
        let (_db, client) = hero().await;
        let first = client.session().get_id(SESSION_KEY);
        client.post("/game/new").await;

        assert_ne!(client.session().get_id(SESSION_KEY), first);
    }
}

/// Getting from a page into the fight it holds.
mod reaching_a_combat {
    use super::*;

    #[tokio::test]
    async fn a_page_with_a_fight_offers_it() {
        let (_db, client) = hero().await;

        assert!(client
            .get("/book/1")
            .await
            .text()
            .contains("Engager le combat"));
    }

    #[tokio::test]
    async fn a_page_without_one_does_not() {
        let (_db, client) = common::client().await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains("Engager le combat"));
    }

    #[tokio::test]
    async fn the_fight_opens() {
        let (_db, client) = hero().await;

        let response = client.get("/combat/1").await;

        assert_eq!(response.status, 200);
        assert!(response.text().contains("collecteur"));
    }

    #[tokio::test]
    async fn both_sides_are_shown() {
        let (_db, client) = hero().await;

        let body = client.get("/combat/1").await.text();

        assert!(body.contains("Prêtre Jean"));
        assert!(body.contains("Force 6"));
    }

    /// The parser does emit fights whose adversaries are unusable.
    #[tokio::test]
    async fn a_fight_that_fields_nobody_is_not_a_combat() {
        let (_db, client) = hero().await;

        assert_eq!(client.get("/combat/3").await.status, 404);
    }

    /// Page 3 looks like it holds a fight, but nothing in it can be fielded.
    #[tokio::test]
    async fn an_assault_on_an_unfieldable_fight_changes_nothing() {
        let (db, client) = hero().await;
        let before = stored(&db).await;

        let response = client.post("/combat/3/assault").await;

        assert_eq!(response.status, 302);
        assert_eq!(stored(&db).await, before);
    }

    #[tokio::test]
    async fn an_assault_on_an_unknown_page_changes_nothing() {
        let (db, client) = hero().await;
        let before = stored(&db).await;

        let response = client.post("/combat/999/assault").await;

        assert_eq!(response.status, 302);
        assert_eq!(stored(&db).await, before);
    }

    #[tokio::test]
    async fn an_unknown_page_has_no_combat() {
        let (_db, client) = hero().await;

        assert_eq!(client.get("/combat/999").await.status, 404);
    }

    #[tokio::test]
    async fn without_a_hero_the_reader_is_sent_to_roll_one() {
        let (_db, client) = fighting_client().await;

        let response = client.get("/combat/1").await;

        assert_eq!(response.status, 302);
        assert!(response.location().ends_with("/game"));
    }
}

/// One assault per click.
mod fighting_an_assault {
    use super::*;

    #[tokio::test]
    async fn an_assault_is_logged() {
        let (_db, client) = hero().await;
        client.post("/combat/1/assault").await;
        let body = client.get("/combat/1").await.text();

        assert!(body.contains("Journal des assauts"));
        assert!(body.contains("Assaut 1"));
    }

    #[tokio::test]
    async fn the_assault_shows_its_dice_and_forces() {
        let (_db, client) = hero().await;
        client.post("/combat/1/assault").await;
        let body = client.get("/combat/1").await.text();

        assert!(Regex::new(r"(?s)Assaut 1 .*? \d \+ \d \+ Force \d+")
            .expect("the pattern is written here")
            .is_match(&body));
    }

    #[tokio::test]
    async fn assaults_accumulate() {
        let (_db, client) = hero().await;
        for _ in 0..2 {
            if combat_status(&client, "1").await == "ongoing" {
                client.post("/combat/1/assault").await;
            }
        }
        let body = client.get("/combat/1").await.text();

        assert!(body.contains("Assaut 1"));
    }

    #[tokio::test]
    async fn an_assault_costs_somebody_vie() {
        let (db, client) = hero().await;
        let before = stored(&db).await;
        client.post("/combat/1/assault").await;
        let after = stored(&db).await;

        let hero_hurt = after["vie_actuelle"] < before["vie_actuelle"];
        let combat = after["combat"].as_dict().expect("the fight is on");
        let enemy = combat["enemies"].as_list().expect("the fight has enemies")[0]
            .as_dict()
            .expect("one adversary");
        let enemy_hurt = enemy["vie_actuelle"] < enemy["vie_max"];
        assert!(hero_hurt || enemy_hurt);
    }

    #[tokio::test]
    async fn reloading_the_page_does_not_play_an_assault() {
        let (db, client) = hero().await;
        client.post("/combat/1/assault").await;
        let after_one = stored(&db).await;

        for _ in 0..3 {
            client.get("/combat/1").await;
        }

        assert_eq!(stored(&db).await, after_one);
    }

    #[tokio::test]
    async fn the_fight_reaches_a_verdict() {
        let (_db, client) = hero().await;

        assert!(["victory", "defeat"].contains(&fight_to_the_end(&client, "1").await));
    }

    #[tokio::test]
    async fn a_decided_fight_offers_to_move_on() {
        let (_db, client) = hero().await;
        fight_to_the_end(&client, "1").await;

        assert!(client
            .get("/combat/1")
            .await
            .text()
            .contains("Poursuivre l'aventure"));
    }

    #[tokio::test]
    async fn a_decided_fight_takes_no_more_assaults() {
        let (db, client) = hero().await;
        fight_to_the_end(&client, "1").await;
        let decided = stored(&db).await;
        client.post("/combat/1/assault").await;

        assert_eq!(stored(&db).await, decided);
    }

    #[tokio::test]
    async fn an_assault_without_a_hero_is_harmless() {
        let (db, client) = fighting_client().await;

        let response = client.post("/combat/1/assault").await;

        assert_eq!(response.status, 302);
        assert!(db.table("games").docs().await.is_empty());
    }

    /// Page 4 exists and holds no fight, so there is nothing to arm.
    #[tokio::test]
    async fn an_assault_on_a_page_holding_no_fight_changes_nothing() {
        let (db, client) = hero().await;
        let before = stored(&db).await;

        let response = client.post("/combat/4/assault").await;

        assert_eq!(response.status, 302);
        assert_eq!(stored(&db).await, before);
    }
}

/// Following the book once the fight is decided.
mod leaving_a_combat {
    use super::*;

    #[tokio::test]
    async fn a_victory_follows_the_book() {
        let (_db, client) = hero().await;
        if fight_to_the_end(&client, "1").await != "victory" {
            return;
        }

        let response = client.post("/combat/1/resolve").await;

        assert_eq!(response.location(), "/book/2");
    }

    /// on_defeat is "death" here, so the hero is sent to the death page.
    #[tokio::test]
    async fn a_defeat_ends_the_adventure() {
        let db = fake_db().await;
        let client = common::client_for(
            db.db(),
            common::books_path_with_fights(),
            Some(std::sync::Arc::new(common::seeded(1))),
        );
        client.post("/game/new").await;
        let mut game = stored(&db).await;
        game.insert("vie_actuelle".to_string(), Field::Int(0));
        game.insert(
            "combat".to_string(),
            combat_blob(row! {
                "page" => "1",
                "fight_type" => "single",
                "status" => "defeat",
                "on_victory" => "2",
                "on_defeat" => "death",
                "enemies" => Field::List(vec![Field::Dict(row! {
                    "name" => "collecteur",
                    "force" => 6_i64,
                    "vie_max" => 10_i64,
                    "vie_actuelle" => 4_i64,
                })]),
                "assaults" => Field::List(Vec::new()),
            }),
        );
        db.table("games").seed(vec![game]).await;

        let response = client.post("/combat/1/resolve").await;

        assert!(response.location().ends_with("/game/death"));
    }

    #[tokio::test]
    async fn leaving_clears_the_fight() {
        let (db, client) = hero().await;
        fight_to_the_end(&client, "1").await;
        client.post("/combat/1/resolve").await;

        assert_eq!(stored(&db).await["combat"], Field::Null);
    }

    #[tokio::test]
    async fn an_undecided_fight_goes_back_to_the_page() {
        let (_db, client) = hero().await;

        let response = client.post("/combat/1/resolve").await;

        assert_eq!(response.location(), "/book/1");
    }

    /// Seeded rather than fought: whether one assault decides it is chance.
    #[tokio::test]
    async fn an_undecided_fight_is_not_cleared() {
        let (db, client) = hero().await;
        let mut game = stored(&db).await;
        game.insert(
            "combat".to_string(),
            combat_blob(row! {
                "page" => "1",
                "fight_type" => "single",
                "status" => "ongoing",
                "on_victory" => "2",
                "on_defeat" => "death",
                "enemies" => Field::List(vec![Field::Dict(row! {
                    "name" => "collecteur",
                    "force" => 6_i64,
                    "vie_max" => 400_i64,
                    "vie_actuelle" => 400_i64,
                })]),
                "assaults" => Field::List(Vec::new()),
            }),
        );
        db.table("games").seed(vec![game]).await;

        client.post("/combat/1/resolve").await;

        assert_ne!(stored(&db).await["combat"], Field::Null);
    }

    #[tokio::test]
    async fn resolving_without_a_hero_goes_back_to_the_page() {
        let (_db, client) = fighting_client().await;

        let response = client.post("/combat/1/resolve").await;

        assert_eq!(response.location(), "/book/1");
    }

    #[tokio::test]
    async fn the_death_page_offers_a_new_hero() {
        let (_db, client) = hero().await;

        let body = client.get("/game/death").await.text();

        assert!(body.contains("Reprendre"));
        assert!(body.contains(r#"value="normal""#));
        assert!(body.contains(r#"value="easy""#));
    }
}

/// Pages whose text adds a rule the engine does not model.
mod special_rules {
    use super::*;

    #[tokio::test]
    async fn a_flagged_page_warns_the_reader() {
        let (db, client) = hero().await;
        let mut game = stored(&db).await;
        game.insert(
            "combat".to_string(),
            combat_blob(row! {
                "page" => "1",
                "fight_type" => "simultaneous",
                "status" => "ongoing",
                "has_special_rules" => true,
                "enemies" => Field::List(vec![Field::Dict(row! {
                    "name" => "lepreux",
                    "force" => 6_i64,
                    "vie_max" => 6_i64,
                    "vie_actuelle" => 6_i64,
                })]),
                "assaults" => Field::List(Vec::new()),
            }),
        );
        db.table("games").seed(vec![game]).await;

        assert!(client
            .get("/combat/1")
            .await
            .text()
            .contains("règle particulière"));
    }

    #[tokio::test]
    async fn an_ordinary_page_does_not() {
        let (_db, client) = hero().await;

        assert!(!client
            .get("/combat/1")
            .await
            .text()
            .contains("règle particulière"));
    }
}
