//! Every request written into the general log: what came in, what went out, and
//! how long it took.
//!
//! Wired onto the router itself and not onto a group of routes, so that
//! **nothing is outside it**: a page, a fight, a refusal, an address that
//! matches no route at all — each one leaves its two lines, `Request` then
//! `Answer`, and a third when it fails.
//!
//! ```text
//! Request — method='GET', path='/combat/22', endpoint='game.show_combat', game='3'
//! Answer  — status=200, type='text/html; charset=utf-8', took='4.2 ms', body=<5264 bytes of text/html>
//! ```
//!
//! **A JSON answer is written out in full**, because that is what one comes to
//! this log for: what the page was actually given, not what it was meant to be.
//! Three exceptions:
//!
//! - a **streamed** answer is never read here — reading it would consume it;
//! - anything **not JSON** is described rather than copied: a page is 12 KB of
//!   HTML that says nothing a status code does not say better — **unless it is a
//!   refusal**, from 400 up, where the body carries the sentence explaining it;
//! - a value longer than the limit is cut and says by how much.

use std::time::Instant;

use axum::body::{Body, Bytes};
use axum::extract::{MatchedPath, Request, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use indexmap::IndexMap;
use serde_json::Value;

use crate::application::errors::DatabaseTrouble;
use crate::application::models::game::SESSION_KEY;
use crate::application::routes::{endpoint_of, whole_number, AppState};
use crate::application::session::Session;
use crate::core::logs::general_log::{failure, note, without_the_secrets};
use crate::core::models::hybrid_document::{Field, RowDict};
use crate::vars;

/// How much of an answer is read to be written into the log.
const MOST_A_BODY_CAN_BE: usize = 8 * 1024 * 1024;

/// Write both lines of one request, and hand the answer back untouched.
///
/// # Errors
///
/// Never: a request whose trace cannot be written is still answered.
pub async fn trace_the_request(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let query = query_of(request.uri().query());
    let matched = request
        .extensions()
        .get::<MatchedPath>()
        .map(|matched| matched.as_str().to_string());
    let endpoint = matched
        .as_deref()
        .and_then(|matched| endpoint_of(&method, matched));
    let headers = request.headers().clone();
    let game = game_of(&headers, &state);

    let (parts, body) = request.into_parts();
    let payload = read_the_payload(&parts.headers, body).await;
    let request = Request::from_parts(parts, Body::from(payload.bytes.clone()));

    note(
        "Request",
        vars! {
            "method" => method.as_str(),
            "path" => path.clone(),
            "query" => Field::Dict(query),
            "endpoint" => endpoint,
            "route" => Field::Dict(route_arguments(matched.as_deref(), &path)),
            "game" => game,
            "address" => address_of(&headers),
            "payload" => payload.written,
        },
    );

    let response = next.run(request).await;
    let (parts, body) = response.into_parts();
    if let Some(trouble) = parts.extensions.get::<DatabaseTrouble>() {
        failure(
            "The database could not answer, refusing the request",
            Some(&trouble.written()),
            vars! {
                "method" => method.as_str(),
                "path" => path.clone(),
                "endpoint" => endpoint,
            },
        );
    }
    let content_type = parts
        .headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let answer = read_the_answer(parts.status, body).await;

    let mut variables = vars! {
        "method" => method.as_str(),
        "path" => path.clone(),
        "status" => i64::from(parts.status.as_u16()),
        "type" => content_type.clone(),
        "took" => how_long_it_took(started),
        "body" => answer.written,
    };
    if let Some(destination) = where_it_sends(&parts.headers) {
        variables.push(("destination".to_string(), Field::Str(destination)));
    }
    note("Answer", variables);

    if parts.status == StatusCode::INTERNAL_SERVER_ERROR {
        failure(
            "Request failed",
            Some(&answer.bytes_as_text),
            vars! {
                "method" => method.as_str(),
                "path" => path,
                "endpoint" => endpoint,
                "took" => how_long_it_took(started),
            },
        );
    }

    Response::from_parts(parts, Body::from(answer.bytes))
}

/// Where a redirect sends, for the lines that are one.
///
/// A redirect's body says nothing and its `Location` says everything: it is what
/// a fight is followed on, assault after assault.
fn where_it_sends(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::LOCATION)
        .and_then(|value| value.to_str().ok())
        .map(without_the_secrets)
}

/// How long the request has been in the server's hands.
pub fn how_long_it_took(started: Instant) -> String {
    format!("{:.1} ms", started.elapsed().as_secs_f64() * 1000.0)
}

/// The query string, as a dict of its parameters.
fn query_of(query: Option<&str>) -> RowDict {
    let Some(query) = query.filter(|query| !query.is_empty()) else {
        return RowDict::new();
    };
    serde_urlencoded::from_str::<Vec<(String, String)>>(query)
        .unwrap_or_default()
        .into_iter()
        .map(|(name, value)| (name, Field::Str(value)))
        .collect()
}

/// The play-through the session cookie names.
fn game_of(headers: &HeaderMap, state: &AppState) -> Field {
    let cookie = headers
        .get(header::COOKIE)
        .and_then(|value| value.to_str().ok());
    match Session::decode(cookie, &state.secret).get(SESSION_KEY) {
        Some(Value::Number(number)) => Field::Int(number.as_i64().unwrap_or_default()),
        Some(Value::String(text)) => Field::Str(text.clone()),
        _ => Field::Null,
    }
}

/// Who is asking, as far as the headers say.
fn address_of(headers: &HeaderMap) -> Field {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.to_str().ok())
        .map_or(Field::Null, |address| Field::Str(address.to_string()))
}

/// The arguments the route matched, read as the converters read them.
///
/// Every one of this application's is a whole number, so a segment is written as
/// the number it is rather than as the text it arrived in.
fn route_arguments(matched: Option<&str>, path: &str) -> RowDict {
    let Some(matched) = matched else {
        return RowDict::new();
    };
    let mut arguments: IndexMap<String, Field> = IndexMap::new();
    for (pattern, segment) in matched.split('/').zip(path.split('/')) {
        let Some(name) = pattern
            .strip_prefix('{')
            .and_then(|name| name.strip_suffix('}'))
        else {
            continue;
        };
        arguments.insert(
            name.to_string(),
            whole_number(segment).map_or_else(|| Field::Str(segment.to_string()), Field::Int),
        );
    }
    arguments
}

/// A body that has been read, and the way it is written into the log.
struct Payload {
    bytes: Bytes,
    written: Field,
}

/// The request's payload, in the form it is read back.
///
/// The parsed JSON body, the form as a dict, nothing for a request carrying
/// nothing, and a description of anything else.
async fn read_the_payload(headers: &HeaderMap, body: Body) -> Payload {
    let bytes = axum::body::to_bytes(body, MOST_A_BODY_CAN_BE)
        .await
        .unwrap_or_default();
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();
    let written = if content_type.starts_with("application/json") {
        serde_json::from_slice::<Value>(&bytes).map_or(Field::Null, |body| Field::from_json(&body))
    } else if content_type.starts_with("application/x-www-form-urlencoded") && !bytes.is_empty() {
        Field::Dict(
            serde_urlencoded::from_bytes::<Vec<(String, String)>>(&bytes)
                .unwrap_or_default()
                .into_iter()
                .map(|(name, value)| (name, Field::Str(value)))
                .collect(),
        )
    } else if bytes.is_empty() {
        Field::Null
    } else {
        Field::Str(format!("<{} bytes, {content_type}>", bytes.len()))
    };
    Payload { bytes, written }
}

/// An answer that has been read, and the way it is written into the log.
struct Answer {
    bytes: Bytes,
    bytes_as_text: String,
    written: Field,
}

/// The answer's body, in full where reading it costs nothing.
async fn read_the_answer(status: StatusCode, body: Body) -> Answer {
    if body.size_hint_is_unknown() {
        return Answer {
            bytes: Bytes::new(),
            bytes_as_text: String::new(),
            written: Field::Str(STREAMED.to_string()),
        };
    }
    let bytes = axum::body::to_bytes(body, MOST_A_BODY_CAN_BE)
        .await
        .unwrap_or_default();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let written = what_goes_out(status, &text, bytes.len());
    Answer {
        bytes,
        bytes_as_text: text,
        written,
    }
}

/// What a streamed answer is written as, rather than being read.
pub const STREAMED: &str = "<streamed, left untouched>";

/// The answer's body as the log carries it.
///
/// The refusal first: a 404 raised by the router would otherwise read as a page
/// and be described rather than written, and a refusal is where the sentence
/// explaining it lives.
pub fn what_goes_out(status: StatusCode, text: &str, length: usize) -> Field {
    if status.as_u16() >= 400 {
        return match serde_json::from_str::<Value>(text) {
            Ok(body) => Field::from_json(&body),
            Err(_) => Field::Str(text.to_string()),
        };
    }
    match serde_json::from_str::<Value>(text) {
        Ok(body) if looks_like_json(text) => Field::from_json(&body),
        _ => Field::Str(format!("<{length} bytes of {}>", mimetype(text))),
    }
}

/// Whether a body is JSON rather than a page that happens to parse as one.
fn looks_like_json(text: &str) -> bool {
    let trimmed = text.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

/// The kind of thing a body is, for the line that describes rather than copies.
fn mimetype(text: &str) -> &'static str {
    if text.trim_start().starts_with("<!doctype") || text.trim_start().starts_with("<html") {
        return "text/html";
    }
    "text/html"
}

/// Whether a body's length is unknown, which is what a streamed one is.
trait StreamedBody {
    fn size_hint_is_unknown(&self) -> bool;
}

impl StreamedBody for Body {
    fn size_hint_is_unknown(&self) -> bool {
        use axum::body::HttpBody as _;
        self.size_hint().exact().is_none()
    }
}
