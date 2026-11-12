//! The web layer: the router, the handlers, the templates and the error pages.
//!
//! Nothing below this knows what a request is. A handler calls one `core::db`
//! function and renders; the failures propagate to the one handler in
//! [`errors`], which answers 503.

pub mod errors;
pub mod factory;
pub mod french_date;
pub mod listener;
pub mod logs;
pub mod models;
pub mod routes;
pub mod session;
pub mod templates;
