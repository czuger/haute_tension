//! The general log: what it writes, and what it must never write.
//!
//! Ported from `tests/test_core_logs.py`. The lines are read through the
//! capture and not off the disk: `logs/general.log` is the same file for the
//! whole run, and a test that read it would be reading every other test's trace
//! as well. Every test here takes the log lock, because the capture belongs to
//! the process.

// The environment and the log belong to the process, so the lock that
// serialises the tests reading them is held for the length of each one.
#![allow(clippy::await_holding_lock)]

#[path = "common/mod.rs"]
mod common;

use common::{clean_env, env_lock, log_lock, EnvVar};
use haute_tension::core::logs::general_log::{
    cut, current_value_limit, event, failure, general_log_path, hidden, is_a_secret,
    is_an_identifier, note, readable, sanitised, set_value_limit, shortened, shown, spell_out,
    start_capture, take_capture, the_level, the_limit, without_the_secrets, Level, Record,
    DEFAULT_LEVEL, DEFAULT_VALUE_LIMIT, IDENTIFIER_LENGTH,
};
use haute_tension::core::logs::rotating_log::{RotatingLog, MAX_BYTES};
use haute_tension::core::models::hybrid_document::{Field, RowDict};
use haute_tension::{row, vars};

/// The whole log as one text, the way `caplog.text` reads.
fn text_of(records: &[Record]) -> String {
    records
        .iter()
        .map(Record::text)
        .collect::<Vec<String>>()
        .join("\n")
}

/// The two ways in, and the shape of what they write.
mod writing_a_line {
    use super::*;

    #[test]
    fn a_note_is_a_step() {
        let _lock = log_lock();
        start_capture();

        note("Combat armed", vars! { "page" => "22" });

        assert_eq!(take_capture()[0].level, Level::Debug);
    }

    #[test]
    fn an_event_is_something_that_happened() {
        let _lock = log_lock();
        start_capture();

        event("Hero rolled up", vars! { "force" => 15_i64 });

        assert_eq!(take_capture()[0].level, Level::Info);
    }

    #[test]
    fn a_failure_is_an_error() {
        let _lock = log_lock();
        start_capture();

        failure("It broke", None, Vec::new());

        assert_eq!(take_capture()[0].level, Level::Error);
    }

    #[test]
    fn a_failure_carries_what_went_wrong() {
        let _lock = log_lock();
        start_capture();

        failure(
            "The database refused",
            Some("ValueError: no reachable servers"),
            Vec::new(),
        );

        let written = text_of(&take_capture());
        assert!(written.contains("ValueError"));
        assert!(written.contains("no reachable servers"));
    }

    #[test]
    fn a_failure_without_a_trouble_still_writes() {
        let _lock = log_lock();
        start_capture();

        failure("Nothing was raised", None, vars! { "page" => "22" });

        assert!(text_of(&take_capture()).contains("Nothing was raised"));
    }

    #[test]
    fn a_message_alone_is_written_alone() {
        assert_eq!(spell_out("Application built", &[]), "Application built");
    }

    #[test]
    fn variables_follow_an_em_dash() {
        assert_eq!(
            spell_out("Combat armed", &vars! { "page" => "22", "enemies" => 1_i64 }),
            "Combat armed — page='22', enemies=1"
        );
    }

    #[test]
    fn the_order_given_is_the_order_written() {
        let line = spell_out("x", &vars! { "b" => 1_i64, "a" => 2_i64 });

        assert!(line.find("b=") < line.find("a="));
    }
}

/// What the log must never write.
mod secrets {
    use super::*;

    #[test]
    fn every_secret_name_is_recognised() {
        for name in [
            "secret",
            "token",
            "password",
            "authorization",
            "cookie",
            "session",
            "key",
        ] {
            assert!(is_a_secret(name), "for {name}");
        }
    }

    #[test]
    fn a_name_that_merely_contains_one_is_recognised() {
        assert!(is_a_secret("FLASK_SECRET_KEY"));
        assert!(is_a_secret("api_token_header"));
    }

    #[test]
    fn an_ordinary_name_is_not() {
        assert!(!is_a_secret("page"));
        assert!(!is_a_secret("force"));
    }

    #[test]
    fn a_secret_is_written_as_its_length() {
        assert_eq!(
            shown("secret_key", &Field::Str("abcdef".to_string())),
            "<hidden, 6 characters>"
        );
    }

    #[test]
    fn an_absent_secret_says_so() {
        assert_eq!(shown("token", &Field::Null), "<absent>");
        assert_eq!(hidden(&Field::Null), "<absent>");
    }

    #[test]
    fn a_secret_never_appears_in_the_line() {
        let _lock = log_lock();
        start_capture();

        note(
            "Session opened",
            vars! { "session_token" => "hunter2-and-then-some" },
        );

        let written = text_of(&take_capture());
        assert!(!written.contains("hunter2"));
        assert!(written.contains("<hidden, 21 characters>"));
    }

    #[test]
    fn a_secret_buried_in_a_body_is_hidden_too() {
        let body = Field::Dict(row! {
            "page" => "22",
            "auth" => Field::Dict(row! { "token" => "abc123" }),
        });

        let cleaned = sanitised(&body);

        let cleaned = cleaned.as_dict().expect("a dict comes back a dict");
        assert_eq!(
            cleaned["auth"].as_dict().expect("a dict")["token"],
            Field::Str("<hidden, 6 characters>".to_string())
        );
        assert_eq!(cleaned["page"], Field::Str("22".to_string()));
    }

    #[test]
    fn a_secret_inside_a_list_is_hidden() {
        let body = Field::List(vec![
            Field::Dict(row! { "password" => "abc" }),
            Field::Dict(row! { "page" => "1" }),
        ]);

        let cleaned = sanitised(&body);

        let entries = cleaned.as_list().expect("a list comes back a list");
        assert_eq!(
            entries[0].as_dict().expect("a dict")["password"],
            Field::Str("<hidden, 3 characters>".to_string())
        );
    }

    /// A sequence comes back a sequence, whatever was walked into it.
    #[test]
    fn a_list_comes_back_a_list() {
        let walked = sanitised(&Field::List(vec![
            Field::Str("a".to_string()),
            Field::Str("b".to_string()),
        ]));

        assert_eq!(
            walked,
            Field::List(vec![
                Field::Str("a".to_string()),
                Field::Str("b".to_string()),
            ])
        );
    }

    #[test]
    fn a_plain_value_is_left_alone() {
        assert_eq!(sanitised(&Field::Int(3)), Field::Int(3));
    }

    #[test]
    fn a_body_logged_whole_is_scrubbed() {
        let _lock = log_lock();
        start_capture();

        note(
            "Answer",
            vars! {
                "body" => Field::Dict(row! {
                    "cookie" => "session=abc",
                    "status" => 200_i64,
                }),
            },
        );

        let written = text_of(&take_capture());
        assert!(!written.contains("session=abc"));
        assert!(written.contains("200"));
    }
}

/// A game id is a credential here, and is written in part only.
mod play_through_ids {
    use super::*;

    #[test]
    fn a_game_field_is_an_identifier() {
        assert!(is_an_identifier("game"));
        assert!(is_an_identifier("game_id"));
        assert!(!is_an_identifier("page"));
    }

    #[test]
    fn only_its_head_is_written() {
        let written = shortened(&Field::Str("4f2a91c0deadbeefcafe".to_string()));

        assert_eq!(written, "'4f2a91c0…'");
        assert!(!written.contains("deadbeef"));
    }

    #[test]
    fn a_short_one_is_written_whole() {
        assert_eq!(shortened(&Field::Str("abc".to_string())), "'abc'");
    }

    #[test]
    fn an_absent_one_says_so() {
        assert_eq!(shortened(&Field::Null), "<absent>");
    }

    #[test]
    fn the_head_is_long_enough_to_tell_two_apart() {
        const { assert!(IDENTIFIER_LENGTH >= 8) };
    }

    #[test]
    fn it_never_appears_whole_in_a_line() {
        let _lock = log_lock();
        start_capture();

        note(
            "Combat armed",
            vars! { "game" => "4f2a91c0deadbeefcafe", "page" => "22" },
        );

        let written = text_of(&take_capture());
        assert!(!written.contains("deadbeefcafe"));
        assert!(written.contains("4f2a91c0"));
    }
}

/// How a value is written so it reads back.
mod how_a_value_reads {
    use super::*;

    #[test]
    fn a_string_keeps_its_quotes() {
        assert_eq!(readable(&Field::Str("22".to_string())), "'22'");
    }

    /// Without the quotes it would read as nothing at all.
    #[test]
    fn an_empty_string_is_visible() {
        assert_eq!(readable(&Field::Str(String::new())), "''");
    }

    #[test]
    fn a_trailing_space_is_visible() {
        assert_eq!(readable(&Field::Str("22 ".to_string())), "'22 '");
    }

    #[test]
    fn a_number_is_written_plainly() {
        assert_eq!(readable(&Field::Int(3)), "3");
        assert_eq!(readable(&Field::Float(3.5)), "3.5");
    }

    #[test]
    fn a_boolean_is_written_plainly() {
        assert_eq!(readable(&Field::Bool(true)), "True");
    }

    #[test]
    fn nothing_is_written_as_none() {
        assert_eq!(readable(&Field::Null), "None");
    }

    #[test]
    fn a_structure_is_written_as_json() {
        assert_eq!(
            readable(&Field::Dict(row! { "page" => "22" })),
            r#"{"page": "22"}"#
        );
    }

    #[test]
    fn accents_survive() {
        assert!(readable(&Field::Dict(row! { "message" => "le numéro" })).contains("numéro"));
    }

    #[test]
    fn something_json_refuses_falls_back_to_str() {
        let day = chrono::NaiveDate::from_ymd_opt(2026, 9, 9).expect("a real day");

        assert!(readable(&Field::Dict(row! { "day" => Field::Date(day) })).contains("2026"));
    }

    /// JSON has no way of writing a number that is not one; the log still does.
    #[test]
    fn something_nothing_can_write_falls_back_to_its_own_description() {
        let awkward = Field::Dict(row! { "share" => Field::Float(f64::NAN) });

        assert_eq!(readable(&awkward), "{'share': nan}");
    }
}

/// What is too long to be worth a whole file.
mod cutting {
    use super::*;

    #[test]
    fn a_short_value_is_left_alone() {
        let _lock = log_lock();

        assert_eq!(cut("abc"), "abc");
    }

    #[test]
    fn a_long_value_says_what_was_cut() {
        let _lock = log_lock();
        let was = current_value_limit();
        set_value_limit(10);

        let written = cut(&"x".repeat(50));

        set_value_limit(was);
        assert!(written.starts_with(&"x".repeat(10)));
        assert!(written.contains("50 characters in all"));
    }

    #[test]
    fn a_limit_of_zero_writes_everything() {
        let _lock = log_lock();
        let was = current_value_limit();
        set_value_limit(0);

        let written = cut(&"x".repeat(5000));

        set_value_limit(was);
        assert_eq!(written, "x".repeat(5000));
    }
}

/// What the environment may say about the log.
mod configuration {
    use super::*;

    #[test]
    fn debug_is_the_default() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _unset = EnvVar::unset("LOG_LEVEL");

        assert_eq!(the_level(), DEFAULT_LEVEL);
        assert_eq!(DEFAULT_LEVEL, Level::Debug);
    }

    #[test]
    fn a_named_level_is_read() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("LOG_LEVEL", "warning");

        assert_eq!(the_level(), Level::Warning);
    }

    /// A mistyped level must not cost a start-up.
    #[test]
    fn an_unreadable_level_falls_back_rather_than_stopping() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("LOG_LEVEL", "chatty");

        assert_eq!(the_level(), DEFAULT_LEVEL);
    }

    #[test]
    fn the_limit_is_read_from_the_environment() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("LOG_VALUE_LIMIT", "50");

        assert_eq!(the_limit(), 50);
    }

    #[test]
    fn an_unreadable_limit_falls_back() {
        let _lock = env_lock();
        let _clean = clean_env();
        let _set = EnvVar::set("LOG_VALUE_LIMIT", "lots");

        assert_eq!(the_limit(), DEFAULT_VALUE_LIMIT);
    }
}

/// A URL as a log may carry it.
mod urls {
    use super::*;

    #[test]
    fn a_url_without_a_query_is_untouched() {
        assert_eq!(without_the_secrets("/book/22"), "/book/22");
    }

    #[test]
    fn an_ordinary_parameter_is_kept() {
        assert_eq!(without_the_secrets("/game?rolled=1"), "/game?rolled=1");
    }

    #[test]
    fn a_secret_parameter_is_hidden() {
        let written = without_the_secrets("/x?page=22&token=abcdef");

        assert!(!written.contains("abcdef"));
        assert!(written.contains("page=22"));
    }
}

/// The file the log is written into.
mod the_rotating_file {
    use super::*;

    /// `logs/` is not versioned, so a fresh clone has none.
    #[test]
    fn the_directory_is_made_if_it_is_missing() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = directory.path().join("nowhere").join("general.log");

        let log = RotatingLog::open(&path, MAX_BYTES, 5);

        assert!(log.path().parent().is_some_and(std::path::Path::is_dir));
    }

    #[test]
    fn the_line_carries_its_time() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let log = RotatingLog::open(directory.path().join("general.log"), MAX_BYTES, 5);

        let written = log.format("a line");

        assert!(written.ends_with("  a line"));
    }

    #[test]
    fn it_rotates_by_size() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let path = directory.path().join("general.log");
        let mut log = RotatingLog::open(&path, 200, 2);

        for _ in 0..40 {
            log.emit(&"x".repeat(50));
        }

        assert!(path.exists());
        assert!(directory.path().join("general.log.1").exists());
        assert!(!directory.path().join("general.log.3").exists());
    }

    #[test]
    fn the_default_size_is_generous_enough_for_a_run() {
        const { assert!(MAX_BYTES >= 100 * 1024) };
    }

    /// `logs/` going under a running server must not cost every later line.
    ///
    /// The handler reopens the file at every rotation, so making the directory
    /// once, before handing it over, is not enough.
    #[test]
    fn a_directory_that_disappears_is_remade() {
        let root = tempfile::tempdir().expect("a temporary directory is always made");
        let directory = root.path().join("logs");
        let path = directory.join("general.log");
        let mut log = RotatingLog::open(&path, 200, 2);
        log.emit("before");
        std::fs::remove_dir_all(&directory).expect("the directory is removed");

        for _ in 0..20 {
            log.emit(&"x".repeat(50));
        }

        assert!(directory.is_dir());
        assert!(path.exists());
        assert!(std::fs::read_to_string(&path)
            .expect("the file is readable")
            .contains('x'));
    }

    /// A lost log line must cost nothing beyond that line.
    #[test]
    fn nothing_is_reported_when_it_disappears() {
        let root = tempfile::tempdir().expect("a temporary directory is always made");
        let directory = root.path().join("logs");
        let mut log = RotatingLog::open(directory.join("general.log"), 200, 5);
        log.emit("before");
        std::fs::remove_dir_all(&directory).expect("the directory is removed");

        for _ in 0..20 {
            log.emit(&"x".repeat(50));
        }

        assert!(directory.is_dir());
    }

    #[test]
    fn the_directory_is_made_on_the_first_open_too() {
        let directory = tempfile::tempdir().expect("a temporary directory is always made");
        let deeper = directory.path().join("deeper").join("still");

        RotatingLog::open(deeper.join("general.log"), MAX_BYTES, 5);

        assert!(deeper.is_dir());
    }
}

/// Giving the log its file and its level.
mod setting_up {
    use super::*;

    #[test]
    fn the_log_has_a_file() {
        let _lock = log_lock();
        start_capture();

        note("Application built", Vec::new());

        assert_eq!(take_capture().len(), 1);
        assert!(general_log_path().exists());
    }

    /// A file opened twice would otherwise write every line twice.
    #[test]
    fn one_line_is_written_once() {
        let _lock = log_lock();
        start_capture();

        note("Combat armed", vars! { "page" => "22" });
        note("Combat armed", vars! { "page" => "23" });

        let records = take_capture();
        assert_eq!(records.len(), 2);
        assert_ne!(records[0].message, records[1].message);
    }

    #[test]
    fn it_writes_into_the_logs_directory() {
        let path = general_log_path();

        assert_eq!(
            path.parent().and_then(std::path::Path::file_name),
            Some(std::ffi::OsStr::new("logs"))
        );
        assert_eq!(path.file_name(), Some(std::ffi::OsStr::new("general.log")));
    }
}

/// The dict a line's variables are built from.
mod the_variables {
    use super::*;

    #[test]
    fn a_row_is_a_dict_of_fields() {
        let written: RowDict = row! { "page" => "22" };

        assert_eq!(written["page"], Field::Str("22".to_string()));
    }
}
