//! The general log: the server's own trace, in `logs/general.log`.
//!
//! What is read when something has gone wrong and the page itself has nothing to
//! say about it — a fight that opened on the wrong page, a hero the session
//! lost, a database that stopped answering. The requests that come in and the
//! answers that go out in full (`request_trace`), the games rolled up, the
//! fights begun and decided.
//!
//! **DEBUG, and on.** The level is DEBUG unless `LOG_LEVEL` says otherwise in
//! the environment: a trace one must first go and turn on is a trace one does
//! not have on the day it is needed.
//!
//! Two ways in, and one rule for both: **name every variable and write out its
//! content.**
//!
//! ```ignore
//! note("Combat armed", vars![("page", "22"), ("enemies", 1)]);   // a step, DEBUG
//! event("Hero rolled up", vars![("force", 15), ("vie", 24)]);    // it happened, INFO
//! ```
//!
//! The rule that is not negotiable: **a secret is never written out.** A name
//! that says token, secret, password, authorization, cookie, key or session — at
//! the top level or deep inside a body being logged — is replaced by its length.
//!
//! `game_id` is the exception this application adds. It names the play-through
//! the session cookie is on, and is written to its first few characters. Ids are
//! small integers the database counts up, so in practice they are written whole:
//! guessing one gains nothing, because the cookie that carries it is signed. The
//! cut still shortens anything longer, such as an old uuid left in a cookie from
//! before migration 001.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::core::config::root;
use crate::core::logs::rotating_log::{open_the_log, RotatingLog};
use crate::core::models::hybrid_document::{Field, RowDict};

/// How loud a line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Debug = 10,
    Info = 20,
    Warning = 30,
    Error = 40,
}

impl Level {
    /// The level a name asks for, or nothing when it names none.
    pub fn named(name: &str) -> Option<Self> {
        match name.trim().to_uppercase().as_str() {
            "DEBUG" => Some(Level::Debug),
            "INFO" => Some(Level::Info),
            "WARNING" | "WARN" => Some(Level::Warning),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }
}

/// The level, unless `LOG_LEVEL` says otherwise. Anything unreadable there falls
/// back here rather than stopping the application: a mistyped level must not
/// cost a start-up.
pub const DEFAULT_LEVEL: Level = Level::Debug;

/// Beyond that many characters a value is cut — and says it was, with its full
/// length. A story page is a couple of kilobytes and `/data/<n>` hands one over
/// whole, so a trace that wrote every one of them would hold an afternoon's
/// reading and nothing else. `LOG_VALUE_LIMIT` raises it; 0 there writes every
/// answer whole, however long.
pub const DEFAULT_VALUE_LIMIT: usize = 2000;

/// A field whose name carries one of these is never written out. Broad on
/// purpose: a new secret passing through a name nobody thought of is hidden by
/// default rather than published by default.
pub const SECRET_NAMES: [&str; 7] = [
    "secret",
    "token",
    "password",
    "authorization",
    "cookie",
    "session",
    "key",
];

/// How much of a play-through id is written. A counted-up id is shorter than
/// this and written whole; an old uuid still carried by a cookie is cut to it.
pub const IDENTIFIER_NAMES: [&str; 2] = ["game_id", "game"];
pub const IDENTIFIER_LENGTH: usize = 8;

/// One variable of a line: its name, and its content.
pub type Variables = Vec<(String, Field)>;

/// Name the variables of a line, in the order they are written.
#[macro_export]
macro_rules! vars {
    ($($name:expr => $value:expr),* $(,)?) => {{
        let variables: $crate::core::logs::general_log::Variables =
            vec![$(($name.to_string(), $crate::core::models::hybrid_document::Field::from($value))),*];
        variables
    }};
}

/// One line the log wrote, as a test reads it back.
#[derive(Clone, Debug)]
pub struct Record {
    pub level: Level,
    pub message: String,
    pub exception: Option<String>,
}

impl Record {
    /// The line and its traceback, as the file holds them.
    pub fn text(&self) -> String {
        match &self.exception {
            None => self.message.clone(),
            Some(trouble) => format!("{}\n{trouble}", self.message),
        }
    }
}

/// The file the log writes into.
pub fn general_log_path() -> std::path::PathBuf {
    root().join("logs").join("general.log")
}

/// The open file, made on the first line written.
fn handler() -> &'static Mutex<RotatingLog> {
    static HANDLER: OnceLock<Mutex<RotatingLog>> = OnceLock::new();
    HANDLER.get_or_init(|| Mutex::new(open_the_log(general_log_path())))
}

/// What a test listening to the log collects.
fn capture() -> &'static Mutex<Option<Vec<Record>>> {
    static CAPTURE: OnceLock<Mutex<Option<Vec<Record>>>> = OnceLock::new();
    CAPTURE.get_or_init(|| Mutex::new(None))
}

/// Start collecting the lines rather than only writing them.
pub fn start_capture() {
    *capture().lock().expect("the capture outlives a panic") = Some(Vec::new());
}

/// Every line collected since [`start_capture`], and stop collecting.
pub fn take_capture() -> Vec<Record> {
    capture()
        .lock()
        .expect("the capture outlives a panic")
        .take()
        .unwrap_or_default()
}

/// Every line collected so far, the collecting going on.
pub fn captured() -> Vec<Record> {
    capture()
        .lock()
        .expect("the capture outlives a panic")
        .clone()
        .unwrap_or_default()
}

/// Forget the lines collected so far, the collecting going on.
pub fn clear_capture() {
    if let Some(records) = capture()
        .lock()
        .expect("the capture outlives a panic")
        .as_mut()
    {
        records.clear();
    }
}

/// The cut a value is written out to, which a test may move.
fn value_limit() -> &'static AtomicUsize {
    static LIMIT: OnceLock<AtomicUsize> = OnceLock::new();
    LIMIT.get_or_init(|| AtomicUsize::new(the_limit()))
}

/// The cut as it stands.
pub fn current_value_limit() -> usize {
    value_limit().load(Ordering::Relaxed)
}

/// Move the cut, which only a test needs.
pub fn set_value_limit(limit: usize) {
    value_limit().store(limit, Ordering::Relaxed);
}

/// Read the level from `LOG_LEVEL`, DEBUG failing that.
pub fn the_level() -> Level {
    crate::core::config::load_dotenv();
    std::env::var("LOG_LEVEL")
        .ok()
        .and_then(|asked| Level::named(&asked))
        .unwrap_or(DEFAULT_LEVEL)
}

/// Read the cut from `LOG_VALUE_LIMIT`, [`DEFAULT_VALUE_LIMIT`] failing that.
///
/// # Returns
///
/// The number of characters a value is written out to; 0 for no cut at all.
pub fn the_limit() -> usize {
    crate::core::config::load_dotenv();
    let asked = std::env::var("LOG_VALUE_LIMIT").unwrap_or_default();
    let asked = asked.trim();
    if !asked.is_empty() && asked.chars().all(|character| character.is_ascii_digit()) {
        return asked.parse().unwrap_or(DEFAULT_VALUE_LIMIT);
    }
    DEFAULT_VALUE_LIMIT
}

/// Write one step of what the server is doing, at DEBUG.
pub fn note(message: &str, variables: Variables) {
    write(Level::Debug, message, variables, None);
}

/// Write something that happened rather than a step towards it, at INFO.
///
/// A hero rolled up, a fight decided, a game abandoned: what is still worth
/// having when the level has been raised to leave the steps out.
pub fn event(message: &str, variables: Variables) {
    write(Level::Info, message, variables, None);
}

/// Write something that went wrong, at ERROR, with the trouble under the line.
///
/// # Arguments
///
/// * `trouble` - What went wrong, written under the line; `None` for a failure
///   that raised nothing.
pub fn failure(message: &str, trouble: Option<&str>, variables: Variables) {
    write(Level::Error, message, variables, trouble);
}

/// Put a line together and send it to the file, and to a listening test.
fn write(level: Level, message: &str, variables: Variables, trouble: Option<&str>) {
    if level < the_level() {
        return;
    }
    let record = Record {
        level,
        message: spell_out(message, &variables),
        exception: trouble.map(str::to_string),
    };
    if let Some(records) = capture()
        .lock()
        .expect("the capture outlives a panic")
        .as_mut()
    {
        records.push(record.clone());
    }
    let mut log = handler().lock().expect("the log outlives a panic");
    let line = log.format(&record.text());
    log.emit(&line);
}

/// Put a message and its variables into the one line the log carries.
///
/// ```text
/// Combat armed — page='22', enemies=1, game='3'
/// ```
///
/// # Returns
///
/// The message alone when there is no variable; the message, an em dash and the
/// variables otherwise.
pub fn spell_out(message: &str, variables: &[(String, Field)]) -> String {
    if variables.is_empty() {
        return message.to_string();
    }
    let written: Vec<String> = variables
        .iter()
        .map(|(name, value)| format!("{name}={}", shown(name, value)))
        .collect();
    format!("{message} — {}", written.join(", "))
}

/// Write out one variable's content, unless its name forbids it.
///
/// # Returns
///
/// The content, cut at the limit; a description of its length for a secret; the
/// first characters for a play-through id.
pub fn shown(name: &str, value: &Field) -> String {
    if is_a_secret(name) {
        return hidden(value);
    }
    if is_an_identifier(name) {
        return shortened(value);
    }
    cut(&readable(&sanitised(value)))
}

/// Say whether a field's name forbids writing its content.
pub fn is_a_secret(name: &str) -> bool {
    let lowered = name.to_lowercase();
    SECRET_NAMES.iter().any(|word| lowered.contains(word))
}

/// Say whether a field names a play-through, which is written in part only.
pub fn is_an_identifier(name: &str) -> bool {
    IDENTIFIER_NAMES.contains(&name.to_lowercase().as_str())
}

/// Describe a secret without writing it: its length, and nothing else.
///
/// # Returns
///
/// `<absent>` for nothing at all, `<hidden, n characters>` otherwise.
pub fn hidden(value: &Field) -> String {
    if value.is_null() {
        return "<absent>".to_string();
    }
    format!("<hidden, {} characters>", value.to_string().chars().count())
}

/// Write the head of a play-through id, enough to follow it through a run.
///
/// # Returns
///
/// `<absent>` for nothing at all, the first characters followed by an ellipsis
/// otherwise.
pub fn shortened(value: &Field) -> String {
    if value.is_null() {
        return "<absent>".to_string();
    }
    let written = value.to_string();
    if written.chars().count() <= IDENTIFIER_LENGTH {
        return quoted(&written);
    }
    let head: String = written.chars().take(IDENTIFIER_LENGTH).collect();
    quoted(&format!("{head}…"))
}

/// Walk a value and replace every secret-named field inside it.
///
/// A body is logged whole, and a body is where a secret travels.
///
/// # Returns
///
/// The same value, with the fields whose name is a secret's replaced by their
/// description.
pub fn sanitised(value: &Field) -> Field {
    match value {
        Field::Dict(entries) => Field::Dict(
            entries
                .iter()
                .map(|(name, content)| {
                    let cleaned = if is_a_secret(name) {
                        Field::Str(hidden(content))
                    } else {
                        sanitised(content)
                    };
                    (name.clone(), cleaned)
                })
                .collect::<RowDict>(),
        ),
        Field::List(entries) => Field::List(entries.iter().map(sanitised).collect()),
        other => other.clone(),
    }
}

/// Write a value the way it is read back: JSON for a structure, quoted for text.
///
/// Quoted strings on purpose — an empty one, a trailing space or a stray newline
/// are read in the quotes and invisible without them. Anything JSON refuses (a
/// date, a number that is not one) is written as it looks, since the log wants
/// what it looks like, not what it can be reloaded from.
pub fn readable(value: &Field) -> String {
    match value {
        Field::Null | Field::Bool(_) | Field::Int(_) | Field::Float(_) => value.to_string(),
        Field::Str(text) => quoted(text),
        other => as_json_text(other).unwrap_or_else(|| own_description(other)),
    }
}

/// A URL as a log may carry it: every parameter but the credentials in it.
///
/// Used on the `Location` of every redirect, which is where this application
/// sends a reader after an assault.
///
/// The query is rebuilt unescaped, as it reads rather than as it would be
/// replayed: this is a log.
pub fn without_the_secrets(url: &str) -> String {
    let Some((before, query)) = url.split_once('?') else {
        return url.to_string();
    };
    let (query, fragment) = match query.split_once('#') {
        Some((query, fragment)) => (query, Some(fragment)),
        None => (query, None),
    };
    if query.is_empty() {
        return url.to_string();
    }
    let parameters: Vec<String> = query
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
            let name = decoded(name);
            let value = decoded(value);
            if is_a_secret(&name) {
                format!("{name}={}", hidden(&Field::Str(value)))
            } else {
                format!("{name}={value}")
            }
        })
        .collect();
    let rebuilt = format!("{before}?{}", parameters.join("&"));
    match fragment {
        Some(fragment) => format!("{rebuilt}#{fragment}"),
        None => rebuilt,
    }
}

/// Shorten what is too long to be worth a whole file, and say what was cut.
pub fn cut(text: &str) -> String {
    let limit = current_value_limit();
    let length = text.chars().count();
    if limit == 0 || length <= limit {
        return text.to_string();
    }
    let head: String = text.chars().take(limit).collect();
    format!("{head}… (cut, {length} characters in all)")
}

/// One query parameter, as it reads rather than as it was escaped.
fn decoded(text: &str) -> String {
    percent_encoding::percent_decode_str(&text.replace('+', " "))
        .decode_utf8_lossy()
        .into_owned()
}

/// A value as JSON, the way `json.dumps` writes one, or nothing when it refuses.
fn as_json_text(value: &Field) -> Option<String> {
    match value {
        Field::Null => Some("null".to_string()),
        Field::Bool(true) => Some("true".to_string()),
        Field::Bool(false) => Some("false".to_string()),
        Field::Int(number) => Some(number.to_string()),
        Field::Float(number) if number.is_finite() => Some(number.to_string()),
        // JSON has no way of writing a number that is not one.
        Field::Float(_) => None,
        Field::Str(text) => Some(json_string(text)),
        // What JSON cannot write goes through its text, as the hook did.
        Field::Instant(_) | Field::Date(_) => Some(json_string(&value.to_string())),
        Field::List(entries) => {
            let written: Option<Vec<String>> = entries.iter().map(as_json_text).collect();
            Some(format!("[{}]", written?.join(", ")))
        }
        Field::Dict(entries) => {
            let written: Option<Vec<String>> = entries
                .iter()
                .map(|(key, entry)| Some(format!("{}: {}", json_string(key), as_json_text(entry)?)))
                .collect();
            Some(format!("{{{}}}", written?.join(", ")))
        }
    }
}

/// One string as JSON writes it, accents kept.
fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// A value as its own description, for the log's last resort.
fn own_description(value: &Field) -> String {
    match value {
        Field::Str(text) => quoted(text),
        Field::Float(number) if number.is_nan() => "nan".to_string(),
        Field::Float(number) if number.is_infinite() && *number > 0.0 => "inf".to_string(),
        Field::Float(number) if number.is_infinite() => "-inf".to_string(),
        Field::List(entries) => format!(
            "[{}]",
            entries
                .iter()
                .map(own_description)
                .collect::<Vec<String>>()
                .join(", ")
        ),
        Field::Dict(entries) => format!(
            "{{{}}}",
            entries
                .iter()
                .map(|(key, entry)| format!("{}: {}", quoted(key), own_description(entry)))
                .collect::<Vec<String>>()
                .join(", ")
        ),
        other => other.to_string(),
    }
}

/// One string in quotes, the way it is read back off a line.
fn quoted(text: &str) -> String {
    if text.contains('\'') && !text.contains('"') {
        return format!("\"{text}\"");
    }
    format!("'{}'", text.replace('\\', "\\\\").replace('\'', "\\'"))
}
