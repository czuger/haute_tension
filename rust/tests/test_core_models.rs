//! The hybrid pattern every table follows: columns, child rows, one JSON blob.
//!
//! Ported from `tests/test_core_models.py`.

#[path = "common/mod.rs"]
mod common;

use chrono::{Duration, FixedOffset, NaiveDate, Utc};
use common::{at, execute, fake_db, scalar, stored, FakeDb, BOOK};
use haute_tension::core::db::schema::TABLES;
use haute_tension::core::models::game::Game;
use haute_tension::core::models::hybrid_document::{
    json_default, Field, HybridDocument, RowDict,
};
use haute_tension::core::models::item::Item;
use haute_tension::core::models::page_inspection::PageInspection;
use haute_tension::core::models::page_view::PageView;
use haute_tension::core::models::utc_datetime::to_text;
use haute_tension::row;
use serde_json::Value;

/// The four stats a game keeps in columns of their own.
const STATS: [&str; 4] = ["force", "vie_max", "vie_actuelle", "gold"];

/// The instant every test that needs one uses.
fn noon() -> chrono::DateTime<Utc> {
    at(2026, 9, 10, 12, 0)
}

/// A hero, as a flat dict.
fn hero() -> RowDict {
    row! {
        "book" => BOOK,
        "force" => 12_i64,
        "vie_max" => 20_i64,
        "vie_actuelle" => 20_i64,
        "gold" => 5_i64,
    }
}

/// One line of a bag, as a flat dict.
fn sword() -> RowDict {
    row! { "element" => "sword", "label" => "épée", "count" => 1_i64 }
}

/// Another line of a bag.
fn key() -> RowDict {
    row! { "element" => "key", "label" => "clé", "count" => 2_i64 }
}

/// A hero with more written onto him.
fn hero_with(extra: RowDict) -> RowDict {
    let mut values = hero();
    values.extend(extra);
    values
}

/// A copy of a dict with one key left out.
fn without(values: &RowDict, key: &str) -> RowDict {
    values
        .iter()
        .filter(|(name, _)| name.as_str() != key)
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// What an item row says, without its id and its dates.
fn bag_line(item: &RowDict) -> RowDict {
    ["element", "label", "count"]
        .iter()
        .filter_map(|name| {
            item.get(*name)
                .map(|value| ((*name).to_string(), value.clone()))
        })
        .collect()
}

/// The one game a test seeded, loaded, changed and written back.
async fn rewrite_the_bag(db: &FakeDb, lines: Vec<RowDict>) -> Vec<RowDict> {
    let mut game = db.table("games").one().await;
    game.insert(
        "items".to_string(),
        Field::List(lines.into_iter().map(Field::Dict).collect()),
    );
    db.table("games").seed(vec![game]).await;
    db.table("items").docs().await
}

/// Which fields are real columns, and which live in the blob.
mod indexed_columns {
    use super::*;

    #[test]
    fn a_game_indexes_what_is_queried_or_constrained() {
        assert_eq!(
            Game::indexed_columns(),
            [
                "id",
                "book",
                "force",
                "vie_max",
                "vie_actuelle",
                "gold",
                "died_at",
                "created_at",
                "updated_at",
            ]
        );
    }

    #[test]
    fn an_item_indexes_its_game() {
        assert_eq!(
            Item::indexed_columns(),
            ["id", "game_id", "created_at", "updated_at"]
        );
    }

    #[test]
    fn a_visit_indexes_what_the_history_sorts_on() {
        assert_eq!(
            PageView::indexed_columns(),
            ["id", "book", "game_id", "viewed_at"]
        );
    }

    #[test]
    fn a_flagged_page_indexes_what_the_list_filters_on() {
        assert_eq!(
            PageInspection::indexed_columns(),
            ["id", "book", "path", "status", "created_at", "updated_at"]
        );
    }

    #[test]
    fn the_blob_itself_is_not_listed() {
        assert!(!Game::indexed_columns().contains(&"data"));
        assert!(!Item::indexed_columns().contains(&"data"));
        assert!(!PageView::indexed_columns().contains(&"data"));
        assert!(!PageInspection::indexed_columns().contains(&"data"));
    }
}

/// Which rows travel inside another row's dict.
mod child_tables {
    use super::*;

    #[test]
    fn a_game_carries_its_bag() {
        assert_eq!(Game::child_tables(), [("items", "items")]);
    }

    #[test]
    fn the_other_tables_carry_nothing() {
        assert!(Item::child_tables().is_empty());
        assert!(PageView::child_tables().is_empty());
        assert!(PageInspection::child_tables().is_empty());
    }
}

/// Sorting one flat dict into columns, children and blob.
mod from_dict {
    use super::*;

    #[test]
    fn a_column_key_becomes_the_column() {
        let game = Game::from_dict(&hero_with(row! {
            "id" => 1_i64,
            "died_at" => noon(),
        }));

        assert_eq!(game.column("id"), Field::Int(1));
        assert_eq!(game.column("book"), Field::Str(BOOK.to_string()));
        assert_eq!(game.column("force"), Field::Int(12));
        assert_eq!(game.column("gold"), Field::Int(5));
        assert_eq!(game.column("died_at"), Field::Instant(noon()));
    }

    #[test]
    fn every_other_key_goes_in_the_blob() {
        let game = Game::from_dict(&hero_with(row! {
            "mode" => "easy",
            "pending" => Field::List(Vec::new()),
        }));

        assert_eq!(
            *game.data(),
            row! { "mode" => "easy", "pending" => Field::List(Vec::new()) }
        );
    }

    #[test]
    fn a_relationship_key_becomes_child_rows() {
        let game = Game::from_dict(&hero_with(row! {
            "items" => Field::List(vec![Field::Dict(sword()), Field::Dict(key())]),
        }));

        let blobs: Vec<&RowDict> = game.items.iter().map(HybridDocument::data).collect();
        assert_eq!(blobs, [&sword(), &key()]);
        assert!(!game.data().contains_key("items"));
    }

    #[test]
    fn nothing_but_columns_is_an_empty_blob() {
        assert!(Game::from_dict(&hero()).data().is_empty());
    }

    #[test]
    fn an_instant_in_the_blob_is_text_at_once() {
        let view = PageView::from_dict(&row! {
            "book" => BOOK,
            "viewed_at" => noon(),
            "seen" => noon(),
        });

        assert_eq!(
            *view.data(),
            row! { "seen" => "2026-09-10T12:00:00+00:00" }
        );
    }

    /// The blob is built from what it was given rather than sharing it.
    #[test]
    fn the_blob_does_not_share_what_it_was_given() {
        let mut pending = vec![Field::Dict(row! { "element" => "key" })];
        let game = Game::from_dict(&hero_with(row! {
            "pending" => Field::List(pending.clone()),
        }));

        pending.push(Field::Str("nothing".to_string()));

        assert_eq!(
            game.data()["pending"],
            Field::List(vec![Field::Dict(row! { "element" => "key" })])
        );
    }
}

/// Reading one flat dict back.
mod to_dict {
    use super::*;

    #[test]
    fn columns_children_and_blob_are_one_dict() {
        let game = Game::from_dict(&hero_with(row! {
            "id" => 1_i64,
            "mode" => "easy",
            "items" => Field::List(vec![Field::Dict(sword())]),
        }));

        let mut expected = row! { "id" => 1_i64 };
        expected.extend(hero());
        expected.insert("died_at".to_string(), Field::Null);
        expected.insert("created_at".to_string(), Field::Null);
        expected.insert("updated_at".to_string(), Field::Null);
        let mut line = row! {
            "id" => Field::Null,
            "game_id" => Field::Null,
            "created_at" => Field::Null,
            "updated_at" => Field::Null,
        };
        line.extend(sword());
        expected.insert("items".to_string(), Field::List(vec![Field::Dict(line)]));
        expected.insert("mode".to_string(), Field::Str("easy".to_string()));

        assert_eq!(game.to_dict(), expected);
    }

    #[test]
    fn columns_come_first_then_children_then_the_blob() {
        let mut values = row! { "mode" => "easy", "items" => Field::List(Vec::new()) };
        values.extend(hero());
        values.insert("id".to_string(), Field::Int(1));
        let game = Game::from_dict(&values);

        let written = game.to_dict();
        let keys: Vec<&str> = written.keys().map(String::as_str).collect();
        let mut expected: Vec<&str> = Game::indexed_columns().to_vec();
        expected.extend(["items", "mode"]);
        assert_eq!(keys, expected);
    }

    /// The dict is built each time rather than handed out.
    #[test]
    fn the_dict_is_a_copy() {
        let game = Game::from_dict(&hero_with(row! {
            "pending" => Field::List(vec![Field::Dict(row! { "count" => 1_i64 })]),
        }));

        let mut taken = game.to_dict();
        taken.insert("pending".to_string(), Field::Str("nothing".to_string()));

        assert_eq!(
            game.to_dict()["pending"],
            Field::List(vec![Field::Dict(row! { "count" => 1_i64 })])
        );
    }

    #[test]
    fn an_unset_blob_reads_as_the_columns_alone() {
        let view = PageView::from_dict(&row! { "id" => 1_i64, "book" => BOOK });

        assert_eq!(
            view.to_dict(),
            row! {
                "id" => 1_i64,
                "book" => BOOK,
                "game_id" => Field::Null,
                "viewed_at" => Field::Null,
            }
        );
    }
}

/// Writing a changed dict back onto a row.
mod update_from_dict {
    use super::*;

    #[test]
    fn the_columns_are_set() {
        let mut game = Game::from_dict(&hero());

        game.update_from_dict(&hero_with(row! {
            "vie_actuelle" => 3_i64,
            "died_at" => noon(),
        }));

        assert_eq!(game.column("vie_actuelle"), Field::Int(3));
        assert_eq!(game.column("died_at"), Field::Instant(noon()));
    }

    #[test]
    fn the_blob_is_replaced_whole() {
        let mut game = Game::from_dict(&hero_with(row! {
            "mode" => "easy",
            "combat" => Field::Null,
        }));

        game.update_from_dict(&hero_with(row! { "mode" => "normal" }));

        assert_eq!(*game.data(), row! { "mode" => "normal" });
    }

    #[test]
    fn a_column_left_out_keeps_its_value() {
        let mut game = Game::from_dict(&hero_with(row! { "died_at" => noon() }));

        game.update_from_dict(&row! { "mode" => "easy" });

        assert_eq!(game.column("book"), Field::Str(BOOK.to_string()));
        assert_eq!(game.column("force"), Field::Int(12));
        assert_eq!(game.column("died_at"), Field::Instant(noon()));
    }

    #[test]
    fn children_left_out_are_kept() {
        let mut game = Game::from_dict(&hero_with(row! {
            "items" => Field::List(vec![Field::Dict(sword())]),
        }));

        game.update_from_dict(&hero());

        let blobs: Vec<&RowDict> = game.items.iter().map(HybridDocument::data).collect();
        assert_eq!(blobs, [&sword()]);
    }
}

/// A game's items, written and kept in step as rows of their own.
mod the_bag_as_rows {
    use super::*;

    #[tokio::test]
    async fn one_row_per_line_of_the_bag() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword()), Field::Dict(key())]),
            })])
            .await;

        let rows = db.table("items").docs().await;

        let read: Vec<(Field, RowDict)> = rows
            .iter()
            .map(|row| (row["game_id"].clone(), bag_line(row)))
            .collect();
        assert_eq!(
            read,
            [(Field::Int(1), sword()), (Field::Int(1), key())]
        );
    }

    #[tokio::test]
    async fn a_line_named_by_its_id_is_updated_in_place() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword())]),
            })])
            .await;
        let before = db.table("items").one().await;

        let mut changed = before.clone();
        changed.insert("count".to_string(), Field::Int(2));
        let rows = rewrite_the_bag(&db, vec![changed]).await;

        assert_eq!(rows[0]["id"], before["id"]);
        assert_eq!(rows[0]["count"], Field::Int(2));
        assert_eq!(rows[0]["created_at"], before["created_at"]);
        assert!(rows[0]["updated_at"] > before["updated_at"]);
    }

    #[tokio::test]
    async fn a_line_without_an_id_is_a_new_row() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword())]),
            })])
            .await;
        let before = db.table("items").one().await;

        let rows = rewrite_the_bag(&db, vec![before.clone(), key()]).await;

        let ids: Vec<i64> = rows
            .iter()
            .filter_map(|row| row["id"].as_int())
            .collect();
        let first = before["id"].as_int().expect("a stored row has an id");
        assert_eq!(ids, [first, first + 1]);
        assert_eq!(bag_line(&rows[1]), key());
    }

    #[tokio::test]
    async fn a_line_left_out_is_deleted() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword()), Field::Dict(key())]),
            })])
            .await;
        let stored_key = db.table("items").docs().await[1].clone();

        let rows = rewrite_the_bag(&db, vec![stored_key.clone()]).await;

        assert_eq!(rows, [stored_key]);
    }

    #[tokio::test]
    async fn the_bag_reads_in_the_order_it_was_filled() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(key()), Field::Dict(sword())]),
            })])
            .await;

        let game = db.table("games").one().await;

        let lines: Vec<RowDict> = game["items"]
            .as_list()
            .expect("a game carries its bag")
            .iter()
            .filter_map(Field::as_dict)
            .map(bag_line)
            .collect();
        assert_eq!(lines, [key(), sword()]);
    }

    #[tokio::test]
    async fn a_deleted_game_takes_its_bag_with_it() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword())]),
            })])
            .await;

        db.table("games").seed(Vec::new()).await;

        assert!(db.table("items").docs().await.is_empty());
    }

    #[tokio::test]
    async fn a_line_needs_a_game() {
        let db = fake_db().await;

        let mut line = row! { "game_id" => 99_i64 };
        line.extend(sword());
        let trouble = db
            .table("items")
            .set_docs(vec![line])
            .await
            .expect_err("a line without a game is refused");

        assert!(trouble.to_string().contains("FOREIGN KEY"));
    }
}

/// When a row was created, and when it last changed.
mod timestamps {
    use super::*;

    #[tokio::test]
    async fn a_new_row_was_last_changed_when_it_was_created() {
        let db = fake_db().await;
        db.table("games").seed(vec![hero_with(row! { "id" => 1_i64 })]).await;

        let game = db.table("games").one().await;

        assert_eq!(game["created_at"], game["updated_at"]);
        assert!(matches!(game["created_at"], Field::Instant(_)));
    }

    #[tokio::test]
    async fn a_change_moves_updated_at_and_nothing_else() {
        let db = fake_db().await;
        db.table("games").seed(vec![hero_with(row! { "id" => 1_i64 })]).await;
        let before = db.table("games").one().await;

        let mut changed = before.clone();
        changed.insert("vie_actuelle".to_string(), Field::Int(4));
        db.table("games").seed(vec![changed]).await;
        let after = db.table("games").one().await;

        assert_eq!(after["created_at"], before["created_at"]);
        assert!(after["updated_at"] > before["updated_at"]);
    }

    #[tokio::test]
    async fn writing_the_same_values_moves_nothing() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword())]),
            })])
            .await;
        let before = db.table("games").docs().await;

        db.table("games").seed(before.clone()).await;

        assert_eq!(db.table("games").docs().await, before);
    }

    #[tokio::test]
    async fn an_update_that_sets_updated_at_keeps_it() {
        let db = fake_db().await;
        db.table("games").seed(vec![hero_with(row! { "id" => 1_i64 })]).await;
        let game = db.table("games").one().await;

        let mut changed = game.clone();
        changed.insert("gold".to_string(), Field::Int(1));
        changed.insert("updated_at".to_string(), Field::Instant(noon()));
        db.table("games").seed(vec![changed]).await;

        assert_eq!(
            db.table("games").one().await["updated_at"],
            Field::Instant(noon())
        );
    }

    #[tokio::test]
    async fn a_line_changing_does_not_move_its_game() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "items" => Field::List(vec![Field::Dict(sword())]),
            })])
            .await;
        let before = db.table("games").one().await;

        let mut line = before["items"].as_list().expect("a bag")[0]
            .as_dict()
            .cloned()
            .expect("a line");
        line.insert("count".to_string(), Field::Int(3));
        let mut changed = before.clone();
        changed.insert("items".to_string(), Field::List(vec![Field::Dict(line)]));
        db.table("games").seed(vec![changed]).await;
        let after = db.table("games").one().await;

        assert_eq!(after["updated_at"], before["updated_at"]);
        let was = &before["items"].as_list().expect("a bag")[0]
            .as_dict()
            .expect("a line")["updated_at"];
        let now = &after["items"].as_list().expect("a bag")[0]
            .as_dict()
            .expect("a line")["updated_at"];
        assert!(now > was);
    }

    #[tokio::test]
    async fn a_row_inserted_by_hand_is_stamped_by_the_database() {
        let db = fake_db().await;

        execute(
            &db.connection,
            "INSERT INTO games (book, force, vie_max, vie_actuelle, gold, data) \
             VALUES ('livre', 1, 1, 1, 0, '{}')",
        )
        .await
        .expect("a row the database accepts");
        let created = scalar(&db.connection, "SELECT created_at FROM games")
            .await
            .expect("a stamped row");
        let updated = scalar(&db.connection, "SELECT updated_at FROM games")
            .await
            .expect("a stamped row");

        assert_eq!(created, updated);
        assert_eq!(created.len(), to_text(noon()).len());
        assert!(matches!(
            db.table("games").one().await["created_at"],
            Field::Instant(_)
        ));
    }
}

/// Force, Vie and gold: small numbers that never go below zero.
mod stat_columns {
    use super::*;

    #[tokio::test]
    async fn a_negative_value_is_refused() {
        for stat in STATS {
            let db = fake_db().await;
            let mut values = hero_with(row! { "id" => 1_i64 });
            values.insert(stat.to_string(), Field::Int(-1));

            let trouble = db
                .table("games")
                .set_docs(vec![values])
                .await
                .expect_err("a negative stat is refused");

            assert!(
                trouble
                    .to_string()
                    .contains(&format!("ck_games_{stat}_not_negative")),
                "for {stat}: {trouble}"
            );
        }
    }

    #[tokio::test]
    async fn a_missing_value_is_refused() {
        for stat in ["force", "vie_max", "vie_actuelle"] {
            let db = fake_db().await;
            let mut values = without(&hero(), stat);
            values.insert("id".to_string(), Field::Int(1));

            let trouble = db
                .table("games")
                .set_docs(vec![values])
                .await
                .expect_err("a missing stat is refused");

            assert!(
                trouble
                    .to_string()
                    .contains(&format!("NOT NULL constraint failed: games.{stat}")),
                "for {stat}: {trouble}"
            );
        }
    }

    #[tokio::test]
    async fn an_unset_purse_is_empty() {
        let db = fake_db().await;
        let mut values = without(&hero(), "gold");
        values.insert("id".to_string(), Field::Int(1));
        db.table("games").seed(vec![values]).await;

        assert_eq!(db.table("games").one().await["gold"], Field::Int(0));
    }

    #[tokio::test]
    async fn zero_is_allowed() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! {
                "id" => 1_i64,
                "vie_actuelle" => 0_i64,
                "gold" => 0_i64,
            })])
            .await;

        assert_eq!(
            db.table("games").one().await["vie_actuelle"],
            Field::Int(0)
        );
    }

    /// The schema is the contract: each stat is declared a small integer in it.
    #[test]
    fn each_is_a_small_integer() {
        let games = TABLES
            .iter()
            .find(|statement| statement.contains("CREATE TABLE games"))
            .expect("the games table is in the schema");

        for stat in STATS {
            assert!(
                games.contains(&format!("{stat} SMALLINT NOT NULL")),
                "for {stat}"
            );
        }
    }
}

/// Integers the database counts up, and never hands out twice.
mod ids {
    use super::*;

    #[tokio::test]
    async fn they_count_up_from_one() {
        let db = fake_db().await;
        db.table("games").seed(vec![hero(), hero()]).await;

        let ids: Vec<Field> = db
            .table("games")
            .docs()
            .await
            .iter()
            .map(|game| game["id"].clone())
            .collect();
        assert_eq!(ids, [Field::Int(1), Field::Int(2)]);
    }

    #[tokio::test]
    async fn an_id_is_never_reused() {
        let db = fake_db().await;
        db.table("games").seed(vec![hero(), hero()]).await;
        let first = db.table("games").docs().await[0].clone();

        db.table("games").seed(vec![first.clone()]).await;
        db.table("games").seed(vec![first, hero()]).await;

        let ids: Vec<Field> = db
            .table("games")
            .docs()
            .await
            .iter()
            .map(|game| game["id"].clone())
            .collect();
        assert_eq!(ids, [Field::Int(1), Field::Int(3)]);
    }
}

/// The row as text.
mod to_json {
    use super::*;

    #[test]
    fn it_is_the_dict_as_json() {
        let game = Game::from_dict(&hero_with(row! { "id" => 1_i64, "mode" => "easy" }));

        let written: Value =
            serde_json::from_str(&game.to_json()).expect("a row is always written");
        assert_eq!(written, Field::Dict(game.to_dict()).to_json());
    }

    #[test]
    fn a_column_instant_is_written_as_iso_8601() {
        let game = Game::from_dict(&hero_with(row! { "died_at" => noon() }));

        let written: Value =
            serde_json::from_str(&game.to_json()).expect("a row is always written");
        assert_eq!(written["died_at"], "2026-09-10T12:00:00+00:00");
    }

    #[test]
    fn the_bag_is_written_too() {
        let game = Game::from_dict(&hero_with(row! {
            "items" => Field::List(vec![Field::Dict(sword())]),
        }));

        let written: Value =
            serde_json::from_str(&game.to_json()).expect("a row is always written");
        let line = written["items"][0]
            .as_object()
            .expect("one line of the bag")
            .clone();
        let read: RowDict = line
            .iter()
            .map(|(key, value)| (key.clone(), Field::from_json(value)))
            .collect();
        assert_eq!(bag_line(&read), sword());
    }

    #[test]
    fn accents_are_kept() {
        let game = Game::from_dict(&hero_with(row! { "died_of" => "épée" }));

        assert!(game.to_json().contains("épée"));
    }
}

/// What the blob does with what JSON cannot write.
mod the_json_hook {
    use super::*;

    #[test]
    fn a_datetime_is_its_iso_text() {
        assert_eq!(
            json_default(&Field::Instant(noon())).expect("an instant is written"),
            "2026-09-10T12:00:00+00:00"
        );
    }

    #[test]
    fn a_date_is_its_iso_text() {
        let day = NaiveDate::from_ymd_opt(2026, 9, 10).expect("a real day");

        assert_eq!(
            json_default(&Field::Date(day)).expect("a date is written"),
            "2026-09-10"
        );
    }

    #[test]
    fn anything_else_is_refused() {
        let trouble = json_default(&Field::List(Vec::new()))
            .expect_err("the hook only writes what JSON cannot");

        assert!(trouble.to_string().contains("list cannot be written"));
    }
}

/// The timestamp column, through a real database.
mod utc_datetime {
    use super::*;

    #[tokio::test]
    async fn an_aware_instant_comes_back_aware_and_equal() {
        let db = fake_db().await;
        db.table("page_views")
            .seed(vec![row! {
                "book" => BOOK,
                "page" => "1",
                "viewed_at" => noon(),
            }])
            .await;

        let view = db.table("page_views").one().await;

        assert_eq!(view["viewed_at"], Field::Instant(noon()));
    }

    #[tokio::test]
    async fn a_naive_instant_is_taken_to_be_utc() {
        let db = fake_db().await;
        db.table("page_views")
            .seed(vec![row! {
                "book" => BOOK,
                "page" => "1",
                "viewed_at" => "2026-09-10T12:00:00",
            }])
            .await;

        assert_eq!(
            db.table("page_views").one().await["viewed_at"],
            Field::Instant(noon())
        );
    }

    #[tokio::test]
    async fn another_timezone_is_stored_as_utc() {
        let db = fake_db().await;
        let paris = noon().with_timezone(
            &FixedOffset::east_opt(2 * 3600).expect("a real offset"),
        );
        db.table("page_views")
            .seed(vec![row! {
                "book" => BOOK,
                "page" => "1",
                "viewed_at" => paris.to_rfc3339(),
            }])
            .await;

        assert_eq!(
            db.table("page_views").one().await["viewed_at"],
            Field::Instant(noon())
        );
    }

    #[test]
    fn the_text_sorts_in_time_order() {
        let mut instants = vec![
            noon() + Duration::microseconds(500_000),
            noon() + Duration::seconds(1),
            noon(),
            noon() - Duration::days(1),
        ];
        let mut by_text = instants.clone();
        instants.sort();
        by_text.sort_by_key(|instant| to_text(*instant));

        assert_eq!(instants, by_text);
    }

    #[test]
    fn the_text_is_fixed_width() {
        assert_eq!(to_text(noon()), "2026-09-10T12:00:00.000000+00:00");
        assert_eq!(stored(noon()), "2026-09-10T12:00:00.000000+00:00");
    }

    #[tokio::test]
    async fn nothing_stays_nothing() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! { "id" => 1_i64, "died_at" => Field::Null })])
            .await;

        assert_eq!(db.table("games").one().await["died_at"], Field::Null);
    }
}

/// What a row prints as, for a message about a run.
mod rows_print {
    use super::*;

    #[test]
    fn a_living_hero() {
        let game = Game::from_dict(&hero_with(row! {
            "mode" => "easy",
            "force" => 14_i64,
            "vie_max" => 27_i64,
            "vie_actuelle" => 20_i64,
        }));

        assert_eq!(game.to_string(), format!("{BOOK} (easy) — Force 14, Vie 20/27"));
    }

    #[test]
    fn a_dead_hero() {
        let game = Game::from_dict(&hero_with(row! {
            "mode" => "normal",
            "force" => 9_i64,
            "died_at" => noon(),
        }));

        assert_eq!(game.to_string(), format!("{BOOK} (normal) — Force 9, mort"));
        assert!(game.is_dead());
    }

    #[test]
    fn a_line_of_the_bag() {
        let ration = Item::from_dict(&row! {
            "element" => "ration",
            "label" => "ration",
            "count" => 4_i64,
        });

        assert_eq!(ration.to_string(), "ration ×4");
    }

    #[test]
    fn a_flagged_page_counts_its_comments() {
        let filed = PageInspection::from_dict(&row! {
            "book" => BOOK,
            "path" => "/book/2",
            "status" => "open",
            "comments" => Field::List(vec![
                Field::Dict(RowDict::new()),
                Field::Dict(RowDict::new()),
            ]),
        });

        assert_eq!(filed.to_string(), format!("{BOOK} /book/2 (open, 2 comments)"));
    }
}

/// A row written and read back is the same flat dict.
mod through_the_database {
    use super::*;

    #[tokio::test]
    async fn a_game_round_trips() {
        let db = fake_db().await;
        let written = hero_with(row! {
            "id" => 1_i64,
            "died_at" => noon(),
            "mode" => "normal",
            "combat" => Field::Null,
        });
        let mut seeded = written.clone();
        seeded.insert(
            "items".to_string(),
            Field::List(vec![Field::Dict(sword())]),
        );
        db.table("games").seed(vec![seeded]).await;

        let stored = db.table("games").one().await;

        for (key, value) in &written {
            assert_eq!(stored.get(key), Some(value), "for {key}");
        }
        let lines: Vec<RowDict> = stored["items"]
            .as_list()
            .expect("a game carries its bag")
            .iter()
            .filter_map(Field::as_dict)
            .map(bag_line)
            .collect();
        assert_eq!(lines, [sword()]);
    }

    #[tokio::test]
    async fn the_blob_is_json_text_with_its_accents() {
        let db = fake_db().await;
        db.table("games")
            .seed(vec![hero_with(row! { "id" => 1_i64, "died_of" => "épée" })])
            .await;

        let stored = scalar(&db.connection, "SELECT data FROM games")
            .await
            .expect("the blob is text");

        assert_eq!(
            serde_json::from_str::<Value>(&stored).expect("the blob is JSON"),
            serde_json::json!({"died_of": "épée"})
        );
        assert!(stored.contains("épée"));
    }
}
