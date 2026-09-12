//! Building the application: the book off disk, the routes, and the trace.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::routing::{get, post};
use axum::Router;

use crate::application::logs::request_trace::trace_the_request;
use crate::application::routes::{api, game, inspection, web, AppState};
use crate::core::config::{root, session_secret};
use crate::core::db::Db;
use crate::core::dice::Rng;
use crate::core::logs::general_log::event;
use crate::core::story::{load_story, StoryError};
use crate::vars;

pub const BOOK: &str = "pretre_jean/forteresse_alamuth";
pub const BOOK_SERIES: &str = "Prêtre Jean";
pub const BOOK_TITLE: &str = "La Forteresse d'Alamuth";

/// The directory the books live under.
pub fn books_path() -> PathBuf {
    root().join("books")
}

/// How the application is put together.
pub struct AppConfig {
    pub book: String,
    pub books_path: PathBuf,
    /// Source of chance for character creation and combat. Seed one to make a
    /// run reproducible; the default leaves it to chance.
    pub rng: Option<Arc<dyn Rng>>,
    /// The database the routes read and write.
    pub db: Db,
    /// The key the session cookie is signed with.
    pub secret: Vec<u8>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            book: BOOK.to_string(),
            books_path: books_path(),
            rng: None,
            db: Db::Environment,
            secret: session_secret().into_bytes(),
        }
    }
}

impl AppConfig {
    /// Serve one book off one directory.
    pub fn for_book(book: &str, books_path: &Path) -> Self {
        Self {
            book: book.to_string(),
            books_path: books_path.to_path_buf(),
            ..Self::default()
        }
    }
}

/// Create and configure the application.
///
/// The book is read off disk once, at startup: it is static, small enough to sit
/// in memory, and changes only when the import pipeline is re-run. The database
/// holds the reading history and the play-throughs.
///
/// # Errors
///
/// If the book has no `pages.json`, or the book data is malformed or repeats a
/// page.
pub fn create_app(config: AppConfig) -> Result<Router, StoryError> {
    let story = load_story(&config.books_path.join(&config.book))?;
    let fights = story
        .values()
        .filter(|page| page.fight().is_some())
        .count() as i64;
    let pages = story.len() as i64;
    let state = AppState {
        db: config.db,
        story: Arc::new(story),
        book: config.book.clone(),
        book_series: BOOK_SERIES.to_string(),
        book_title: BOOK_TITLE.to_string(),
        rng: config.rng,
        secret: Arc::new(config.secret),
    };

    let router = Router::new()
        .route("/", get(web::landing_page))
        .route("/book/{number}", get(web::read_page))
        .route("/data/{number}", get(api::get_numbers))
        .route(
            "/game/new",
            get(game::offer_game).post(game::create_game),
        )
        .route("/game/resume", get(game::resume_game))
        .route("/game", get(game::show_character))
        .route("/game/death", get(game::show_death))
        .route("/heroes", get(game::show_fallen))
        .route("/combat/{number}", get(game::show_combat))
        .route("/combat/{number}/assault", post(game::fight_assault))
        .route("/combat/{number}/resolve", post(game::leave_combat))
        .route("/book/{number}/choice/{index}", post(game::take_choice))
        .route("/game/pending/dismiss", post(game::dismiss_all_changes))
        .route("/game/pending/{index}/apply", post(game::apply_change))
        .route("/game/pending/{index}/dismiss", post(game::dismiss_change))
        .route("/flag-page", post(inspection::report_page))
        .route("/flagged-pages", get(inspection::list_flagged))
        .route(
            "/flagged-pages/{inspection_id}",
            get(inspection::show_flagged),
        )
        .route(
            "/flagged-pages/{inspection_id}/resolve",
            post(inspection::resolve_flagged),
        )
        .route(
            "/flagged-pages/{inspection_id}/reopen",
            post(inspection::reopen_flagged),
        )
        .fallback(no_such_address)
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            trace_the_request,
        ))
        .with_state(state);

    event(
        "Application built",
        vars! {
            "book" => config.book,
            "pages" => pages,
            "fights" => fights,
        },
    );
    Ok(router)
}

/// An address that matches no route at all is a 404, and is traced like any other.
async fn no_such_address() -> axum::response::Response {
    use axum::response::IntoResponse;
    crate::application::errors::refusal(&crate::application::errors::AppError::NotFound, false)
        .into_response()
}
