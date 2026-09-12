//! Tests for how a run picks its database.
//!
//! Every test here takes the environment lock: the variables belong to the
//! process, so two tests changing them at once would read each other's.

// The environment and the log belong to the process, so the lock that
// serialises the tests reading them is held for the length of each one.
#![allow(clippy::await_holding_lock)]

#[path = "common/mod.rs"]
mod common;

use common::{clean_env, env_lock, EnvVar};
use haute_tension::core::config::{
    current_db_name, current_env, database_path, load_env, root,
};

/// Which environment a run works in.
mod the_current_env {
    use super::*;

    #[test]
    fn dev_is_the_default() {
        let _lock = env_lock();
        let _clean = clean_env();

        assert_eq!(current_env().expect("dev is a known environment"), "dev");
    }

    #[test]
    fn a_known_env_is_accepted() {
        let _lock = env_lock();
        let _clean = clean_env();

        for name in ["dev", "prod"] {
            let _set = EnvVar::set("APP_ENV", name);

            assert_eq!(current_env().expect("both are known"), name);
        }
    }

    #[test]
    fn case_and_padding_are_ignored() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("APP_ENV", "  PROD ");

        assert_eq!(current_env().expect("prod is a known environment"), "prod");
    }

    #[test]
    fn an_unknown_env_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("APP_ENV", "staging");

        let trouble = current_env().expect_err("staging is not an environment");

        assert!(trouble.to_string().contains("APP_ENV must be one of"));
    }
}

/// The database name a run reads and writes.
mod the_current_db_name {
    use super::*;

    #[test]
    fn the_env_suffixes_the_database_name() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("APP_ENV", "prod");

        assert_eq!(
            current_db_name().expect("prod is a known environment"),
            "haute_tension_prod"
        );
    }

    #[test]
    fn dev_and_prod_are_separate_databases() {
        let _lock = env_lock();
        let _clean = clean_env();
        let dev = {
            let _set = EnvVar::set("APP_ENV", "dev");
            current_db_name().expect("dev is a known environment")
        };
        let _set = EnvVar::set("APP_ENV", "prod");

        assert_ne!(dev, current_db_name().expect("prod is a known environment"));
    }
}

/// Refusing to run without the variables a script needs.
mod loading_the_env {
    use super::*;

    #[test]
    fn set_variables_are_returned() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("DATABASE_DIR", "data");

        assert_eq!(
            load_env(&["DATABASE_DIR"]).expect("it is set"),
            vec![("DATABASE_DIR".to_string(), "data".to_string())]
        );
    }

    #[test]
    fn nothing_required_is_nothing_to_check() {
        let _lock = env_lock();
        let _clean = clean_env();

        assert!(load_env(&[]).expect("nothing to check").is_empty());
    }

    #[test]
    fn a_missing_variable_is_refused() {
        let _lock = env_lock();
        let _clean = clean_env();

        let trouble = load_env(&["DATABASE_DIR"]).expect_err("it is unset");

        assert!(trouble.to_string().contains("DATABASE_DIR"));
    }

    #[test]
    fn an_empty_variable_counts_as_missing() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("DATABASE_DIR", "");

        let trouble = load_env(&["DATABASE_DIR"]).expect_err("empty counts as missing");

        assert!(trouble.to_string().contains("DATABASE_DIR"));
    }

    #[test]
    fn every_missing_variable_is_named() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _also = EnvVar::unset("APP_SECRET");

        let trouble =
            load_env(&["DATABASE_DIR", "APP_SECRET"]).expect_err("both are unset");

        assert!(trouble.to_string().contains("DATABASE_DIR"));
        assert!(trouble.to_string().contains("APP_SECRET"));
    }
}

/// Which SQLite file a run opens.
mod the_database_path {
    use super::*;

    #[test]
    fn the_file_is_named_after_the_env() {
        let _lock = env_lock();
        let _clean = clean_env();
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let _dir = EnvVar::set("DATABASE_DIR", &directory.path().display().to_string());
        let _env = EnvVar::set("APP_ENV", "prod");

        assert_eq!(
            database_path().expect("prod is a known environment"),
            Some(directory.path().join("haute_tension_prod.sqlite3"))
        );
    }

    #[test]
    fn a_relative_directory_is_taken_from_the_repository_root() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _dir = EnvVar::set("DATABASE_DIR", "data");

        assert_eq!(
            database_path().expect("dev is a known environment"),
            Some(root().join("data").join("haute_tension_dev.sqlite3"))
        );
    }

    #[test]
    fn a_home_directory_is_expanded() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _dir = EnvVar::set("DATABASE_DIR", "~/haute_tension_data");

        let path = database_path()
            .expect("dev is a known environment")
            .expect("a directory is configured");

        assert!(!path.display().to_string().contains('~'));
        assert!(path.is_absolute());
    }

    #[test]
    fn dev_and_prod_are_separate_files() {
        let _lock = env_lock();
        let _clean = clean_env();
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let _dir = EnvVar::set("DATABASE_DIR", &directory.path().display().to_string());
        let dev = {
            let _env = EnvVar::set("APP_ENV", "dev");
            database_path().expect("dev is known").expect("configured")
        };
        let _env = EnvVar::set("APP_ENV", "prod");
        let prod = database_path().expect("prod is known").expect("configured");

        assert_ne!(dev, prod);
        assert_eq!(dev.parent(), prod.parent());
    }

    #[test]
    fn nothing_configured_is_no_path() {
        let _lock = env_lock();
        let _clean = clean_env();

        assert_eq!(database_path().expect("dev is a known environment"), None);
    }

    #[test]
    fn a_blank_directory_counts_as_unset() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _dir = EnvVar::set("DATABASE_DIR", "   ");

        assert_eq!(database_path().expect("dev is a known environment"), None);
    }
}
