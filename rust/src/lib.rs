//! An Axum adaptation of *La Forteresse d'Alamuth*.
//!
//! Two halves, layered and unconnected at runtime: [`application`], the web
//! layer — the router, the handlers, the templates — and [`core`], everything
//! below it, which knows nothing about HTTP. The imports only go one way.
//!
//! Only the reading history, the play-throughs and the flagged pages are stored.
//! The book itself is static, and is read off disk into memory at startup.

pub mod application;
pub mod core;
pub mod escape;
pub mod pipeline;
