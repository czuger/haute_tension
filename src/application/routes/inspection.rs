//! Flagging a page as having a problem, and reviewing the pages flagged.
//!
//! The header's **Signaler** opens a dialog on every page; the form in it posts
//! here with the page's own path and title filled in. Sent by the browser's
//! script, the answer is JSON and the reader never leaves the page; sent as a
//! plain form, it is a flash message and a redirect back to the page, so the
//! feature works without JavaScript.
//!
//! There are no users, so nothing here is signed and nothing is restricted:
//! `/flagged-pages` is a tool for whoever maintains the book.

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::application::errors::AppError;
use crate::application::routes::{whole_number, AppState, FormData, Reader};
use crate::application::templates::{FlaggedPageTemplate, FlaggedPagesTemplate};
use crate::core::db;
use crate::core::models::page_inspection::{OPEN, RESOLVED, STATUSES};

pub const BAD_REQUEST: StatusCode = StatusCode::BAD_REQUEST;

pub const FLAGGED: &str = "Merci, la page est signalée pour inspection.";
pub const NOTHING_SAID: &str = "Dites en quelques mots ce qui ne va pas.";

/// File a report on a page: open the file on it, or add to it.
///
/// A blank remark is refused with a 400 rather than filed: a flag with nothing
/// said tells the inspector nothing.
pub async fn report_page(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: FormData,
) -> Response {
    let mut reader = Reader::of(&state, &headers, "/flag-page");
    let path = reported_path(&form);
    let text = form.trimmed("comment");
    if text.is_empty() {
        return refused(&mut reader, NOTHING_SAID, &path);
    }

    let title = form.trimmed("title");
    let filed = db::flag_page(
        &state.db,
        &state.book,
        &path,
        Some(title.as_str()).filter(|title| !title.is_empty()),
        &text,
    )
    .await;
    match filed {
        Err(trouble) => reader.refuse(&AppError::Database(trouble)),
        Ok(inspection) if reader.wants_json => reader.answer(
            json_answer(
                StatusCode::OK,
                serde_json::json!({
                    "success": true,
                    "message": FLAGGED,
                    "comments": inspection.comments.len(),
                }),
            ),
        ),
        Ok(_) => {
            reader.session.flash(FLAGGED, "success");
            reader.redirect(&path)
        }
    }
}

/// How the list was narrowed.
#[derive(serde::Deserialize, Default)]
pub struct ListQuery {
    pub status: Option<String>,
}

/// The pages flagged for inspection, most recently commented first.
///
/// `?status=open` or `?status=resolved` narrows the list; anything else shows
/// everything.
pub async fn list_flagged(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
    headers: HeaderMap,
) -> Response {
    let mut reader = Reader::of(&state, &headers, "/flagged-pages");
    let status = query
        .status
        .filter(|status| STATUSES.contains(&status.as_str()));
    match render_list(&state, &mut reader, status).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The list, narrowed to one state or not narrowed at all.
async fn render_list(
    state: &AppState,
    reader: &mut Reader,
    status: Option<String>,
) -> Result<String, AppError> {
    let inspections = db::flagged_pages(&state.db, &state.book, status.as_deref()).await?;
    let layout = reader.layout(format!("Pages signalées · {}", state.book_title));
    Ok(FlaggedPagesTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        inspections,
        status,
    }
    .render()?)
}

/// Everything said about one flagged page.
pub async fn show_flagged(
    State(state): State<AppState>,
    Path(inspection_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/flagged-pages/{inspection_id}");
    let mut reader = Reader::of(&state, &headers, &path);
    match render_file(&state, &mut reader, &inspection_id).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// One flagged page's whole thread.
async fn render_file(
    state: &AppState,
    reader: &mut Reader,
    inspection_id: &str,
) -> Result<String, AppError> {
    let Some(inspection_id) = whole_number(inspection_id) else {
        return Err(AppError::NotFound);
    };
    let Some(inspection) = db::find_inspection(&state.db, Some(inspection_id)).await? else {
        return Err(AppError::NotFound);
    };
    let layout = reader.layout(format!(
        "Page signalée · {} · {}",
        inspection.path, state.book_title
    ));
    Ok(FlaggedPageTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        inspection,
    }
    .render()?)
}

/// Close the file on a page.
pub async fn resolve_flagged(
    State(state): State<AppState>,
    Path(inspection_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    set_status(state, inspection_id, headers, RESOLVED).await
}

/// Open the file on a page again.
pub async fn reopen_flagged(
    State(state): State<AppState>,
    Path(inspection_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    set_status(state, inspection_id, headers, OPEN).await
}

/// Mark one file, and come back to it.
async fn set_status(
    state: AppState,
    inspection_id: String,
    headers: HeaderMap,
    status: &str,
) -> Response {
    let verb = if status == RESOLVED { "resolve" } else { "reopen" };
    let path = format!("/flagged-pages/{inspection_id}/{verb}");
    let reader = Reader::of(&state, &headers, &path);
    let Some(id) = whole_number(&inspection_id) else {
        return reader.refuse(&AppError::NotFound);
    };
    match db::set_inspection_status(&state.db, id, status).await {
        Err(trouble) => reader.refuse(&AppError::from(trouble)),
        Ok(None) => reader.refuse(&AppError::NotFound),
        Ok(Some(_)) => reader.redirect(&format!("/flagged-pages/{id}")),
    }
}

/// The path the form says it was sent from, kept on this site.
///
/// Only a local path is filed or redirected to: an absolute URL in the field
/// would make the redirect an open one, so anything else counts as the root.
fn reported_path(form: &FormData) -> String {
    let path = form.trimmed("path");
    if path.starts_with('/') && !path.starts_with("//") {
        return path;
    }
    "/".to_string()
}

/// Send a bad report back with the reason: JSON for a script, a flash otherwise.
fn refused(reader: &mut Reader, message: &str, path: &str) -> Response {
    if reader.wants_json {
        return reader.answer(json_answer(
            BAD_REQUEST,
            serde_json::json!({"success": false, "message": message}),
        ));
    }
    reader.session.flash(message, "error");
    reader.redirect(path)
}

/// One JSON answer, accents kept as they are written.
fn json_answer(status: StatusCode, body: serde_json::Value) -> Response {
    (
        status,
        [(header::CONTENT_TYPE, "application/json")],
        body.to_string(),
    )
        .into_response()
}
