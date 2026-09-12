//! The route serving story data.

use axum::extract::{Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::application::errors::AppError;
use crate::application::routes::{whole_number, AppState, Reader};
use crate::core::db;

/// Return one story page with its choices and actions.
pub async fn get_numbers(
    State(state): State<AppState>,
    Path(number): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/data/{number}");
    let reader = Reader::of(&state, &headers, &path);
    let Some(number) = whole_number(&number) else {
        return reader.refuse(&AppError::NotFound);
    };
    let number_string = number.to_string();
    if let Err(trouble) = db::record_page_view(&state.db, &state.book, &number_string, None).await {
        return reader.refuse(&AppError::Database(trouble));
    }
    let Some(page) = state.story.get(&number_string) else {
        return page_not_found_response(number);
    };
    json_answer(
        serde_json::to_string(page.as_object()).unwrap_or_else(|_| "{}".to_string()),
    )
}

/// Build the UTF-8 answer for an unknown page number.
fn page_not_found_response(number: i64) -> Response {
    let body = serde_json::json!({
        "success": false,
        "message": format!("le numéro {number} n'a pas été trouvé"),
    });
    json_answer(serde_json::to_string_pretty(&body).unwrap_or_default())
}

/// One JSON answer, accents kept as they are written.
fn json_answer(body: String) -> Response {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
        .into_response()
}
