//! The browser routes for the book landing page and the reader.

use askama::Template;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;

use crate::application::errors::AppError;
use crate::application::models::game::{GameDict, SESSION_KEY};
use crate::application::routes::{whole_number, AppState, Reader};
use crate::application::templates::{ChoiceView, IndexTemplate, PageTemplate, ReaderPage};
use crate::core::db::{self, Db, MAX_PAGE_HISTORY};
use crate::core::inventory::read_changes;

pub const OPENING_PAGE: i64 = 1;

/// Show the current book and a link to begin reading.
///
/// It reads nothing: the landing page must keep working when there is no
/// database at all.
pub async fn landing_page(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let mut reader = Reader::of(&state, &headers, "/");
    let layout = reader.layout(format!("{} · Haute Tension", state.book_title));
    match (IndexTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        opening_page: OPENING_PAGE,
    })
    .render()
    {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&AppError::Render(trouble)),
    }
}

/// Show one story page, its destinations, and the trail that led here.
pub async fn read_page(
    State(state): State<AppState>,
    Path(number): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/book/{number}");
    let mut reader = Reader::of(&state, &headers, &path);
    match render_page(&state, &mut reader, &number).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// Build the reader's page, or say why it cannot be built.
async fn render_page(
    state: &AppState,
    reader: &mut Reader,
    number: &str,
) -> Result<String, AppError> {
    let Some(number) = whole_number(number) else {
        return Err(AppError::NotFound);
    };
    let number = number.to_string();
    let Some(page) = state.story.get(&number) else {
        return Err(AppError::NotFound);
    };
    let game = current_game(&state.db, reader).await?;
    let trail = reading_trail(&state.db, &state.book, &number, game.as_ref()).await?;
    let choices: Vec<ChoiceView> = page
        .choices()
        .iter()
        .map(|choice| ChoiceView {
            goto: choice
                .get("goto")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("")
                .to_string(),
            changes: read_changes(choice)
                .iter()
                .map(crate::core::inventory::Change::described)
                .collect(),
        })
        .collect();
    let layout = reader.layout(format!("Page {} · {}", page.page(), state.book_title));
    Ok(PageTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        page: ReaderPage {
            page: page.page().to_string(),
            text: page.text().iter().map(|line| (*line).to_string()).collect(),
            has_fight: page.fight_has_enemies(),
            choices,
        },
        trail,
        game,
    }
    .render()?)
}

/// Record this visit and return the pages most recently read, oldest first.
///
/// Nothing is caught here. A database that cannot answer used to cost the
/// breadcrumb and nothing else, which read well and worked badly: every page
/// then paid the driver's full timeout to render a page that quietly lacked half
/// of itself. A reader is better told.
async fn reading_trail(
    db: &Db,
    book: &str,
    current_page: &str,
    game: Option<&GameDict>,
) -> Result<Vec<String>, AppError> {
    db::record_page_view(db, book, current_page, game.map(|game| game.id)).await?;
    Ok(db::last_pages(db, book, MAX_PAGE_HISTORY).await?)
}

/// The game the session is on, or nothing — a reader may have no hero yet.
///
/// The reader works without one: the book is readable, and only the choices that
/// cost something need somewhere to charge them.
pub async fn current_game(db: &Db, reader: &Reader) -> Result<Option<GameDict>, AppError> {
    Ok(db::find_game(db, reader.session.get_id(SESSION_KEY)).await?)
}
