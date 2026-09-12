//! Everything the web layer sits on.
//!
//! Nothing here knows about Axum: `application` imports from this module, never
//! the other way round. The modules are layered so the imports only ever go one
//! way:
//!
//! ```text
//! config      files and environment variables
//! story       the book, read off disk into memory
//! models      the shape of what is stored, and nothing else
//! db          the database, on top of config and models
//! ```
//!
//! Only the reading history, the play-throughs and the flagged pages are stored,
//! in one SQLite file per environment. The book is static, so it is read once at
//! startup rather than kept in a database that would have to be loaded first.

pub mod character;
pub mod combat;
pub mod config;
pub mod db;
pub mod dice;
pub mod inventory;
pub mod logs;
pub mod models;
pub mod story;
