//! Heroes who did not come back: how they are remembered, and where.
//!
//! Ported from `tests/test_fallen_heroes.py`. The browser half is in
//! `test_memorial_page.rs`.

#[path = "common/mod.rs"]
mod common;

use common::{at, fake_db, fixed, seeded, FakeDb, BOOK};
use haute_tension::application::models::game::GameDict;
use haute_tension::core::character::{EASY, NORMAL};
use haute_tension::core::db::{
    apply_pending, begin_combat, fallen_heroes, find_game, follow_choice, play_assault, start_game,
    MAX_FALLEN_HEROES,
};
use haute_tension::core::models::hybrid_document::Field;
use serde_json::{Map, Value};

const OTHER_BOOK: &str = "pretre_jean/autre_livre";

/// One choice or fight, as the book stores it.
fn written(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn deadly_fight() -> Map<String, Value> {
    written(serde_json::json!({
        "fight_type": "single",
        "enemies": [{"name": "collecteur d'impots", "force": 60, "vie": 10}],
        "outcome": {"on_victory": "621", "on_defeat": "death"},
    }))
}

fn fatal_choice() -> Map<String, Value> {
    written(serde_json::json!({
        "goto": "2",
        "gains": [],
        "losses": [{"element": "life point", "label_fr": "point de Vie", "amount": 99}],
    }))
}

/// A hero rolled at the bottom of the range, easy to kill.
async fn weak_hero(db: &FakeDb) -> GameDict {
    start_game(
        &db.db(),
        BOOK,
        NORMAL,
        Some(&fixed(&[1, 1, 1, 1, 1, 1, 1, 1])),
    )
    .await
    .expect("the database answers")
}

/// The one game, as it now stands.
async fn reload(db: &FakeDb, id: i64) -> GameDict {
    find_game(&db.db(), Some(id))
        .await
        .expect("the database answers")
        .expect("the game is there")
}

/// Fight until the hero falls.
async fn fight_to_the_death(db: &FakeDb, id: i64) {
    begin_combat(&db.db(), Some(id), "22", &deadly_fight())
        .await
        .expect("the database answers");
    while reload(db, id).await.vie_actuelle > 0 {
        play_assault(&db.db(), Some(id), Some(&seeded(3)))
            .await
            .expect("the database answers");
    }
}

/// The stone is cut when Vie reaches zero.
mod falling_in_combat {
    use super::*;

    #[tokio::test]
    async fn a_hero_who_loses_is_laid_to_rest() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        fight_to_the_death(&db, game.id).await;

        assert!(reload(&db, game.id).await.is_dead);
    }

    #[tokio::test]
    async fn he_is_remembered_where_he_fell() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        fight_to_the_death(&db, game.id).await;

        assert_eq!(
            reload(&db, game.id).await.died_on_page.as_deref(),
            Some("22")
        );
    }

    #[tokio::test]
    async fn what_killed_him_is_named() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        fight_to_the_death(&db, game.id).await;

        assert_eq!(
            reload(&db, game.id).await.died_of.as_deref(),
            Some("collecteur d'impots")
        );
    }

    #[tokio::test]
    async fn a_living_hero_is_not_among_them() {
        let db = fake_db().await;
        start_game(&db.db(), BOOK, NORMAL, Some(&seeded(1)))
            .await
            .expect("the database answers");

        assert!(fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers")
            .is_empty());
    }
}

/// A hero can die of a paragraph as well as of an adversary.
mod falling_on_the_road {
    use super::*;

    #[tokio::test]
    async fn a_fatal_loss_lays_him_to_rest() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;

        let after = follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert_eq!(after.vie_actuelle, 0);
        assert!(after.is_dead);
        assert_eq!(after.died_on_page.as_deref(), Some("63"));
    }

    #[tokio::test]
    async fn the_cause_says_it_was_the_road() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers");

        assert_eq!(
            reload(&db, game.id).await.died_of.as_deref(),
            Some("les épreuves du chemin")
        );
    }

    #[tokio::test]
    async fn a_change_the_reader_applies_can_kill_too() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        let fatal = written(serde_json::json!({
            "goto": "2",
            "gains": [],
            "losses": [{
                "element": "life point",
                "label_fr": "point de Vie",
                "amount": 99,
                "condition": "Si vous n'avez pas mangé",
            }],
        }));
        follow_choice(&db.db(), Some(game.id), "63", &fatal)
            .await
            .expect("the database answers");
        assert!(!reload(&db, game.id).await.is_dead);

        apply_pending(&db.db(), Some(game.id), 0)
            .await
            .expect("the database answers");

        assert!(reload(&db, game.id).await.is_dead);
    }

    /// Nothing afterwards should move the date on the stone.
    #[tokio::test]
    async fn the_first_death_is_the_one_that_counts() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers");
        let first = reload(&db, game.id).await;

        follow_choice(&db.db(), Some(game.id), "999", &fatal_choice())
            .await
            .expect("the database answers");

        assert_eq!(reload(&db, game.id).await.died_on_page, first.died_on_page);
        assert_eq!(first.died_on_page.as_deref(), Some("63"));
    }
}

/// What the list holds.
mod the_memorial {
    use super::*;

    #[tokio::test]
    async fn the_fallen_are_listed() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers");

        let epitaphs = fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers");

        assert_eq!(epitaphs.len(), 1);
        assert_eq!(epitaphs[0].id, game.id);
        assert_eq!(epitaphs[0].died_on_page.as_deref(), Some("63"));
        assert_eq!(epitaphs[0].force, game.force);
        assert_eq!(epitaphs[0].vie_max, game.vie_max);
    }

    #[tokio::test]
    async fn what_he_carried_is_remembered() {
        let db = fake_db().await;
        let game = weak_hero(&db).await;
        follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers");

        let epitaphs = fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers");

        let mut carried: Vec<&str> = epitaphs[0]
            .bag
            .iter()
            .map(|item| item.element.as_str())
            .collect();
        carried.sort_unstable();
        assert_eq!(carried, ["bag", "ration", "sword"]);
    }

    #[tokio::test]
    async fn his_difficulty_is_remembered() {
        let db = fake_db().await;
        let game = start_game(&db.db(), BOOK, EASY, Some(&seeded(1)))
            .await
            .expect("the database answers");
        follow_choice(&db.db(), Some(game.id), "63", &fatal_choice())
            .await
            .expect("the database answers");

        assert_eq!(
            fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
                .await
                .expect("the database answers")[0]
                .mode_label,
            "Facile"
        );
    }

    /// The times are set here: two deaths a millisecond apart have no order.
    #[tokio::test]
    async fn the_most_recent_comes_first() {
        let db = fake_db().await;
        let first = weak_hero(&db).await;
        follow_choice(&db.db(), Some(first.id), "10", &fatal_choice())
            .await
            .expect("the database answers");
        let second = weak_hero(&db).await;
        follow_choice(&db.db(), Some(second.id), "20", &fatal_choice())
            .await
            .expect("the database answers");

        let buried: Vec<_> = db
            .table("games")
            .docs()
            .await
            .into_iter()
            .map(|mut doc| {
                let when = if doc["died_on_page"] == Field::Str("10".to_string()) {
                    at(2026, 9, 10, 12, 0)
                } else {
                    at(2026, 9, 10, 13, 0)
                };
                doc.insert("died_at".to_string(), Field::Instant(when));
                doc
            })
            .collect();
        db.table("games").seed(buried).await;

        let pages: Vec<Option<String>> = fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers")
            .iter()
            .map(|epitaph| epitaph.died_on_page.clone())
            .collect();
        assert_eq!(pages, [Some("20".to_string()), Some("10".to_string())]);
    }

    #[tokio::test]
    async fn only_this_book_is_remembered() {
        let db = fake_db().await;
        let here = weak_hero(&db).await;
        follow_choice(&db.db(), Some(here.id), "10", &fatal_choice())
            .await
            .expect("the database answers");
        let elsewhere = start_game(&db.db(), OTHER_BOOK, NORMAL, Some(&seeded(1)))
            .await
            .expect("the database answers");
        follow_choice(&db.db(), Some(elsewhere.id), "10", &fatal_choice())
            .await
            .expect("the database answers");

        let ids: Vec<i64> = fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers")
            .iter()
            .map(|epitaph| epitaph.id)
            .collect();
        assert_eq!(ids, [here.id]);
    }

    #[tokio::test]
    async fn the_list_is_bounded() {
        let db = fake_db().await;
        for number in 0..4 {
            let game = weak_hero(&db).await;
            follow_choice(&db.db(), Some(game.id), &number.to_string(), &fatal_choice())
                .await
                .expect("the database answers");
        }

        assert_eq!(
            fallen_heroes(&db.db(), BOOK, 2)
                .await
                .expect("the database answers")
                .len(),
            2
        );
    }

    /// Being left behind is not dying.
    #[tokio::test]
    async fn an_abandoned_hero_in_good_health_is_not_among_them() {
        let db = fake_db().await;
        start_game(&db.db(), BOOK, NORMAL, Some(&seeded(1)))
            .await
            .expect("the database answers");
        start_game(&db.db(), BOOK, NORMAL, Some(&seeded(2)))
            .await
            .expect("the database answers");

        assert!(fallen_heroes(&db.db(), BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the database answers")
            .is_empty());
    }
}
