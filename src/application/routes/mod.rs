//! The routes, and what they all share.
//!
//! A handler calls one `core::db` function and renders. It catches nothing:
//! a database that cannot answer leaves as an [`AppError`], which
//! `application::errors` turns into a 503 page — or into JSON under the API
//! routes, where that is what the caller parses.

pub mod api;
pub mod game;
pub mod inspection;
pub mod web;

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::application::errors::{asks_for_json, refusal, AppError};
use crate::application::models::story_page::StoryData;
use crate::application::session::Session;
use crate::application::templates::{Flash, Layout};
use crate::core::db::Db;
use crate::core::dice::Rng;

/// Everything a handler is given besides the request itself.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub story: Arc<StoryData>,
    pub book: String,
    pub book_series: String,
    pub book_title: String,
    /// Source of chance for character creation and combat, seeded by the tests.
    pub rng: Option<Arc<dyn Rng>>,
    pub secret: Arc<Vec<u8>>,
    /// The sub-path the site is served under, empty at the root of its host.
    pub prefix: String,
}

impl AppState {
    /// The dice this run throws, or none to leave it to chance.
    pub fn dice(&self) -> Option<&dyn Rng> {
        self.rng.as_deref()
    }
}

/// The request as every handler reads it: its session, its address, its shape.
pub struct Reader {
    pub session: Session,
    pub path: String,
    pub wants_json: bool,
    pub secret: Arc<Vec<u8>>,
    /// The sub-path the site is served under. The address above never carries it
    /// — the proxy strips it — so everything written back out adds it here.
    pub prefix: String,
}

impl Reader {
    /// Read the session and the address off one request.
    pub fn of(state: &AppState, headers: &HeaderMap, path: &str) -> Self {
        let cookie = headers
            .get(header::COOKIE)
            .and_then(|value| value.to_str().ok());
        Self {
            session: Session::decode(cookie, &state.secret),
            path: path.to_string(),
            wants_json: asks_for_json(headers),
            secret: state.secret.clone(),
            prefix: state.prefix.clone(),
        }
    }

    /// The header of the page being built, the messages waiting taken out of it.
    pub fn layout(&mut self, title: impl Into<String>) -> Layout {
        let flashes: Vec<Flash> = self.session.take_flashes();
        Layout::new(title, self.path.clone(), flashes).under(&self.prefix)
    }

    /// The answer, carrying the session when this request changed it.
    pub fn answer(&self, response: Response) -> Response {
        if !self.session.changed() {
            return response;
        }
        let mut response = response;
        if let Ok(cookie) = self.session.encode(&self.secret, &self.prefix).parse() {
            response.headers_mut().append(header::SET_COOKIE, cookie);
        }
        response
    }

    /// A rendered page, as the reader receives it.
    pub fn html(&self, body: String) -> Response {
        self.answer(
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                body,
            )
                .into_response(),
        )
    }

    /// A redirect, as every form here answers.
    ///
    /// The address is this application's own, so it is written under the
    /// sub-path the site is served under: a `Location` without it would send the
    /// reader out of the site.
    pub fn redirect(&self, location: &str) -> Response {
        let location = format!("{}{location}", self.prefix);
        self.answer(
            (
                StatusCode::FOUND,
                [(header::LOCATION, location)],
                String::new(),
            )
                .into_response(),
        )
    }

    /// What a handler gives up with, answered the way this caller reads answers.
    pub fn refuse(&self, trouble: &AppError) -> Response {
        self.answer(
            refusal(trouble, self.wants_json || is_api(&self.path), &self.prefix).into_response(),
        )
    }
}

/// Whether an address is one of the API's, whose answers are parsed.
///
/// Named by its address, since that is all a route has: `/data/<number>` is the
/// whole of it.
pub fn is_api(path: &str) -> bool {
    path.starts_with("/data/")
}

/// The body of a request, read as a form.
///
/// Axum's own extractor refuses a request carrying no body at all, and a browser
/// posting an empty form sends exactly that; a missing field reads as absent
/// either way.
pub struct FormData(pub Vec<(String, String)>);

impl FormData {
    /// One field of the form, the first when it was sent twice.
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// One field, trimmed, or an empty string when it was not sent.
    pub fn trimmed(&self, name: &str) -> String {
        self.get(name).unwrap_or("").trim().to_string()
    }

    /// The form as it is written into the log.
    pub fn fields(&self) -> Vec<(String, String)> {
        self.0.clone()
    }
}

impl<S: Send + Sync> axum::extract::FromRequest<S> for FormData {
    type Rejection = std::convert::Infallible;

    async fn from_request(
        request: axum::extract::Request,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let body = axum::body::to_bytes(request.into_body(), 1024 * 1024)
            .await
            .unwrap_or_default();
        Ok(FormData(
            serde_urlencoded::from_bytes::<Vec<(String, String)>>(&body).unwrap_or_default(),
        ))
    }
}

/// One path segment read the way a route did: digits, and nothing else.
///
/// An address whose number is not one matched no route at all, and a route that
/// matched nothing is a 404 — never a complaint about the address's shape.
pub struct Number(pub i64);

impl<S: Send + Sync> FromRequestParts<S> for Number {
    type Rejection = StatusCode;

    async fn from_request_parts(_parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Read a path segment as the whole number a route's converter took.
///
/// # Returns
///
/// The number, or nothing when the segment is not one — which is a 404.
pub fn whole_number(segment: &str) -> Option<i64> {
    if segment.is_empty() || !segment.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    segment.parse().ok()
}

/// The name the log gives one route, the way the endpoints were named.
pub fn endpoint_of(method: &Method, matched: &str) -> Option<&'static str> {
    let posted = method == Method::POST;
    Some(match (matched, posted) {
        ("/", _) => "web.landing_page",
        ("/book/{number}", _) => "web.read_page",
        ("/data/{number}", _) => "api.get_numbers",
        ("/game/new", true) => "game.create_game",
        ("/game/new", false) => "game.offer_game",
        ("/game/resume", _) => "game.resume_game",
        ("/game", _) => "game.show_character",
        ("/game/death", _) => "game.show_death",
        ("/heroes", _) => "game.show_fallen",
        ("/combat/{number}", _) => "game.show_combat",
        ("/combat/{number}/assault", _) => "game.fight_assault",
        ("/combat/{number}/resolve", _) => "game.leave_combat",
        ("/book/{number}/choice/{index}", _) => "game.take_choice",
        ("/game/pending/{index}/apply", _) => "game.apply_change",
        ("/game/pending/{index}/dismiss", _) => "game.dismiss_change",
        ("/game/pending/dismiss", _) => "game.dismiss_all_changes",
        ("/flag-page", _) => "inspection.report_page",
        ("/flagged-pages", _) => "inspection.list_flagged",
        ("/flagged-pages/{inspection_id}", _) => "inspection.show_flagged",
        ("/flagged-pages/{inspection_id}/resolve", _) => "inspection.resolve_flagged",
        ("/flagged-pages/{inspection_id}/reopen", _) => "inspection.reopen_flagged",
        _ => return None,
    })
}
