//! Tests for reading and writing a play-through.

#[path = "common/mod.rs"]
mod common;

use common::{fake_db, fixed, seeded, FakeDb, BOOK};
use haute_tension::core::character::{EASY, NORMAL};
use haute_tension::core::combat::{DEFEAT, ONGOING, VICTORY};
use haute_tension::core::db::{
    apply_pending, begin_combat, dismiss_pending, end_combat, find_game, follow_choice,
    play_assault, start_game,
};
use haute_tension::core::models::game::Game;
use haute_tension::core::models::hybrid_document::{Field, HybridDocument};
use haute_tension::row;
use serde_json::{Map, Value};

/// One fight, as the book stores it.
fn fight(written: Value) -> Map<String, Value> {
    written.as_object().cloned().unwrap_or_default()
}

fn single_fight() -> Map<String, Value> {
    fight(serde_json::json!({
        "fight_type": "single",
        "enemies": [{"name": "collecteur", "force": 6, "vie": 10}],
        "outcome": {"on_victory": "621", "on_defeat": "death", "on_flee": null},
    }))
}

fn melee_fight() -> Map<String, Value> {
    fight(serde_json::json!({
        "fight_type": "simultaneous",
        "enemies": [{"name": "lepreux", "force": 6, "vie": 6, "count": 2}],
        "outcome": {"on_victory": "10", "on_defeat": "death", "on_flee": null},
    }))
}

const GOLD_FIGHT_PAGE: &str = "39";

fn costly_choice() -> Map<String, Value> {
    fight(serde_json::json!({
        "goto": "40",
        "gains": [{"element": "meal", "label_fr": "repas", "amount": 1}],
        "losses": [{"element": "gold coin", "label_fr": "pièce d'or", "amount": 5}],
    }))
}

fn conditional_gain() -> Value {
    serde_json::json!({
        "element": "strength point",
        "label_fr": "point de Force",
        "amount": 1,
        "condition": "pendant tout le temps où vous les porterez",
    })
}

fn conditional_choice() -> Map<String, Value> {
    fight(serde_json::json!({
        "goto": "41",
        "gains": [conditional_gain()],
        "losses": [],
    }))
}

/// A game rolled up for a test that does not care how.
async fn a_game(db: &FakeDb) -> haute_tension::application::models::game::GameDict {
    start_game(&db.db(), BOOK, NORMAL, Some(&seeded(1)))
        .await
        .expect("the database answers")
}

/// Rolling up a hero and opening a game for him.
mod starting_a_game {
    use super::*;

    #[tokio::test]
    async fn the_hero_is_rolled_and_stored() {
        let db = fake_db().await;

        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[3, 4, 5, 2, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");

        assert_eq!(game.force, 13);
        assert_eq!(game.vie_max, 25);
        assert_eq!(game.vie_actuelle, 25);
        assert_eq!(game.force_dice, [3, 4]);
        assert_eq!(game.vie_dice, [5, 2]);
    }

    #[tokio::test]
    async fn the_mode_is_the_book_by_default() {
        let db = fake_db().await;

        let game = a_game(&db).await;

        assert_eq!(game.mode, "normal");
        assert_eq!(game.force_throw, "2D6");
        assert_eq!(game.force_base, 6);
    }

    #[tokio::test]
    async fn an_easy_hero_is_rolled_the_easy_way() {
        let db = fake_db().await;

        let game = start_game(
            &db.db(),
            BOOK,
            EASY,
            Some(&fixed(&[3, 4, 2, 3, 1, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");

        assert_eq!(game.mode, "easy");
        assert_eq!(game.force, 12 + 7);
        assert_eq!(game.force_dice, [3, 4]);
        assert_eq!(game.vie_dice, [2, 3, 1]);
        assert_eq!(game.vie_max, 26 + 6);
    }

    #[tokio::test]
    async fn the_easy_sheet_says_which_dice_made_it() {
        let db = fake_db().await;

        let game = start_game(&db.db(), BOOK, EASY, Some(&seeded(1)))
            .await
            .expect("the database answers");

        assert_eq!(game.mode_label, "Facile");
        assert_eq!(game.force_throw, "2D4");
        assert_eq!(game.vie_throw, "3D4");
        assert_eq!((game.force_base, game.vie_base), (12, 26));
    }

    #[tokio::test]
    async fn the_mode_survives_a_reload() {
        let db = fake_db().await;
        let created = start_game(&db.db(), BOOK, EASY, Some(&seeded(1)))
            .await
            .expect("the database answers");

        let reloaded = find_game(&db.db(), Some(created.id))
            .await
            .expect("the database answers")
            .expect("the game was just written");

        assert_eq!(reloaded.mode, "easy");
        assert_eq!(reloaded, created);
    }

    /// Its floor is above what the book's worst roll can give.
    #[tokio::test]
    async fn an_easy_hero_is_stronger_than_the_book_allows() {
        let db = fake_db().await;
        let mut easy = Vec::new();
        let mut normal = Vec::new();
        for seed in 0..40 {
            easy.push(
                start_game(&db.db(), BOOK, EASY, Some(&seeded(seed)))
                    .await
                    .expect("the database answers")
                    .force,
            );
            normal.push(
                start_game(&db.db(), BOOK, NORMAL, Some(&seeded(seed)))
                    .await
                    .expect("the database answers")
                    .force,
            );
        }

        assert!(easy.iter().min() > normal.iter().min());
    }

    #[tokio::test]
    async fn a_new_game_is_not_in_a_fight() {
        let db = fake_db().await;

        assert_eq!(a_game(&db).await.combat, None);
    }

    #[tokio::test]
    async fn the_adjustment_is_derived_from_the_force() {
        let db = fake_db().await;

        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[6, 6, 1, 1, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");

        assert_eq!(game.damage_adjustment, 2);
    }

    #[tokio::test]
    async fn each_game_gets_its_own_id() {
        let db = fake_db().await;

        let first = a_game(&db).await;
        let second = a_game(&db).await;

        assert_ne!(first.id, second.id);
    }

    #[tokio::test]
    async fn it_is_written_to_the_database() {
        let db = fake_db().await;
        a_game(&db).await;

        assert_eq!(db.table("games").docs().await.len(), 1);
    }
}

/// Loading one back.
mod finding_a_game {
    use super::*;

    #[tokio::test]
    async fn a_game_is_loaded_as_it_was_written() {
        let db = fake_db().await;
        let created = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[3, 4, 5, 2, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");

        assert_eq!(
            find_game(&db.db(), Some(created.id))
                .await
                .expect("the database answers"),
            Some(created)
        );
    }

    /// The dice are thrown once, at creation, and never again.
    #[tokio::test]
    async fn reopening_never_re_rolls_the_hero() {
        let db = fake_db().await;
        let created = a_game(&db).await;

        for _ in 0..5 {
            let reloaded = find_game(&db.db(), Some(created.id))
                .await
                .expect("the database answers")
                .expect("the game was just written");
            assert_eq!(reloaded.force, created.force);
            assert_eq!(reloaded.vie_max, created.vie_max);
        }
    }

    #[tokio::test]
    async fn an_unknown_id_finds_nothing() {
        let db = fake_db().await;

        assert_eq!(
            find_game(&db.db(), Some(404))
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn a_blank_id_finds_nothing() {
        let db = fake_db().await;

        assert_eq!(
            find_game(&db.db(), Some(0))
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            find_game(&db.db(), None)
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// Putting the hero into the fight a page holds.
mod beginning_a_combat {
    use super::*;

    #[tokio::test]
    async fn the_adversaries_are_fielded_at_full_vie() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let armed = begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed");
        let combat = armed.combat.expect("the fight is on");

        assert_eq!(combat.page, "22");
        assert_eq!(combat.status, ONGOING);
        assert_eq!(combat.enemies.len(), 1);
        let enemy = &combat.enemies[0];
        assert_eq!(enemy.name, "collecteur");
        assert_eq!(enemy.force, 6);
        assert_eq!(enemy.vie_max, 10);
        assert_eq!(enemy.vie_actuelle, 10);
        assert_eq!(enemy.damage_adjustment, 0);
    }

    #[tokio::test]
    async fn the_outcome_branches_are_kept() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let combat = begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert_eq!(combat.on_victory.as_deref(), Some("621"));
        assert_eq!(combat.on_defeat.as_deref(), Some("death"));
        assert_eq!(combat.on_flee, None);
    }

    #[tokio::test]
    async fn a_repeated_adversary_becomes_that_many_bodies() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let combat = begin_combat(&db.db(), Some(game.id), "74", &melee_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        let names: Vec<&str> = combat
            .enemies
            .iter()
            .map(|enemy| enemy.name.as_str())
            .collect();
        assert_eq!(names, ["lepreux 1", "lepreux 2"]);
        assert!(combat.enemies.iter().all(|enemy| enemy.vie_actuelle == 6));
    }

    #[tokio::test]
    async fn a_lone_adversary_is_not_numbered() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let combat = begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert_eq!(combat.enemies[0].name, "collecteur");
    }

    #[tokio::test]
    async fn a_damage_adjustment_is_carried_over() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let djinn = fight(serde_json::json!({
            "fight_type": "single",
            "enemies": [{"name": "djinn", "force": 16, "vie": 20, "damage_adjustment": 1}],
            "outcome": {},
        }));

        let combat = begin_combat(&db.db(), Some(game.id), "133", &djinn)
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert_eq!(combat.enemies[0].damage_adjustment, 1);
    }

    #[tokio::test]
    async fn a_page_with_special_rules_is_flagged() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let combat = begin_combat(&db.db(), Some(game.id), "74", &melee_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert!(combat.has_special_rules);
    }

    #[tokio::test]
    async fn an_ordinary_page_is_not() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let combat = begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert!(!combat.has_special_rules);
    }

    /// A reload must not hand back full-health adversaries.
    #[tokio::test]
    async fn re_entering_the_page_does_not_restart_the_fight() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");
        play_assault(&db.db(), Some(game.id), Some(&seeded(2)))
            .await
            .expect("the database answers");
        let wounded = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there")
            .combat
            .expect("the fight is on")
            .enemies[0]
            .vie_actuelle;

        let again = begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert_eq!(again.enemies[0].vie_actuelle, wounded);
        assert_eq!(again.assaults.len(), 1);
    }

    #[tokio::test]
    async fn another_page_starts_a_fresh_fight() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");
        play_assault(&db.db(), Some(game.id), Some(&seeded(2)))
            .await
            .expect("the database answers");

        let combat = begin_combat(&db.db(), Some(game.id), "74", &melee_fight())
            .await
            .expect("the database answers")
            .expect("the fight is armed")
            .combat
            .expect("the fight is on");

        assert_eq!(combat.page, "74");
        assert!(combat.assaults.is_empty());
    }

    #[tokio::test]
    async fn a_fight_with_no_adversary_arms_nothing() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let empty = fight(serde_json::json!({
            "fight_type": "single", "enemies": [], "outcome": {}
        }));

        assert_eq!(
            begin_combat(&db.db(), Some(game.id), "18", &empty)
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn a_malformed_adversary_is_dropped() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let unusable = fight(serde_json::json!({
            "fight_type": "single", "enemies": ["orc"], "outcome": {}
        }));

        assert_eq!(
            begin_combat(&db.db(), Some(game.id), "18", &unusable)
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn an_unknown_game_arms_nothing() {
        let db = fake_db().await;

        assert_eq!(
            begin_combat(&db.db(), Some(404), "22", &single_fight())
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// Advancing an open fight, one assault at a time.
mod playing_an_assault {
    use super::*;

    #[tokio::test]
    async fn the_assault_is_logged_with_its_dice() {
        let db = fake_db().await;
        // Force 14, Vie 26.
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[4, 4, 4, 4, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");

        let combat = play_assault(&db.db(), Some(game.id), Some(&fixed(&[3, 3, 1, 2])))
            .await
            .expect("the database answers")
            .expect("the fight is on")
            .combat
            .expect("the fight is on");

        let assault = &combat.assaults[0];
        assert_eq!(assault.number, 1);
        assert_eq!(assault.hero_dice, [3, 3]);
        assert_eq!(assault.hero_attack_force, 20);
        assert_eq!(assault.exchanges[0].enemy_dice, [1, 2]);
        assert_eq!(assault.exchanges[0].enemy_force, 6);
        assert_eq!(assault.exchanges[0].enemy_attack_force, 9);
        assert_eq!(assault.exchanges[0].damage, 11);
    }

    #[tokio::test]
    async fn the_wounds_are_persisted() {
        let db = fake_db().await;
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[4, 4, 4, 4, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");
        play_assault(&db.db(), Some(game.id), Some(&fixed(&[1, 1, 6, 6])))
            .await
            .expect("the database answers");

        let reloaded = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert!(reloaded.vie_actuelle < reloaded.vie_max);
    }

    #[tokio::test]
    async fn assaults_are_numbered_in_order() {
        let db = fake_db().await;
        // Vie enough that three assaults cannot decide it either way.
        let tough = fight(serde_json::json!({
            "fight_type": "single",
            "enemies": [{"name": "thalos", "force": 18, "vie": 400}],
            "outcome": {"on_victory": "149", "on_defeat": "death"},
        }));
        // Force 17, Vie 30.
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[6, 5, 6, 6, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "503", &tough)
            .await
            .expect("the database answers");
        for _ in 0..3 {
            play_assault(&db.db(), Some(game.id), Some(&fixed(&[4, 4, 3, 3])))
                .await
                .expect("the database answers");
        }

        let combat = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there")
            .combat
            .expect("the fight is on");

        let numbers: Vec<i64> = combat.assaults.iter().map(|assault| assault.number).collect();
        assert_eq!(numbers, [1, 2, 3]);
        assert_eq!(combat.status, ONGOING);
    }

    #[tokio::test]
    async fn a_won_fight_stops_accepting_assaults() {
        let db = fake_db().await;
        // Force 18, Vie 30.
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[6, 6, 6, 6, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");
        play_assault(&db.db(), Some(game.id), Some(&fixed(&[6, 6, 1, 2])))
            .await
            .expect("the database answers");

        let combat = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there")
            .combat
            .expect("the fight is on");
        assert_eq!(combat.status, VICTORY);
        assert_eq!(
            play_assault(&db.db(), Some(game.id), Some(&seeded(1)))
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn a_lost_fight_is_a_defeat() {
        let db = fake_db().await;
        // Force 8, Vie 20.
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[1, 1, 1, 1, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");

        let combat = play_assault(&db.db(), Some(game.id), Some(&fixed(&[3, 4, 1, 1])))
            .await
            .expect("the database answers")
            .expect("the fight is on")
            .combat
            .expect("the fight is on");

        assert_eq!(combat.status, DEFEAT);
        assert_eq!(
            find_game(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .vie_actuelle,
            0
        );
    }

    #[tokio::test]
    async fn a_game_not_in_a_fight_has_nothing_to_play() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        assert_eq!(
            play_assault(&db.db(), Some(game.id), Some(&seeded(1)))
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn an_unknown_game_has_nothing_to_play() {
        let db = fake_db().await;

        assert_eq!(
            play_assault(&db.db(), Some(404), Some(&seeded(1)))
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// Leaving a fight behind.
mod ending_a_combat {
    use super::*;

    #[tokio::test]
    async fn the_fight_is_cleared() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");

        assert_eq!(
            end_combat(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .combat,
            None
        );
        assert_eq!(
            find_game(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .combat,
            None
        );
    }

    /// Leaving the fight does not heal the hero.
    #[tokio::test]
    async fn the_wounds_it_cost_are_kept() {
        let db = fake_db().await;
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[1, 1, 4, 4, 1, 1, 1, 1])),
        )
        .await
        .expect("the database answers");
        begin_combat(&db.db(), Some(game.id), "22", &single_fight())
            .await
            .expect("the database answers");
        play_assault(&db.db(), Some(game.id), Some(&fixed(&[1, 1, 6, 5])))
            .await
            .expect("the database answers");
        let wounded = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there")
            .vie_actuelle;

        assert_eq!(
            end_combat(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .vie_actuelle,
            wounded
        );
    }

    #[tokio::test]
    async fn an_unknown_game_clears_nothing() {
        let db = fake_db().await;

        assert_eq!(
            end_combat(&db.db(), Some(404))
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// Corners the browser flow never reaches on its own.
mod the_game_row {
    use super::*;

    /// The three writers share the same guard as finding a game.
    #[tokio::test]
    async fn a_blank_id_loads_no_row() {
        let db = fake_db().await;

        assert_eq!(
            end_combat(&db.db(), None)
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            play_assault(&db.db(), None, Some(&seeded(1)))
                .await
                .expect("the database answers"),
            None
        );
    }

    #[test]
    fn a_game_prints_its_hero() {
        let game = Game::from_dict(&row! {
            "id" => 1_i64,
            "book" => BOOK,
            "mode" => "easy",
            "force" => 14_i64,
            "vie_max" => 27_i64,
            "vie_actuelle" => 20_i64,
        });

        assert_eq!(game.to_string(), format!("{BOOK} (easy) — Force 14, Vie 20/27"));
    }
}

/// What the hero sets out with.
mod the_starting_holdings {
    use super::*;

    #[tokio::test]
    async fn he_carries_a_sword_a_bag_and_four_rations() {
        let db = fake_db().await;

        let game = a_game(&db).await;

        let carried: Vec<(&str, i64)> = game
            .bag
            .iter()
            .map(|item| (item.element.as_str(), item.count))
            .collect();
        assert_eq!(carried, [("sword", 1), ("bag", 1), ("ration", 4)]);
    }

    #[tokio::test]
    async fn his_purse_is_two_throws_of_two_dice() {
        let db = fake_db().await;

        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[1, 1, 1, 1, 3, 4, 5, 2])),
        )
        .await
        .expect("the database answers");

        assert_eq!(game.gold_dice, [3, 4, 5, 2]);
        assert_eq!(game.gold, 14);
    }

    #[tokio::test]
    async fn the_purse_never_leaves_four_to_twenty_four() {
        let db = fake_db().await;
        let mut purses = Vec::new();
        for seed in 0..60 {
            purses.push(
                start_game(&db.db(), BOOK, NORMAL, Some(&seeded(seed)))
                    .await
                    .expect("the database answers")
                    .gold,
            );
        }

        assert!(purses.iter().min().copied().unwrap_or(0) >= 4);
        assert!(purses.iter().max().copied().unwrap_or(0) <= 24);
    }

    #[tokio::test]
    async fn he_owes_nothing_and_waits_on_nothing() {
        let db = fake_db().await;

        assert!(a_game(&db).await.pending.is_empty());
    }
}

/// What a choice costs and gives.
mod following_a_choice {
    use super::*;

    #[tokio::test]
    async fn an_unconditional_loss_is_paid() {
        let db = fake_db().await;
        let game = start_game(
            &db.db(),
            BOOK,
            NORMAL,
            Some(&fixed(&[1, 1, 1, 1, 6, 6, 6, 6])),
        )
        .await
        .expect("the database answers");
        assert_eq!(game.gold, 24);

        let after = follow_choice(&db.db(), Some(game.id), GOLD_FIGHT_PAGE, &costly_choice())
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert_eq!(after.gold, 19);
    }

    #[tokio::test]
    async fn an_unconditional_gain_is_taken() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let after = follow_choice(&db.db(), Some(game.id), GOLD_FIGHT_PAGE, &costly_choice())
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert!(after.bag.iter().any(|item| item.element == "meal"));
    }

    #[tokio::test]
    async fn it_is_written_to_the_database() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        follow_choice(&db.db(), Some(game.id), GOLD_FIGHT_PAGE, &costly_choice())
            .await
            .expect("the database answers");

        assert_eq!(
            find_game(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .gold,
            game.gold - 5
        );
    }

    #[tokio::test]
    async fn a_conditional_change_waits_for_the_reader() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let after = follow_choice(&db.db(), Some(game.id), "11", &conditional_choice())
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert_eq!(after.force, game.force);
        assert_eq!(after.pending.len(), 1);
        assert_eq!(after.pending[0].element, "strength point");
        assert_eq!(after.pending[0].page.as_deref(), Some("11"));
        assert!(after.pending[0]
            .condition
            .as_deref()
            .expect("a condition was written")
            .contains("porterez"));
    }

    #[tokio::test]
    async fn a_choice_carrying_nothing_changes_nothing() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let after = follow_choice(
            &db.db(),
            Some(game.id),
            "1",
            &fight(serde_json::json!({"goto": "2"})),
        )
        .await
        .expect("the database answers")
        .expect("the game is there");

        assert_eq!(after.gold, game.gold);
        assert_eq!(after.bag, game.bag);
        assert!(after.pending.is_empty());
    }

    /// A reader who walked past one condition is not asked about it forever.
    #[tokio::test]
    async fn following_again_replaces_what_was_waiting() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        follow_choice(&db.db(), Some(game.id), "11", &conditional_choice())
            .await
            .expect("the database answers");

        let after = follow_choice(
            &db.db(),
            Some(game.id),
            "1",
            &fight(serde_json::json!({"goto": "2"})),
        )
        .await
        .expect("the database answers")
        .expect("the game is there");

        assert!(after.pending.is_empty());
    }

    #[tokio::test]
    async fn an_unknown_game_follows_nothing() {
        let db = fake_db().await;

        assert_eq!(
            follow_choice(&db.db(), Some(404), "1", &costly_choice())
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// The reader's word on a condition only he can judge.
mod ruling_on_a_waiting_change {
    use super::*;

    #[tokio::test]
    async fn applying_it_moves_the_hero() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        follow_choice(&db.db(), Some(game.id), "11", &conditional_choice())
            .await
            .expect("the database answers");

        let after = apply_pending(&db.db(), Some(game.id), 0)
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert_eq!(after.force, game.force + 1);
        assert!(after.pending.is_empty());
    }

    #[tokio::test]
    async fn dismissing_it_leaves_the_hero_alone() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        follow_choice(&db.db(), Some(game.id), "11", &conditional_choice())
            .await
            .expect("the database answers");

        let after = dismiss_pending(&db.db(), Some(game.id), Some(0))
            .await
            .expect("the database answers")
            .expect("the game is there");

        assert_eq!(after.force, game.force);
        assert!(after.pending.is_empty());
    }

    #[tokio::test]
    async fn dismissing_everything_clears_the_lot() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let twice = fight(serde_json::json!({
            "goto": "12",
            "gains": [conditional_gain(), conditional_gain()],
            "losses": [],
        }));
        follow_choice(&db.db(), Some(game.id), "11", &twice)
            .await
            .expect("the database answers");

        assert_eq!(
            find_game(&db.db(), Some(game.id))
                .await
                .expect("the database answers")
                .expect("the game is there")
                .pending
                .len(),
            2
        );
        assert!(dismiss_pending(&db.db(), Some(game.id), None)
            .await
            .expect("the database answers")
            .expect("the game is there")
            .pending
            .is_empty());
    }

    #[tokio::test]
    async fn an_index_that_names_nothing_does_nothing() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        assert_eq!(
            apply_pending(&db.db(), Some(game.id), 0)
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            dismiss_pending(&db.db(), Some(game.id), Some(3))
                .await
                .expect("the database answers"),
            None
        );
    }

    #[tokio::test]
    async fn an_unknown_game_rules_on_nothing() {
        let db = fake_db().await;

        assert_eq!(
            apply_pending(&db.db(), Some(404), 0)
                .await
                .expect("the database answers"),
            None
        );
        assert_eq!(
            dismiss_pending(&db.db(), Some(404), None)
                .await
                .expect("the database answers"),
            None
        );
    }
}

/// The hero's bag, kept as one row per line through a play-through.
mod the_bag_as_rows {
    use super::*;

    #[tokio::test]
    async fn a_new_hero_carries_three_lines() {
        let db = fake_db().await;
        let game = a_game(&db).await;

        let rows = db.table("items").docs().await;

        let elements: Vec<String> = rows
            .iter()
            .map(|row| row["element"].to_string())
            .collect();
        assert_eq!(elements, ["sword", "bag", "ration"]);
        assert!(rows
            .iter()
            .all(|row| row["game_id"] == Field::Int(game.id)));
    }

    #[tokio::test]
    async fn a_line_still_in_the_bag_stays_the_row_it_was() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let before: Vec<(String, i64)> = db
            .table("items")
            .docs()
            .await
            .iter()
            .map(|row| {
                (
                    row["element"].to_string(),
                    row["id"].as_int().expect("a stored row has an id"),
                )
            })
            .collect();

        follow_choice(&db.db(), Some(game.id), GOLD_FIGHT_PAGE, &costly_choice())
            .await
            .expect("the database answers");
        let after: Vec<(String, i64)> = db
            .table("items")
            .docs()
            .await
            .iter()
            .map(|row| {
                (
                    row["element"].to_string(),
                    row["id"].as_int().expect("a stored row has an id"),
                )
            })
            .collect();

        for line in &before {
            assert!(after.contains(line), "{line:?} kept its row");
        }
        let highest = before.iter().map(|(_, id)| *id).max().unwrap_or(0);
        let meal = after
            .iter()
            .find(|(element, _)| element == "meal")
            .expect("the meal was picked up");
        assert!(meal.1 > highest);
    }

    #[tokio::test]
    async fn a_line_used_up_is_deleted() {
        let db = fake_db().await;
        let game = a_game(&db).await;
        let eat_everything = fight(serde_json::json!({
            "goto": "2",
            "gains": [],
            "losses": [{"element": "ration", "label_fr": "ration", "amount": 4}],
        }));

        follow_choice(&db.db(), Some(game.id), "12", &eat_everything)
            .await
            .expect("the database answers");

        let elements: Vec<String> = db
            .table("items")
            .docs()
            .await
            .iter()
            .map(|row| row["element"].to_string())
            .collect();
        assert_eq!(elements, ["sword", "bag"]);
        let carried: Vec<String> = find_game(&db.db(), Some(game.id))
            .await
            .expect("the database answers")
            .expect("the game is there")
            .bag
            .iter()
            .map(|item| item.element.clone())
            .collect();
        assert_eq!(carried, ["sword", "bag"]);
    }
}
