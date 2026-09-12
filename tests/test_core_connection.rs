//! Tests for opening the engine every read and write goes through.
//!
//! The one part of `core::db` the other tests stand in for, so it is exercised
//! here for real: on SQLite files in a temporary directory, which is what a run
//! opens.
//!
//! Every test takes the environment lock and resets the process-wide
//! connection: both belong to the process.

// The environment and the log belong to the process, so the lock that
// serialises the tests reading them is held for the length of each one.
#![allow(clippy::await_holding_lock)]

#[path = "common/mod.rs"]
mod common;

use common::{clean_env, env_lock, EnvVar};
use haute_tension::core::db::connection::{object_names, open_engine};
use haute_tension::core::db::{
    connect_db, connected_to, create_the_schema, engine, last_pages, opened_file, record_page_view,
    reset_connection, schema_version, table_names, DatabaseFailure, Db, MAX_PAGE_HISTORY,
    SCHEMA_VERSION,
};
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
use std::path::Path;

const FILE: &str = "haute_tension_dev.sqlite3";

/// A directory of the test's own for the files `connect_db` opens.
struct DatabaseDir {
    directory: tempfile::TempDir,
    _set: EnvVar,
}

impl DatabaseDir {
    /// Point the environment at a directory nothing else uses.
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let _set = EnvVar::set("DATABASE_DIR", &directory.path().display().to_string());
        Self { directory, _set }
    }

    /// The directory itself.
    fn path(&self) -> &Path {
        self.directory.path()
    }
}

impl Drop for DatabaseDir {
    fn drop(&mut self) {
        reset_connection();
    }
}

/// The version a file on disk carries.
fn version_of(path: &Path) -> i64 {
    sqlite_open(path)
        .query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))
        .unwrap_or(-1)
}

/// The tables a file on disk holds.
fn tables_of(path: &Path) -> Vec<String> {
    let connection = sqlite_open(path);
    let mut statement = connection
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .expect("the query is written here");
    let names = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("the query is written here")
        .flatten()
        .collect();
    names
}

/// Open one file the way a tool outside the application would.
fn sqlite_open(path: &Path) -> rusqlite::Connection {
    rusqlite::Connection::open(path).expect("the file is on disk")
}

/// Write a file carrying a schema version, and a table unless told not to.
fn a_file_at(path: &Path, version: i64, with_a_table: bool) {
    let connection = sqlite_open(path);
    if with_a_table {
        connection
            .execute("CREATE TABLE games (id VARCHAR(32) PRIMARY KEY)", [])
            .expect("the table is created");
    }
    connection
        .execute_batch(&format!("PRAGMA user_version = {version}"))
        .expect("the version is written");
}

/// Opening, reusing and refusing a connection.
mod connecting {
    use super::*;

    #[tokio::test]
    async fn the_env_database_is_opened() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();

        connect_db().await.expect("the directory is there");

        assert_eq!(
            opened_file(),
            Some(database_dir.path().join(FILE).display().to_string())
        );
    }

    #[tokio::test]
    async fn the_tables_are_created() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();

        connect_db().await.expect("the directory is there");

        let connection = engine().expect("the engine is open");
        assert_eq!(
            table_names(&connection).await.expect("the file answers"),
            ["games", "items", "page_inspections", "page_views"]
        );
    }

    #[tokio::test]
    async fn a_missing_directory_is_made() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        let deeper = database_dir.path().join("deeper").join("still");
        let _set = EnvVar::set("DATABASE_DIR", &deeper.display().to_string());

        connect_db().await.expect("the directory is made");

        assert!(deeper.is_dir());
    }

    #[tokio::test]
    async fn a_second_call_reuses_the_engine() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();

        connect_db().await.expect("the directory is there");
        let first = connected_to();
        connect_db().await.expect("the directory is there");

        assert_eq!(connected_to(), first);
        assert_eq!(connected_to().as_deref(), Some("haute_tension_dev"));
    }

    #[tokio::test]
    async fn switching_env_reopens() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        connect_db().await.expect("the directory is there");
        let dev = opened_file();

        let _set = EnvVar::set("APP_ENV", "prod");
        connect_db().await.expect("the directory is there");

        assert_ne!(opened_file(), dev);
        assert_eq!(
            opened_file(),
            Some(
                database_dir
                    .path()
                    .join("haute_tension_prod.sqlite3")
                    .display()
                    .to_string()
            )
        );
    }

    #[tokio::test]
    async fn a_missing_directory_variable_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();

        let trouble = connect_db().await.expect_err("nothing is configured");

        assert!(trouble.to_string().contains("Missing DATABASE_DIR"));
        assert!(trouble.is_unavailable());
    }

    #[tokio::test]
    async fn a_directory_that_is_a_file_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        let not_a_directory = database_dir.path().join("a_file");
        std::fs::write(&not_a_directory, "").expect("the file is written");
        let _set = EnvVar::set("DATABASE_DIR", &not_a_directory.display().to_string());

        let trouble = connect_db().await.expect_err("that is not a directory");

        assert!(trouble.to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn a_refusal_leaves_no_engine_behind() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        {
            let _blank = EnvVar::set("DATABASE_DIR", "");
            connect_db().await.expect_err("nothing is configured");

            assert!(engine().is_none());
        }

        let _set = EnvVar::set("DATABASE_DIR", &database_dir.path().display().to_string());
        connect_db().await.expect("the directory is there");

        assert!(engine().is_some());
    }

    /// The whole point: a second engine on the same file sees the row.
    #[tokio::test]
    async fn what_is_written_is_read_back_from_the_file() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();

        record_page_view(&Db::Environment, "livre", "22", None)
            .await
            .expect("the file answers");
        reset_connection();

        assert_eq!(
            last_pages(&Db::Environment, "livre", MAX_PAGE_HISTORY)
                .await
                .expect("the file answers"),
            ["22"]
        );
    }

    /// SQLite only checks them when told to, on every connection.
    #[tokio::test]
    async fn foreign_keys_are_enforced() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();
        connect_db().await.expect("the directory is there");
        let connection = engine().expect("the engine is open");

        let refused = connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                "INSERT INTO page_views (book, game_id, viewed_at, data) \
                 VALUES ('livre', 999, '2026', '{}')",
            ))
            .await;

        let trouble = refused.expect_err("the hero does not exist");
        assert!(trouble.to_string().contains("FOREIGN KEY"));
    }
}

/// Marking a new file, and refusing one another schema wrote.
mod the_schema_version {
    use super::*;

    #[tokio::test]
    async fn a_new_file_is_marked_with_the_current_version() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();

        connect_db().await.expect("the directory is there");

        assert_eq!(
            version_of(&database_dir.path().join(FILE)),
            i64::from(SCHEMA_VERSION)
        );
    }

    #[tokio::test]
    async fn a_file_an_older_schema_wrote_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        a_file_at(&database_dir.path().join(FILE), 0, true);

        let trouble = connect_db().await.expect_err("the file is older");

        let written = trouble.to_string();
        assert!(written.contains("schema version 0, and this code reads version 1"));
        assert!(written.contains("migrations/"));
        assert!(engine().is_none());
    }

    #[tokio::test]
    async fn a_refused_file_is_left_as_it_was() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        let path = database_dir.path().join(FILE);
        a_file_at(&path, 0, true);

        connect_db().await.expect_err("the file is older");

        assert_eq!(version_of(&path), 0);
        assert_eq!(tables_of(&path), ["games"]);
    }

    #[tokio::test]
    async fn a_file_a_newer_schema_wrote_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        a_file_at(
            &database_dir.path().join(FILE),
            i64::from(SCHEMA_VERSION) + 1,
            true,
        );

        let trouble = connect_db().await.expect_err("the file is newer");

        assert!(trouble
            .to_string()
            .contains(&format!("schema version {},", SCHEMA_VERSION + 1)));
    }

    #[tokio::test]
    async fn a_file_without_tables_is_new_whatever_it_says() {
        let _lock = env_lock();
        let _clean = clean_env();
        let database_dir = DatabaseDir::new();
        a_file_at(&database_dir.path().join(FILE), 7, false);

        connect_db().await.expect("an empty file is a new one");

        assert_eq!(
            version_of(&database_dir.path().join(FILE)),
            i64::from(SCHEMA_VERSION)
        );
    }

    #[tokio::test]
    async fn a_file_at_the_current_version_is_opened_again() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();

        record_page_view(&Db::Environment, "livre", "1", None)
            .await
            .expect("the file answers");
        reset_connection();

        assert_eq!(
            last_pages(&Db::Environment, "livre", MAX_PAGE_HISTORY)
                .await
                .expect("the file answers"),
            ["1"]
        );
    }

    /// A file the application made carries the schema and the version together.
    #[tokio::test]
    async fn a_fresh_file_gets_the_whole_schema() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let connection = open_engine(&directory.path().join(FILE))
            .await
            .expect("a new file always opens");

        create_the_schema(&connection)
            .await
            .expect("a new file always takes the schema");

        assert_eq!(
            schema_version(&connection).await.expect("the file answers"),
            SCHEMA_VERSION
        );
        assert_eq!(
            object_names(&connection).await.expect("the file answers"),
            [
                "games",
                "items",
                "ix_games_book",
                "ix_games_book_died_at",
                "ix_items_game_id",
                "ix_page_inspections_book_updated_at",
                "ix_page_inspections_status",
                "ix_page_views_book_viewed_at",
                "ix_page_views_game_id_viewed_at",
                "page_inspections",
                "page_views",
            ]
        );
    }
}

/// Disposing of the open engine, which only the tests need.
mod resetting_the_connection {
    use super::*;

    #[tokio::test]
    async fn the_next_call_reopens() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();
        connect_db().await.expect("the directory is there");
        reset_connection();

        assert!(engine().is_none());

        connect_db().await.expect("the directory is there");

        assert!(engine().is_some());
    }

    #[tokio::test]
    async fn the_engine_is_dropped_not_just_forgotten() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _database_dir = DatabaseDir::new();
        connect_db().await.expect("the directory is there");
        reset_connection();

        assert!(engine().is_none());
        assert!(connected_to().is_none());
    }

    #[tokio::test]
    async fn resetting_without_an_engine_is_harmless() {
        let _lock = env_lock();
        let _clean = clean_env();
        reset_connection();

        assert!(engine().is_none());
    }
}

/// What a caller catches around a database call.
mod the_database_failure {
    use super::*;

    #[test]
    fn the_driver_family_is_caught() {
        let trouble = DatabaseFailure::from(sea_orm::DbErr::Custom("refused".to_string()));

        assert!(!trouble.is_unavailable());
        assert_eq!(trouble.kind(), "OperationalError");
    }

    /// Never configuring a database is not a driver error, and is caught too.
    #[test]
    fn no_database_at_all_is_a_failure_of_its_own() {
        let trouble = DatabaseFailure::Unavailable("none".to_string());

        assert!(trouble.is_unavailable());
        assert_eq!(trouble.kind(), "DatabaseUnavailable");
    }

    /// One failure covers both, and a caller tells them apart without catching
    /// everything the file system can raise.
    #[tokio::test]
    async fn it_is_narrower_than_every_file_system_error() {
        let missing = Db::Unavailable("none".to_string())
            .conn()
            .await
            .expect_err("nothing is configured");
        let refusing = Db::Failing("unable to open database file".to_string())
            .conn()
            .await
            .expect_err("the driver refuses");

        assert!(missing.is_unavailable());
        assert!(!refusing.is_unavailable());
    }
}
