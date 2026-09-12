//! The HTTP half of the log: every request written into the general log.
//!
//! The log itself is `core::logs`; only [`request_trace`] lives here, because
//! only it knows what a request is.

pub mod request_trace;
