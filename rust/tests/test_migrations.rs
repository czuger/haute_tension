//! The scripts in `migrations/`, applied to a file the previous schema wrote.
//!
//! Ported from `tests/test_migrations.py`. Every test starts from a version-0
//! database laid out the way the code before migration 001 wrote one — uuid ids,
//! the hero and his bag inside a JSON blob, instants written by `isoformat()` —
//! applies every script in order, and reads the result back: row by row with
//! SQLite, and through `core::db` the way the application will.
//!
//! The scripts themselves are unchanged: they are SQL, and SQL is what the
//! migration is.

#[path = "common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};

use common::{books_path, client_for, seeded, BOOK};
use haute_tension::application::models::game::SESSION_KEY;
use haute_tension::core::db::{
    connection::open_engine, create_the_schema, fallen_heroes, find_inspection, flagged_pages,
    follow_choice, last_page_read, last_pages, start_game, Db, MAX_FALLEN_HEROES,
    MAX_PAGE_HISTORY, SCHEMA_VERSION,
};
use haute_tension::core::character::NORMAL;
use rusqlite::Connection;
use serde_json::{json, Map, Value};

const FILE: &str = "haute_tension_dev.sqlite3";
const OTHER_BOOK: &str = "pretre_jean/autre_livre";

/// What the models at commit 5a899b49 created: the schema of a version-0 file.
const LEGACY_SCHEMA: &str = "
CREATE TABLE games (
    id VARCHAR(32) NOT NULL,
    book VARCHAR NOT NULL,
    died_at VARCHAR(32),
    data TEXT NOT NULL,
    PRIMARY KEY (id)
);
CREATE TABLE page_inspections (
    id VARCHAR(32) NOT NULL,
    book VARCHAR NOT NULL,
    path VARCHAR NOT NULL,
    status VARCHAR(16) NOT NULL,
    updated_at VARCHAR(32) NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT uq_page_inspections_book_path UNIQUE (book, path)
);
CREATE TABLE page_views (
    id INTEGER NOT NULL,
    book VARCHAR NOT NULL,
    game VARCHAR(32),
    viewed_at VARCHAR(32) NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY(game) REFERENCES games (id)
);
CREATE INDEX ix_games_book ON games (book);
CREATE INDEX ix_games_book_died_at ON games (book, died_at);
CREATE INDEX ix_page_inspections_book_updated_at ON page_inspections (book, updated_at);
CREATE INDEX ix_page_inspections_status ON page_inspections (status);
CREATE INDEX ix_page_views_book_viewed_at ON page_views (book, viewed_at);
CREATE INDEX ix_page_views_game_viewed_at ON page_views (game, viewed_at);
";

const LIVING: &str = "4f2a91c0000000000000000000000000";
const FALLEN: &str = "0b7e4d2a111111111111111111111111";
const ELSEWHERE: &str = "9c1f4e3b222222222222222222222222";
const OPEN_FILE: &str = "a1b2c3d4333333333333333333333333";
const RESOLVED_FILE: &str = "e5f6a7b8444444444444444444444444";

const FALLEN_DIED_AT: &str = "2026-09-10T09:30:00.000000+00:00";

/// The keys migration 001 takes out of a game's blob.
const MOVED_OUT: [&str; 6] = [
    "force",
    "vie_max",
    "vie_actuelle",
    "gold",
    "items",
    "created_at",
];

fn sword() -> Value {
    json!({"element": "sword", "label": "épée", "count": 1})
}
fn bag() -> Value {
    json!({"element": "bag", "label": "sac", "count": 1})
}
fn rations() -> Value {
    json!({"element": "ration", "label": "ration", "count": 4})
}
fn key() -> Value {
    json!({"element": "key", "label": "clé", "count": 1})
}

fn living_blob() -> Value {
    json!({
        "mode": "normal",
        "force": 13,
        "vie_max": 25,
        "vie_actuelle": 20,
        "force_dice": [3, 4],
        "vie_dice": [5, 2],
        "gold": 7,
        "gold_dice": [1, 2, 3, 1],
        "items": [sword(), bag(), key()],
        "pending": [{
            "element": "strength point",
            "label": "point de Force",
            "amount": 1,
            "condition": "pendant tout le temps où vous les porterez",
            "note": null,
            "sign": 1,
            "page": "22",
        }],
        "combat": {
            "page": "22",
            "fight_type": "single",
            "enemies": [{
                "name": "collecteur", "force": 6, "vie_max": 10,
                "vie_actuelle": 6, "damage_adjustment": 0,
            }],
            "assaults": [{
                "number": 1,
                "hero_dice": [4, 3],
                "hero_attack_force": 20,
                "exchanges": [{
                    "enemy_name": "collecteur",
                    "enemy_force": 6,
                    "enemy_dice": [2, 2],
                    "enemy_attack_force": 10,
                    "winner": "hero",
                    "damage": 4,
                    "divine_judgement": false,
                }],
            }],
            "status": "ongoing",
            "on_victory": "621",
            "on_defeat": "death",
            "on_flee": null,
            "has_special_rules": false,
        },
        "created_at": "2026-09-11T08:00:00.500000+00:00",
    })
}

fn fallen_blob() -> Value {
    json!({
        "mode": "easy",
        "force": 14,
        "vie_max": 29,
        "vie_actuelle": 0,
        "force_dice": [1, 1],
        "vie_dice": [1, 1, 1],
        "gold": 0,
        "gold_dice": [1, 1, 1, 1],
        "items": [sword(), bag(), rations()],
        "pending": [],
        "combat": null,
        // isoformat() leaves the microseconds out when they are zero.
        "created_at": "2026-09-10T08:00:00+00:00",
        "died_on_page": "63",
        "died_of": "les épreuves du chemin",
    })
}

fn elsewhere_blob() -> Value {
    json!({
        "mode": "normal",
        "force": 9,
        "vie_max": 22,
        "vie_actuelle": 22,
        "force_dice": [1, 2],
        "vie_dice": [2, 2],
        "gold": 12,
        "gold_dice": [3, 3, 3, 3],
        "items": [],
        "pending": [],
        "combat": null,
        "created_at": "2026-09-11T07:00:00.000001+00:00",
    })
}

fn open_blob() -> Value {
    json!({
        "page_title": "Page 22",
        "comments": [
            {"text": "Le choix mène au 23.", "created_at": "2026-09-10T10:00:00.123456+00:00"},
            {"text": "Encore : é, à, ç.", "created_at": "2026-09-10T12:00:00+00:00"},
        ],
        "created_at": "2026-09-10T10:00:00.123456+00:00",
    })
}

fn resolved_blob() -> Value {
    json!({
        "page_title": "Page 5",
        "comments": [{"text": "Illustration manquante.", "created_at": "2026-09-11T10:00:00+00:00"}],
        "created_at": "2026-09-11T10:00:00+00:00",
    })
}

/// A blob the way the old code wrote it.
fn blob(values: &Value) -> String {
    serde_json::to_string(values).expect("the blob is always written")
}

/// The scripts, in the order they are applied.
fn migrations() -> Vec<PathBuf> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate sits in the repository")
        .join("migrations");
    let mut scripts: Vec<PathBuf> = std::fs::read_dir(&directory)
        .expect("migrations/ is in the repository")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "sql"))
        .collect();
    scripts.sort();
    scripts
}

/// Write a version-0 database: three heroes, four visits, two flagged pages.
fn legacy_file(directory: &Path) -> PathBuf {
    let path = directory.join(FILE);
    let connection = Connection::open(&path).expect("the file is made");
    connection
        .execute_batch(LEGACY_SCHEMA)
        .expect("the legacy schema is written");
    for (id, book, died_at, data) in [
        (LIVING, BOOK, None, living_blob()),
        (FALLEN, BOOK, Some(FALLEN_DIED_AT), fallen_blob()),
        (ELSEWHERE, OTHER_BOOK, None, elsewhere_blob()),
    ] {
        connection
            .execute(
                "INSERT INTO games (id, book, died_at, data) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![id, book, died_at, blob(&data)],
            )
            .expect("the hero is written");
    }
    for (id, book, game, viewed_at, page) in [
        (1, BOOK, Some(FALLEN), "2026-09-10T08:05:00.000000+00:00", "1"),
        (2, BOOK, None, "2026-09-10T08:06:00.000000+00:00", "5"),
        (3, BOOK, Some(LIVING), "2026-09-11T08:01:00.000000+00:00", "22"),
        (5, BOOK, Some(LIVING), "2026-09-11T08:02:00.000000+00:00", "23"),
    ] {
        connection
            .execute(
                "INSERT INTO page_views (id, book, game, viewed_at, data) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![id, book, game, viewed_at, blob(&json!({"page": page}))],
            )
            .expect("the visit is written");
    }
    for (id, path_, status, updated_at, data) in [
        (OPEN_FILE, "/book/22", "open", "2026-09-10T12:00:00.000000+00:00", open_blob()),
        (
            RESOLVED_FILE,
            "/book/5",
            "resolved",
            "2026-09-11T11:00:00.250000+00:00",
            resolved_blob(),
        ),
    ] {
        connection
            .execute(
                "INSERT INTO page_inspections (id, book, path, status, updated_at, data) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![id, BOOK, path_, status, updated_at, blob(&data)],
            )
            .expect("the flagged page is written");
    }
    path
}

/// Apply every script in `migrations/`, in order, as `sqlite3 -bail` would.
///
/// # Errors
///
/// Whatever SQLite says: a guard refusing, a row the new schema will not take.
fn migrate(path: &Path) -> Result<(), rusqlite::Error> {
    let connection = Connection::open(path).expect("the file is on disk");
    for script in migrations() {
        let sql = std::fs::read_to_string(&script).expect("the script is in the repository");
        connection.execute_batch(&sql)?;
    }
    Ok(())
}

/// Every row a query reads, as objects.
fn rows(path: &Path, query: &str) -> Vec<Map<String, Value>> {
    let connection = Connection::open(path).expect("the file is on disk");
    let mut statement = connection.prepare(query).expect("the query is written here");
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    let read = statement
        .query_map([], |row| {
            let mut found = Map::new();
            for (index, name) in names.iter().enumerate() {
                let value = match row.get_ref(index).expect("the column is there") {
                    rusqlite::types::ValueRef::Null => Value::Null,
                    rusqlite::types::ValueRef::Integer(number) => Value::from(number),
                    rusqlite::types::ValueRef::Real(number) => Value::from(number),
                    rusqlite::types::ValueRef::Text(text) => {
                        Value::from(String::from_utf8_lossy(text).into_owned())
                    }
                    rusqlite::types::ValueRef::Blob(_) => Value::Null,
                };
                found.insert(name.clone(), value);
            }
            Ok(found)
        })
        .expect("the query is written here")
        .flatten()
        .collect();
    read
}

/// Every table and index a file holds, its SQL with the whitespace collapsed.
fn schema_of(path: &Path) -> Vec<(String, String, String)> {
    rows(
        path,
        "SELECT type, name, sql FROM sqlite_master \
         WHERE name NOT LIKE 'sqlite%' ORDER BY type, name",
    )
    .iter()
    .map(|row| {
        (
            row["type"].as_str().unwrap_or("").to_string(),
            row["name"].as_str().unwrap_or("").to_string(),
            row["sql"]
                .as_str()
                .unwrap_or("")
                .split_whitespace()
                .collect::<Vec<&str>>()
                .join(" "),
        )
    })
    .collect()
}

/// The whole file: its schema, every row of every table, and its version.
fn everything_in(path: &Path) -> String {
    let mut written = String::new();
    for (kind, name, sql) in schema_of(path) {
        written.push_str(&format!("{kind} {name} {sql}\n"));
        if kind == "table" {
            for row in rows(path, &format!("SELECT * FROM \"{name}\" ORDER BY rowid")) {
                written.push_str(&format!("{}\n", Value::Object(row)));
            }
        }
    }
    let connection = Connection::open(path).expect("the file is on disk");
    let version: i64 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap_or(-1);
    written.push_str(&format!("user_version = {version}"));
    written
}

/// A copy of an object with some keys left out.
fn without(values: &Value, keys: &[&str]) -> Value {
    let mut left = values.as_object().cloned().unwrap_or_default();
    for key in keys {
        left.remove(*key);
    }
    Value::Object(left)
}

/// The same object with one key written onto it.
fn with(values: &Value, key: &str, value: Value) -> Value {
    let mut written = values.as_object().cloned().unwrap_or_default();
    written.insert(key.to_string(), value);
    Value::Object(written)
}

/// A version-0 file, with every migration applied.
struct Migrated {
    directory: tempfile::TempDir,
    path: PathBuf,
}

impl Migrated {
    /// Build one, and apply every script to it.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = legacy_file(directory.path());
        migrate(&path).expect("a version-0 file always migrates");
        Self { directory, path }
    }

    /// `core::db`, pointed at the migrated file.
    async fn db(&self) -> Db {
        Db::Bound(
            open_engine(&self.path)
                .await
                .expect("the migrated file always opens"),
        )
    }
}

/// What `migrations/` holds.
mod the_scripts {
    use super::*;

    #[test]
    fn they_are_numbered_from_one_without_a_gap() {
        let numbers: Vec<String> = migrations()
            .iter()
            .map(|script| {
                script
                    .file_name()
                    .map(|name| name.to_string_lossy()[..3].to_string())
                    .unwrap_or_default()
            })
            .collect();
        let expected: Vec<String> = (1..=migrations().len())
            .map(|number| format!("{number:03}"))
            .collect();

        assert_eq!(numbers, expected);
    }

    #[test]
    fn the_last_one_reaches_the_version_the_code_reads() {
        assert_eq!(migrations().len(), SCHEMA_VERSION as usize);
    }
}

/// What a migrated file is made of.
mod the_schema {
    use super::*;

    #[tokio::test]
    async fn it_is_the_schema_a_new_file_gets() {
        let migrated = Migrated::new();
        let fresh_dir = tempfile::tempdir().expect("a temporary directory is always made");
        let fresh = fresh_dir.path().join(FILE);
        let connection = open_engine(&fresh).await.expect("a new file always opens");
        create_the_schema(&connection)
            .await
            .expect("a new file always takes the schema");
        drop(connection);

        assert_eq!(schema_of(&migrated.path), schema_of(&fresh));
    }

    #[test]
    fn it_carries_the_version_the_code_reads() {
        let migrated = Migrated::new();

        assert!(everything_in(&migrated.path)
            .ends_with(&format!("user_version = {SCHEMA_VERSION}")));
    }

    #[test]
    fn nothing_old_is_left_behind() {
        let migrated = Migrated::new();

        let names: Vec<String> = schema_of(&migrated.path)
            .into_iter()
            .map(|(_, name, _)| name)
            .collect();
        assert_eq!(
            names,
            [
                "ix_games_book",
                "ix_games_book_died_at",
                "ix_items_game_id",
                "ix_page_inspections_book_updated_at",
                "ix_page_inspections_status",
                "ix_page_views_book_viewed_at",
                "ix_page_views_game_id_viewed_at",
                "games",
                "items",
                "page_inspections",
                "page_views",
            ]
        );
    }
}

/// Renumbered, their stats in columns, their blob otherwise untouched.
mod the_games {
    use super::*;

    #[test]
    fn they_are_numbered_in_the_order_they_were_created() {
        let migrated = Migrated::new();

        let numbered: Vec<(i64, String)> = rows(&migrated.path, "SELECT id, data FROM games ORDER BY id")
            .iter()
            .map(|game| {
                let data: Value = serde_json::from_str(game["data"].as_str().unwrap_or("{}"))
                    .expect("the blob is JSON");
                (
                    game["id"].as_i64().unwrap_or(0),
                    data["legacy_id"].as_str().unwrap_or("").to_string(),
                )
            })
            .collect();

        assert_eq!(
            numbered,
            [
                (1, FALLEN.to_string()),
                (2, ELSEWHERE.to_string()),
                (3, LIVING.to_string()),
            ]
        );
    }

    #[test]
    fn the_stats_leave_the_blob_for_their_columns() {
        let migrated = Migrated::new();

        let game = rows(
            &migrated.path,
            "SELECT force, vie_max, vie_actuelle, gold FROM games WHERE id = 3",
        );

        assert_eq!(
            Value::Object(game[0].clone()),
            json!({"force": 13, "vie_max": 25, "vie_actuelle": 20, "gold": 7})
        );
    }

    #[test]
    fn the_rest_of_the_blob_is_kept_as_it_was() {
        let migrated = Migrated::new();

        let blobs: Vec<Value> = rows(&migrated.path, "SELECT data FROM games ORDER BY id")
            .iter()
            .map(|game| {
                serde_json::from_str(game["data"].as_str().unwrap_or("{}"))
                    .expect("the blob is JSON")
            })
            .collect();

        assert_eq!(
            blobs,
            [
                with(&without(&fallen_blob(), &MOVED_OUT), "legacy_id", json!(FALLEN)),
                with(&without(&elsewhere_blob(), &MOVED_OUT), "legacy_id", json!(ELSEWHERE)),
                with(&without(&living_blob(), &MOVED_OUT), "legacy_id", json!(LIVING)),
            ]
        );
    }

    #[test]
    fn created_at_is_padded_to_one_width() {
        let migrated = Migrated::new();

        let stamps: Vec<String> = rows(&migrated.path, "SELECT created_at FROM games ORDER BY id")
            .iter()
            .map(|game| game["created_at"].as_str().unwrap_or("").to_string())
            .collect();

        assert_eq!(
            stamps,
            [
                "2026-09-10T08:00:00.000000+00:00",
                "2026-09-11T07:00:00.000001+00:00",
                "2026-09-11T08:00:00.500000+00:00",
            ]
        );
    }

    #[test]
    fn updated_at_is_the_last_instant_the_old_file_knew() {
        let migrated = Migrated::new();

        let stamps: Vec<String> = rows(&migrated.path, "SELECT updated_at FROM games ORDER BY id")
            .iter()
            .map(|game| game["updated_at"].as_str().unwrap_or("").to_string())
            .collect();

        assert_eq!(
            stamps,
            [
                FALLEN_DIED_AT,
                "2026-09-11T07:00:00.000001+00:00",
                "2026-09-11T08:00:00.500000+00:00",
            ]
        );
    }
}

/// One row per line, in bag order.
mod the_bag {
    use super::*;

    #[test]
    fn each_line_is_a_row_of_its_game() {
        let migrated = Migrated::new();

        let lines: Vec<(i64, i64, Value)> =
            rows(&migrated.path, "SELECT id, game_id, data FROM items ORDER BY id")
                .iter()
                .map(|line| {
                    (
                        line["id"].as_i64().unwrap_or(0),
                        line["game_id"].as_i64().unwrap_or(0),
                        serde_json::from_str(line["data"].as_str().unwrap_or("{}"))
                            .expect("the blob is JSON"),
                    )
                })
                .collect();

        assert_eq!(
            lines,
            [
                (1, 1, sword()),
                (2, 1, bag()),
                (3, 1, rations()),
                (4, 3, sword()),
                (5, 3, bag()),
                (6, 3, key()),
            ]
        );
    }

    #[test]
    fn each_line_is_stamped_when_the_migration_ran() {
        let migrated = Migrated::new();

        for line in rows(&migrated.path, "SELECT created_at, updated_at FROM items") {
            assert_eq!(line["created_at"], line["updated_at"]);
            assert_eq!(
                line["created_at"].as_str().unwrap_or("").len(),
                "2026-09-11T08:00:00.500000+00:00".len()
            );
        }
    }
}

/// Their ids kept, their hero followed to his new number.
mod the_visits {
    use super::*;

    #[test]
    fn each_visit_keeps_its_id_and_its_hero() {
        let migrated = Migrated::new();

        let visits: Vec<(i64, Value, Value)> =
            rows(&migrated.path, "SELECT id, game_id, data FROM page_views ORDER BY id")
                .iter()
                .map(|visit| {
                    (
                        visit["id"].as_i64().unwrap_or(0),
                        visit["game_id"].clone(),
                        serde_json::from_str(visit["data"].as_str().unwrap_or("{}"))
                            .expect("the blob is JSON"),
                    )
                })
                .collect();

        assert_eq!(
            visits,
            [
                (1, json!(1), json!({"page": "1"})),
                (2, Value::Null, json!({"page": "5"})),
                (3, json!(3), json!({"page": "22"})),
                (5, json!(3), json!({"page": "23"})),
            ]
        );
    }
}

/// Renumbered, their dates in columns, their comments untouched.
mod the_flagged_pages {
    use super::*;

    #[test]
    fn each_file_keeps_its_dates_and_its_thread() {
        let migrated = Migrated::new();

        let files = rows(&migrated.path, "SELECT * FROM page_inspections ORDER BY id");

        let columns: Vec<Value> = files
            .iter()
            .map(|file| without(&Value::Object(file.clone()), &["data"]))
            .collect();
        assert_eq!(
            columns,
            [
                json!({
                    "id": 1,
                    "book": BOOK,
                    "path": "/book/22",
                    "status": "open",
                    "created_at": "2026-09-10T10:00:00.123456+00:00",
                    "updated_at": "2026-09-10T12:00:00.000000+00:00",
                }),
                json!({
                    "id": 2,
                    "book": BOOK,
                    "path": "/book/5",
                    "status": "resolved",
                    "created_at": "2026-09-11T10:00:00.000000+00:00",
                    "updated_at": "2026-09-11T11:00:00.250000+00:00",
                }),
            ]
        );
        let blobs: Vec<Value> = files
            .iter()
            .map(|file| {
                serde_json::from_str(file["data"].as_str().unwrap_or("{}"))
                    .expect("the blob is JSON")
            })
            .collect();
        assert_eq!(
            blobs,
            [
                with(&without(&open_blob(), &["created_at"]), "legacy_id", json!(OPEN_FILE)),
                with(
                    &without(&resolved_blob(), &["created_at"]),
                    "legacy_id",
                    json!(RESOLVED_FILE)
                ),
            ]
        );
    }
}

/// The migrated file, read and written through `core::db`.
mod the_application_reads_it {
    use super::*;

    #[tokio::test]
    async fn the_living_hero_is_whole() {
        let migrated = Migrated::new();
        let db = migrated.db().await;

        let hero = haute_tension::core::db::find_game(&db, Some(3))
            .await
            .expect("the file answers")
            .expect("the hero came across");

        assert_eq!(
            (hero.force, hero.vie_max, hero.vie_actuelle, hero.gold),
            (13, 25, 20, 7)
        );
        let carried: Vec<(&str, &str, i64)> = hero
            .bag
            .iter()
            .map(|item| (item.element.as_str(), item.label.as_str(), item.count))
            .collect();
        assert_eq!(
            carried,
            [("sword", "épée", 1), ("bag", "sac", 1), ("key", "clé", 1)]
        );
        assert_eq!(
            hero.pending[0].condition.as_deref(),
            Some("pendant tout le temps où vous les porterez")
        );
        assert_eq!(
            hero.combat.expect("the fight is on").assaults[0].exchanges[0].damage,
            4
        );
        assert!(!hero.is_dead);
    }

    #[tokio::test]
    async fn the_fallen_are_remembered() {
        let migrated = Migrated::new();
        let db = migrated.db().await;

        let fallen = fallen_heroes(&db, BOOK, MAX_FALLEN_HEROES)
            .await
            .expect("the file answers");

        assert_eq!(fallen.len(), 1);
        assert_eq!(fallen[0].id, 1);
        assert_eq!(fallen[0].died_on_page.as_deref(), Some("63"));
        assert_eq!(
            fallen[0].died_at.as_deref(),
            Some("2026-09-10T09:30:00+00:00")
        );
        let carried: Vec<&str> = fallen[0]
            .bag
            .iter()
            .map(|item| item.element.as_str())
            .collect();
        assert_eq!(carried, ["sword", "bag", "ration"]);
    }

    #[tokio::test]
    async fn the_reading_history_reads_as_before() {
        let migrated = Migrated::new();
        let db = migrated.db().await;

        assert_eq!(
            last_pages(&db, BOOK, MAX_PAGE_HISTORY)
                .await
                .expect("the file answers"),
            ["1", "5", "22", "23"]
        );
        assert_eq!(
            last_page_read(&db, 3).await.expect("the file answers"),
            Some("23".to_string())
        );
        assert_eq!(
            last_page_read(&db, 1).await.expect("the file answers"),
            Some("1".to_string())
        );
    }

    #[tokio::test]
    async fn the_flagged_pages_are_listed_as_before() {
        let migrated = Migrated::new();
        let db = migrated.db().await;

        let listed: Vec<(i64, String)> = flagged_pages(&db, BOOK, None)
            .await
            .expect("the file answers")
            .iter()
            .map(|filed| (filed.id, filed.path.clone()))
            .collect();

        assert_eq!(
            listed,
            [(2, "/book/5".to_string()), (1, "/book/22".to_string())]
        );
        assert_eq!(
            find_inspection(&db, Some(2))
                .await
                .expect("the file answers")
                .expect("the file is there")
                .created_at,
            "2026-09-11T10:00:00+00:00"
        );
    }

    #[tokio::test]
    async fn play_goes_on_where_it_stopped() {
        let migrated = Migrated::new();
        let db = migrated.db().await;
        let gain_a_key = json!({
            "goto": "24",
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
            "losses": [],
        })
        .as_object()
        .cloned()
        .expect("the choice is an object");

        let after = follow_choice(&db, Some(3), "23", &gain_a_key)
            .await
            .expect("the file answers")
            .expect("the hero is there");

        let carried: Vec<(&str, i64)> = after
            .bag
            .iter()
            .map(|item| (item.element.as_str(), item.count))
            .collect();
        assert_eq!(carried, [("sword", 1), ("bag", 1), ("key", 2)]);
        let ids: Vec<i64> = rows(&migrated.path, "SELECT id FROM items WHERE game_id = 3")
            .iter()
            .map(|line| line["id"].as_i64().unwrap_or(0))
            .collect();
        assert_eq!(ids, [4, 5, 6]);
    }

    #[tokio::test]
    async fn a_new_hero_is_numbered_after_the_old_ones() {
        let migrated = Migrated::new();
        let db = migrated.db().await;

        assert_eq!(
            start_game(&db, BOOK, NORMAL, Some(&seeded(1)))
                .await
                .expect("the file answers")
                .id,
            4
        );
    }

    #[tokio::test]
    async fn a_reader_whose_cookie_predates_it_has_no_hero() {
        let migrated = Migrated::new();
        let client = client_for(migrated.db().await, books_path(), None);
        client.set_session(SESSION_KEY, json!(LIVING));

        assert_eq!(client.get("/game").await.status, 200);
        assert!(client.session().get(SESSION_KEY).is_none());
    }
}

/// What the script refuses, and what a refusal leaves behind.
mod the_guards {
    use super::*;

    #[test]
    fn a_second_run_is_refused_and_changes_nothing() {
        let migrated = Migrated::new();
        let before = everything_in(&migrated.path);

        let trouble = migrate(&migrated.path).expect_err("a migrated file is refused");

        assert!(trouble
            .to_string()
            .contains("expects_a_version_0_database"));
        assert_eq!(everything_in(&migrated.path), before);
    }

    #[test]
    fn a_file_the_current_code_created_is_refused() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = directory.path().join(FILE);
        let runtime = tokio::runtime::Runtime::new().expect("a runtime is always made");
        runtime.block_on(async {
            let connection = open_engine(&path).await.expect("a new file always opens");
            create_the_schema(&connection)
                .await
                .expect("a new file always takes the schema");
        });

        let trouble = migrate(&path).expect_err("a current file is refused");

        assert!(trouble
            .to_string()
            .contains("expects_a_version_0_database"));
    }

    #[test]
    fn a_row_the_new_schema_refuses_keeps_the_old_file_whole() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = legacy_file(directory.path());
        {
            let connection = Connection::open(&path).expect("the file is on disk");
            connection
                .execute(
                    "UPDATE games SET data = json_set(data, '$.gold', -1) WHERE id = ?1",
                    rusqlite::params![LIVING],
                )
                .expect("the hero is changed");
        }
        let before = everything_in(&path);

        let trouble = migrate(&path).expect_err("a negative purse is refused");

        assert!(trouble
            .to_string()
            .contains("ck_games_gold_not_negative"));
        assert_eq!(everything_in(&path), before);
    }

    #[test]
    fn a_visit_whose_hero_is_missing_stops_it() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = legacy_file(directory.path());
        {
            let connection = Connection::open(&path).expect("the file is on disk");
            // The old file was written with foreign keys unchecked, which is how
            // a visit could come to point at a hero that is not there.
            connection
                .execute_batch("PRAGMA foreign_keys = OFF")
                .expect("the pragma is set");
            connection
                .execute(
                    "INSERT INTO page_views (id, book, game, viewed_at, data) \
                     VALUES (9, ?1, 'ghost', '2026-09-11T12:00:00.000000+00:00', '{}')",
                    rusqlite::params![BOOK],
                )
                .expect("the visit is written");
        }
        let before = everything_in(&path);

        let trouble = migrate(&path).expect_err("a visit without a hero is refused");

        assert!(trouble
            .to_string()
            .contains("every_visit_kept_its_hero"));
        assert_eq!(everything_in(&path), before);
    }
}

/// Keeps the temporary directory alive for the length of a migrated file.
#[test]
fn a_migrated_file_lives_in_a_directory_of_its_own() {
    let migrated = Migrated::new();

    assert!(migrated.directory.path().is_dir());
    assert!(migrated.path.starts_with(migrated.directory.path()));
}
