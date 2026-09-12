//! What a reader is shown when the database cannot answer.
//!
//! One answer, built where a handler gives up, so that no handler has to think
//! about it. A page that needs the database and cannot reach it **fails**: it is
//! not served half-built. The reader gets a page saying so, and the log gets the
//! reason.
//!
//! `/data/<number>` answers JSON rather than HTML, because that is what its
//! callers parse; so does any request that asks for JSON first, which is how the
//! page's own script flags a page. The status is the same.
//!
//! **503, not 500.** Nothing is wrong with the request or with the application:
//! the database is not there, and the same request will work once it is.

use askama::Template;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::application::templates::{DatabaseErrorTemplate, Layout};
use crate::core::db::{DatabaseFailure, StatusError};

pub const DATABASE_UNAVAILABLE: StatusCode = StatusCode::SERVICE_UNAVAILABLE;

/// What the Werkzeug page said, so a refusal reads the same in the log.
const NOT_FOUND_BODY: &str = "<!doctype html>\n\
<html lang=en>\n\
<title>404 Not Found</title>\n\
<h1>Not Found</h1>\n\
<p>The requested URL was not found on the server. If you entered the URL manually \
please check your spelling and try again.</p>\n";

/// Why a handler could not answer.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The database could not answer, and the page needed it.
    #[error(transparent)]
    Database(#[from] DatabaseFailure),
    /// The book does not have that page, or the id names nothing.
    #[error("not found")]
    NotFound,
    /// A page could not be built out of what it was handed.
    #[error("the page could not be rendered: {0}")]
    Render(#[from] askama::Error),
}

impl From<StatusError> for AppError {
    fn from(trouble: StatusError) -> Self {
        match trouble {
            StatusError::Database(failure) => AppError::Database(failure),
            // A status neither open nor resolved never reaches a handler: the
            // two that set one name a constant.
            StatusError::Unknown(_) => AppError::NotFound,
        }
    }
}

/// What a failure is answered with, once the handler has given up.
///
/// The answer is built here and the shape is chosen by the request: JSON under
/// the API routes and for a caller that asks for it, a page otherwise.
pub struct Refusal {
    pub status: StatusCode,
    pub content_type: &'static str,
    pub body: String,
    /// What the database said, for the one failure the trace reports itself.
    pub database_trouble: Option<DatabaseTrouble>,
}

/// The database failure behind a refusal, as the log writes it.
///
/// Carried on the answer rather than logged here: the line names the method,
/// the address and the route, and the trace is what knows those.
#[derive(Clone, Debug)]
pub struct DatabaseTrouble {
    pub kind: &'static str,
    pub message: String,
}

impl DatabaseTrouble {
    /// The trouble as the traceback under the line.
    pub fn written(&self) -> String {
        format!("{}: {}", self.kind, self.message)
    }
}

impl IntoResponse for Refusal {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            [(header::CONTENT_TYPE, self.content_type)],
            self.body,
        )
            .into_response();
        if let Some(trouble) = self.database_trouble {
            response.extensions_mut().insert(trouble);
        }
        response
    }
}

/// Report a failure the way the caller reads answers.
///
/// # Arguments
///
/// * `trouble` - What the handler gave up on.
/// * `wants_json` - Whether the caller parses answers rather than reading them:
///   the API blueprint, or a request that put JSON first in what it accepts.
pub fn refusal(trouble: &AppError, wants_json: bool) -> Refusal {
    match trouble {
        AppError::NotFound => Refusal {
            status: StatusCode::NOT_FOUND,
            content_type: "text/html; charset=utf-8",
            body: NOT_FOUND_BODY.to_string(),
            database_trouble: None,
        },
        AppError::Render(_) => Refusal {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            content_type: "text/html; charset=utf-8",
            body: "<!doctype html>\n<html lang=en>\n<title>500 Internal Server Error</title>\n"
                .to_string(),
            database_trouble: None,
        },
        AppError::Database(failure) if wants_json => Refusal {
            status: DATABASE_UNAVAILABLE,
            content_type: "application/json",
            body: serde_json::json!({
                "success": false,
                "message": "la base de données est injoignable",
            })
            .to_string(),
            database_trouble: Some(trouble_of(failure)),
        },
        AppError::Database(failure) => Refusal {
            status: DATABASE_UNAVAILABLE,
            content_type: "text/html; charset=utf-8",
            body: DatabaseErrorTemplate {
                layout: Layout::new("Base de données injoignable · Haute Tension", "/", Vec::new()),
            }
            .render()
            .unwrap_or_else(|_| "La base de données ne répond pas.".to_string()),
            database_trouble: Some(trouble_of(failure)),
        },
    }
}

/// What the database said, named the way the log names it.
fn trouble_of(failure: &DatabaseFailure) -> DatabaseTrouble {
    DatabaseTrouble {
        kind: failure.kind(),
        message: failure.to_string(),
    }
}

/// Whether the request put JSON first in what it accepts.
pub fn asks_for_json(headers: &HeaderMap) -> bool {
    best_mimetype(headers).as_deref() == Some("application/json")
}

/// The media type a request puts first, quality taken into account.
///
/// A request that names none puts nothing first, which is what a browser sending
/// no `Accept` header does — and what makes it a reader rather than a script.
pub fn best_mimetype(headers: &HeaderMap) -> Option<String> {
    let accept = headers.get(header::ACCEPT)?.to_str().ok()?;
    let mut best: Option<(f32, String)> = None;
    for entry in accept.split(',') {
        let mut parts = entry.split(';');
        let Some(mimetype) = parts.next().map(str::trim).filter(|kind| !kind.is_empty()) else {
            continue;
        };
        let quality = parts
            .find_map(|parameter| parameter.trim().strip_prefix("q=").map(str::to_string))
            .and_then(|quality| quality.parse::<f32>().ok())
            .unwrap_or(1.0);
        if best.as_ref().is_none_or(|(top, _)| quality > *top) {
            best = Some((quality, mimetype.to_string()));
        }
    }
    best.map(|(_, mimetype)| mimetype)
}
