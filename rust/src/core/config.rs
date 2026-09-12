//! Where a run gets its settings: `.env`, and which database.
//!
//! The bottom of the module tree: this module knows about files and environment
//! variables, and about nothing else here.

use std::path::{Path, PathBuf};
use std::sync::Once;

use rand::Rng as _;

/// Base database name; the database actually used is this name plus the current
/// environment suffix (see [`current_db_name`]). `APP_ENV` picks which SQLite
/// file is used, so a dev run never touches prod data, and vice versa.
pub const DB_NAME: &str = "haute_tension";
pub const ENV_NAMES: [&str; 2] = ["dev", "prod"];
pub const DEFAULT_ENV: &str = "dev";

/// The directory the SQLite files live in, from the environment. A relative path
/// is taken from the repository root, so `data` in `.env` means `<root>/data`.
pub const DATABASE_DIR_VAR: &str = "DATABASE_DIR";
pub const DATABASE_SUFFIX: &str = ".sqlite3";

/// The environment is wrong: a variable missing, or set to something unusable.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct EnvironmentError(pub String);

/// The repository root, which a relative path in `.env` is taken from.
///
/// The crate sits one directory below it, the way `haute_tension/core/` sat two
/// below it in the Python package.
pub fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the crate is a directory of the repository")
}

/// Read `.env` into the environment, once per process.
///
/// Called by everything that reads a setting, so importing a module never needs
/// a file: an absent `.env` leaves the environment exactly as it was.
pub fn load_dotenv() {
    static LOADED: Once = Once::new();
    LOADED.call_once(|| {
        let _ = dotenvy::from_path(root().join(".env"));
    });
}

/// One environment variable, empty when it is unset.
fn variable(name: &str) -> String {
    load_dotenv();
    std::env::var(name).unwrap_or_default()
}

/// Read required environment variables, refusing to run without them.
///
/// # Arguments
///
/// * `required_vars` - Names every caller of this run needs set.
///
/// # Returns
///
/// The variables and their values.
///
/// # Errors
///
/// If any of them is missing or empty.
pub fn load_env(required_vars: &[&str]) -> Result<Vec<(String, String)>, EnvironmentError> {
    let found: Vec<(String, String)> = required_vars
        .iter()
        .map(|name| ((*name).to_string(), variable(name)))
        .collect();
    let missing: Vec<&str> = found
        .iter()
        .filter(|(_, value)| value.is_empty())
        .map(|(name, _)| name.as_str())
        .collect();
    if !missing.is_empty() {
        return Err(EnvironmentError(format!(
            "Missing required environment variables: {}. \
             Copy .env.example to .env and fill it in.",
            missing.join(", ")
        )));
    }
    Ok(found)
}

/// Return the environment this run works in.
///
/// # Returns
///
/// Either `"dev"` or `"prod"`.
///
/// # Errors
///
/// If `APP_ENV` is set to anything else.
pub fn current_env() -> Result<String, EnvironmentError> {
    let asked = variable("APP_ENV");
    let env_name = if asked.trim().is_empty() {
        DEFAULT_ENV.to_string()
    } else {
        asked.trim().to_lowercase()
    };
    if !ENV_NAMES.contains(&env_name.as_str()) {
        let mut known = ENV_NAMES;
        known.sort_unstable();
        return Err(EnvironmentError(format!(
            "APP_ENV must be one of ['{}'], got '{env_name}'",
            known.join("', '")
        )));
    }
    Ok(env_name)
}

/// Return the name of the database this run reads and writes.
pub fn current_db_name() -> Result<String, EnvironmentError> {
    Ok(format!("{DB_NAME}_{}", current_env()?))
}

/// Return the SQLite file this run reads and writes.
///
/// `DATABASE_DIR` says where the files live; `APP_ENV` picks which one, so a dev
/// run and a prod run on the same directory never open the same file.
///
/// # Returns
///
/// `<DATABASE_DIR>/haute_tension_<env>.sqlite3`, or nothing when `DATABASE_DIR`
/// is unset — there is then no database to reach.
pub fn database_path() -> Result<Option<PathBuf>, EnvironmentError> {
    let directory = variable(DATABASE_DIR_VAR).trim().to_string();
    if directory.is_empty() {
        return Ok(None);
    }
    let name = format!("{}{DATABASE_SUFFIX}", current_db_name()?);
    Ok(Some(root().join(expanded(&directory)).join(name)))
}

/// A path with a leading `~` replaced by the home directory.
fn expanded(directory: &str) -> PathBuf {
    let Some(rest) = directory.strip_prefix('~') else {
        return PathBuf::from(directory);
    };
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return PathBuf::from(directory);
    };
    home.join(rest.trim_start_matches('/'))
}

/// Return the key the session cookie is signed with.
///
/// The cookie carries a game id and nothing else, so a forged one costs a
/// stranger's play-through rather than an account. A generated key is therefore
/// good enough for a dev run, and only bad in production — where it would also
/// log every reader out at each restart, which is the visible half of the
/// problem.
///
/// # Returns
///
/// `FLASK_SECRET_KEY`, or a fresh random key when it is unset.
pub fn session_secret() -> String {
    let configured = variable("FLASK_SECRET_KEY");
    if !configured.is_empty() {
        return configured;
    }
    let mut generator = rand::rng();
    (0..32)
        .map(|_| format!("{:02x}", generator.random::<u8>()))
        .collect()
}
