//! The log: one file, and the two ways of writing into it.
//!
//! - [`rotating_log`] opens a log file that rotates, on a directory it makes
//!   again at every open;
//! - [`general_log`] is the server's own trace, at DEBUG, in `logs/general.log`.
//!
//! It sits in `core` and not beside the routes because nothing here knows what a
//! request is, and because `core::db` writes to it: a log the database layer
//! could not reach would be a log missing everything the database layer does.
//! `application::logs::request_trace` is the HTTP half, and it is the only half
//! that knows what a request is.

pub mod general_log;
pub mod rotating_log;
