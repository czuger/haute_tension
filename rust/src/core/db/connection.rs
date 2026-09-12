//! Opening the database every read and write goes through.
//!
//! The engine is opened on the first call rather than at startup, so building
//! the application never needs a file — and a test can stand an in-memory one in
//! by handing the routes a [`Db`] of its own.
//!
//! A file is only opened if it carries the schema the models describe: a new
//! file is given it, and a file an older schema wrote is refused until the
//! scripts in `migrations/` have been applied to it.

use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use sea_orm::{ConnectionTrait, DatabaseBackend, DatabaseConnection, DbErr, Statement};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

use crate::core::config::{current_db_name, database_path, EnvironmentError, DATABASE_DIR_VAR};
use crate::core::db::schema::{INDEXES, SCHEMA_VERSION, TABLES};

/// Every way the database can fail a request, which is what the application
/// turns into one page: the driver refusing, and there being nothing configured
/// to refuse in the first place.
#[derive(Debug, thiserror::Error)]
pub enum DatabaseFailure {
    /// There is no database to talk to, and no request can pretend otherwise.
    #[error("{0}")]
    Unavailable(String),
    /// The driver refused: a file that cannot be opened or written, a constraint
    /// refused, a query the engine will not run.
    #[error("{0}")]
    Driver(#[from] DbErr),
}

impl DatabaseFailure {
    /// The name this failure carries into a log line, as a traceback would.
    pub fn kind(&self) -> &'static str {
        match self {
            DatabaseFailure::Unavailable(_) => "DatabaseUnavailable",
            DatabaseFailure::Driver(_) => "OperationalError",
        }
    }

    /// Whether nothing was configured, rather than the driver refusing.
    pub fn is_unavailable(&self) -> bool {
        matches!(self, DatabaseFailure::Unavailable(_))
    }
}

impl From<EnvironmentError> for DatabaseFailure {
    fn from(trouble: EnvironmentError) -> Self {
        DatabaseFailure::Unavailable(trouble.0)
    }
}

/// The database as the routes hold it.
///
/// [`Db::Environment`] is what a run uses: the file `APP_ENV` picks, opened
/// once. The other three are the seam the tests replace — a database of their
/// own, one that is not there, and one whose driver refuses everything.
#[derive(Clone, Debug)]
pub enum Db {
    /// The file the environment names, opened on first use.
    Environment,
    /// A connection handed over ready-made.
    Bound(DatabaseConnection),
    /// Nothing configured to reach.
    Unavailable(String),
    /// A driver that refuses every call.
    Failing(String),
}

impl Db {
    /// The connection to read and write on.
    ///
    /// # Errors
    ///
    /// If there is no database configured, or the one configured cannot be
    /// opened.
    pub async fn conn(&self) -> Result<DatabaseConnection, DatabaseFailure> {
        match self {
            Db::Environment => {
                connect_db().await?;
                engine().ok_or_else(|| {
                    DatabaseFailure::Unavailable("The database was closed under us.".to_string())
                })
            }
            Db::Bound(connection) => Ok(connection.clone()),
            Db::Unavailable(message) => Err(DatabaseFailure::Unavailable(message.clone())),
            Db::Failing(message) => Err(DatabaseFailure::Driver(DbErr::Conn(
                sea_orm::RuntimeErr::Internal(message.clone()),
            ))),
        }
    }
}

/// The engine every session opens on, and the database it was opened on, so a
/// run that switches `APP_ENV` mid-flight (the tests do) reopens instead of
/// reading the wrong file.
struct Opened {
    name: String,
    file: String,
    connection: DatabaseConnection,
}

fn opened() -> &'static Mutex<Option<Opened>> {
    static OPENED: OnceLock<Mutex<Option<Opened>>> = OnceLock::new();
    OPENED.get_or_init(|| Mutex::new(None))
}

/// The open connection, or nothing when none has been opened.
pub fn engine() -> Option<DatabaseConnection> {
    opened()
        .lock()
        .expect("the engine outlives a panic")
        .as_ref()
        .map(|open| open.connection.clone())
}

/// The database the open connection is on, for the log and the tests.
pub fn connected_to() -> Option<String> {
    opened()
        .lock()
        .expect("the engine outlives a panic")
        .as_ref()
        .map(|open| open.name.clone())
}

/// The file the open connection is on.
pub fn opened_file() -> Option<String> {
    opened()
        .lock()
        .expect("the engine outlives a panic")
        .as_ref()
        .map(|open| open.file.clone())
}

/// Dispose of the open engine, so the next call reopens it.
///
/// Only the tests need this: a process normally works on one database for its
/// whole life.
pub fn reset_connection() {
    let _ = opened()
        .lock()
        .expect("the engine outlives a panic")
        .take();
}

/// Open the engine on the file `APP_ENV` picks, once per database name.
///
/// A new file is given the tables and marked with [`SCHEMA_VERSION`]. A file
/// carrying another version is refused rather than half-read: the scripts in
/// `migrations/` are what move it.
///
/// # Errors
///
/// If `DATABASE_DIR` is unset, names a file rather than a directory, or leads to
/// a file another schema wrote.
pub async fn connect_db() -> Result<(), DatabaseFailure> {
    let db_name = current_db_name()?;
    if connected_to().as_deref() == Some(db_name.as_str()) {
        return Ok(());
    }

    let Some(path) = database_path()? else {
        return Err(DatabaseFailure::Unavailable(format!(
            "Missing {DATABASE_DIR_VAR} environment variable. \
             Copy .env.example to .env and fill it in."
        )));
    };
    let directory = path
        .parent()
        .ok_or_else(|| DatabaseFailure::Unavailable(format!("{DATABASE_DIR_VAR} names no directory.")))?;
    if directory.exists() && !directory.is_dir() {
        return Err(DatabaseFailure::Unavailable(format!(
            "{DATABASE_DIR_VAR} points at '{}', which is not a directory. \
             It must name the directory the database files live in.",
            directory.display()
        )));
    }

    reset_connection();
    std::fs::create_dir_all(directory).map_err(|trouble| {
        DatabaseFailure::Unavailable(format!(
            "{DATABASE_DIR_VAR} points at '{}', which cannot be made: {trouble}",
            directory.display()
        ))
    })?;
    let connection = open_engine(&path).await?;
    refuse_another_schema(&connection, &path).await?;
    create_the_schema(&connection).await?;

    *opened().lock().expect("the engine outlives a panic") = Some(Opened {
        name: db_name,
        file: path.display().to_string(),
        connection,
    });
    Ok(())
}

/// Build a connection on a SQLite file, with foreign keys enforced.
///
/// SQLite checks foreign keys only when asked to, connection by connection.
pub async fn open_engine(path: &Path) -> Result<DatabaseConnection, DatabaseFailure> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(options)
        .await
        .map_err(|trouble| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(trouble)))?;
    Ok(sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

/// Build a connection on a database of its own, held in memory.
///
/// One connection for the whole of it, because an in-memory SQLite lives and
/// dies with the connection that opened it.
pub async fn open_memory_engine() -> Result<DatabaseConnection, DatabaseFailure> {
    let options = SqliteConnectOptions::from_str("sqlite::memory:")
        .map_err(|trouble| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(trouble)))?
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .min_connections(1)
        .idle_timeout(None)
        .max_lifetime(None)
        .connect_with(options)
        .await
        .map_err(|trouble| DbErr::Conn(sea_orm::RuntimeErr::SqlxError(trouble)))?;
    Ok(sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
}

/// Refuse a file whose tables another version of the schema wrote.
///
/// A file with no table at all is a new one, whatever its version says.
///
/// # Errors
///
/// If the file has tables and a version other than [`SCHEMA_VERSION`].
async fn refuse_another_schema(
    connection: &DatabaseConnection,
    path: &Path,
) -> Result<(), DatabaseFailure> {
    let version = schema_version(connection).await?;
    let has_tables = !table_names(connection).await?.is_empty();
    if !has_tables || version == SCHEMA_VERSION {
        return Ok(());
    }
    connection.clone().close().await.ok();
    Err(DatabaseFailure::Unavailable(format!(
        "The database file '{}' carries schema version {version}, and this code \
         reads version {SCHEMA_VERSION}. Back the file up, then apply the scripts \
         in migrations/ to it (see README).",
        path.display()
    )))
}

/// The version a file carries.
pub async fn schema_version(connection: &DatabaseConnection) -> Result<i32, DbErr> {
    let row = connection
        .query_one(Statement::from_string(
            DatabaseBackend::Sqlite,
            "PRAGMA user_version",
        ))
        .await?
        .ok_or_else(|| DbErr::Custom("a file always carries a version".to_string()))?;
    row.try_get_by_index::<i32>(0)
}

/// Every table a file holds, the ones SQLite keeps for itself excepted.
pub async fn table_names(connection: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    named_objects(connection, "WHERE type = 'table' AND name NOT LIKE 'sqlite%'").await
}

/// Every table and index a file holds, which is what tells a new one apart.
pub async fn object_names(connection: &DatabaseConnection) -> Result<Vec<String>, DbErr> {
    named_objects(connection, "WHERE name NOT LIKE 'sqlite%'").await
}

/// The names `sqlite_master` holds, narrowed by one clause.
async fn named_objects(
    connection: &DatabaseConnection,
    clause: &str,
) -> Result<Vec<String>, DbErr> {
    let rows = connection
        .query_all(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("SELECT name FROM sqlite_master {clause} ORDER BY name"),
        ))
        .await?;
    rows.iter().map(|row| row.try_get_by_index::<String>(0)).collect()
}

/// Give a file the tables it lacks, and mark it with the current version.
pub async fn create_the_schema(connection: &DatabaseConnection) -> Result<(), DbErr> {
    let existing = object_names(connection).await?;
    for statement in TABLES.iter().chain(INDEXES.iter()) {
        let wanted = created_object(statement);
        if existing.iter().any(|name| name == wanted) {
            continue;
        }
        connection
            .execute(Statement::from_string(
                DatabaseBackend::Sqlite,
                (*statement).to_string(),
            ))
            .await?;
    }
    connection
        .execute(Statement::from_string(
            DatabaseBackend::Sqlite,
            format!("PRAGMA user_version = {SCHEMA_VERSION}"),
        ))
        .await?;
    Ok(())
}

/// The name of the table or index one statement creates.
fn created_object(statement: &str) -> &str {
    statement
        .split_whitespace()
        .nth(2)
        .unwrap_or("")
        .trim_end_matches('(')
}
