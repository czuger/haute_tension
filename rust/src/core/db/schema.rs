//! The tables, exactly as the schema describes them.
//!
//! Spelled out rather than generated. `migrations/001_integer_ids_items_and_stats.sql`
//! creates the same tables word for word, and `tests/test_migrations.rs` checks
//! that a migrated file ends with exactly the schema a new file gets — comparing
//! the SQL `sqlite_master` holds, character for character. A builder that emitted
//! its own spacing would fail that comparison without anything being wrong, so
//! the statements are the contract and live here.

/// The schema the models describe, as `PRAGMA user_version` records it in a file.
///
/// Raised by one with every script added to `migrations/`, each of which moves a
/// file from the version before to its own.
pub const SCHEMA_VERSION: i32 = 1;

/// Every table, in the order a new file is given them.
pub const TABLES: [&str; 4] = [
    "CREATE TABLE games (\n\
     \tid INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \n\
     \tbook VARCHAR NOT NULL, \n\
     \tforce SMALLINT NOT NULL, \n\
     \tvie_max SMALLINT NOT NULL, \n\
     \tvie_actuelle SMALLINT NOT NULL, \n\
     \tgold SMALLINT NOT NULL, \n\
     \tdied_at VARCHAR(32), \n\
     \tcreated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tupdated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tdata JSON NOT NULL, \n\
     \tCONSTRAINT ck_games_force_not_negative CHECK (force >= 0), \n\
     \tCONSTRAINT ck_games_vie_max_not_negative CHECK (vie_max >= 0), \n\
     \tCONSTRAINT ck_games_vie_actuelle_not_negative CHECK (vie_actuelle >= 0), \n\
     \tCONSTRAINT ck_games_gold_not_negative CHECK (gold >= 0)\n\
     )",
    "CREATE TABLE items (\n\
     \tid INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \n\
     \tgame_id INTEGER NOT NULL, \n\
     \tcreated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tupdated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tdata JSON NOT NULL, \n\
     \tFOREIGN KEY(game_id) REFERENCES games (id)\n\
     )",
    "CREATE TABLE page_inspections (\n\
     \tid INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \n\
     \tbook VARCHAR NOT NULL, \n\
     \tpath VARCHAR NOT NULL, \n\
     \tstatus VARCHAR(16) NOT NULL, \n\
     \tcreated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tupdated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL, \n\
     \tdata JSON NOT NULL, \n\
     \tCONSTRAINT uq_page_inspections_book_path UNIQUE (book, path)\n\
     )",
    "CREATE TABLE page_views (\n\
     \tid INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT, \n\
     \tbook VARCHAR NOT NULL, \n\
     \tgame_id INTEGER, \n\
     \tviewed_at VARCHAR(32) NOT NULL, \n\
     \tdata JSON NOT NULL, \n\
     \tFOREIGN KEY(game_id) REFERENCES games (id)\n\
     )",
];

/// Every index, in the order a new file is given them.
pub const INDEXES: [&str; 7] = [
    "CREATE INDEX ix_games_book ON games (book)",
    "CREATE INDEX ix_games_book_died_at ON games (book, died_at)",
    "CREATE INDEX ix_items_game_id ON items (game_id)",
    "CREATE INDEX ix_page_inspections_book_updated_at ON page_inspections (book, updated_at)",
    "CREATE INDEX ix_page_inspections_status ON page_inspections (status)",
    "CREATE INDEX ix_page_views_book_viewed_at ON page_views (book, viewed_at)",
    "CREATE INDEX ix_page_views_game_id_viewed_at ON page_views (game_id, viewed_at)",
];
