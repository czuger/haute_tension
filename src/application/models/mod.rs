//! The shapes the routes and templates read, and the routes alone.
//!
//! `core::db` converts a row into one of these on the way out, so nothing above
//! it ever holds a stored row.

pub mod game;
pub mod inspection;
pub mod story_page;
