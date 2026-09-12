//! The pages, and what each of them is handed.
//!
//! One struct per template, checked against the template at compile time. The
//! header is the same on every page — the menu, the flash messages, the flag
//! dialog — so every struct carries a [`Layout`], which is what `base.html`
//! reads: the title of the page, the address it was reached at, and anything
//! waiting to be said to the reader.

use askama::Template;

use crate::application::models::game::{Combat, FallenHero, GameDict, Pending};
use crate::application::models::inspection::InspectionDict;
use crate::core::character::Mode;

/// One thing to tell the reader on the next page he is shown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Flash {
    pub category: String,
    pub message: String,
}

/// What every page carries, whatever else it holds.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    /// The page's own title, in the tab and in the flag dialog.
    pub title: String,
    /// The address the page was reached at, which a report is filed against.
    pub path: String,
    /// What is waiting to be said, put there by the request before this one.
    pub flashes: Vec<Flash>,
}

impl Layout {
    /// Build the header of one page.
    pub fn new(title: impl Into<String>, path: impl Into<String>, flashes: Vec<Flash>) -> Self {
        Self {
            title: title.into(),
            path: path.into(),
            flashes,
        }
    }
}

/// One choice of a page, and what taking it will cost.
#[derive(Clone, Debug)]
pub struct ChoiceView {
    pub goto: String,
    pub changes: Vec<String>,
}

/// The page being read, as the reader's template needs it.
#[derive(Clone, Debug)]
pub struct ReaderPage {
    pub page: String,
    pub text: Vec<String>,
    pub has_fight: bool,
    pub choices: Vec<ChoiceView>,
}

/// The landing page: the one book on offer.
#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub opening_page: i64,
}

/// One story page, its destinations, and the trail that led here.
#[derive(Template)]
#[template(path = "page.html")]
pub struct PageTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub page: ReaderPage,
    pub trail: Vec<String>,
    pub game: Option<GameDict>,
}

/// The hero's sheet: everything about him, on one page.
#[derive(Template)]
#[template(path = "character.html")]
pub struct CharacterTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub game: GameDict,
    pub rolled: bool,
    pub back_page: Option<String>,
    /// How full the Vie gauge is drawn, as a percentage.
    pub gauge: i64,
}

/// The offer to roll a hero up, and what it costs.
#[derive(Template)]
#[template(path = "new_game.html")]
pub struct NewGameTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub game: Option<GameDict>,
    pub modes: Vec<Mode>,
    pub fallen: Vec<FallenHero>,
}

/// The end of an adventure, and the offer of another.
#[derive(Template)]
#[template(path = "death.html")]
pub struct DeathTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub game: Option<GameDict>,
    pub modes: Vec<Mode>,
}

/// The memorial: the heroes who did not come back.
#[derive(Template)]
#[template(path = "heroes.html")]
pub struct HeroesTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub fallen: Vec<FallenHero>,
    pub game: Option<GameDict>,
}

/// The fight a page holds, assault by assault.
#[derive(Template)]
#[template(path = "combat.html")]
pub struct CombatTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub game: GameDict,
    pub combat: Combat,
    pub page_number: String,
}

/// The pages flagged for inspection.
#[derive(Template)]
#[template(path = "flagged_pages.html")]
pub struct FlaggedPagesTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub inspections: Vec<InspectionDict>,
    pub status: Option<String>,
}

/// Everything said about one flagged page.
#[derive(Template)]
#[template(path = "flagged_page.html")]
pub struct FlaggedPageTemplate {
    pub layout: Layout,
    pub book_series: String,
    pub book_title: String,
    pub inspection: InspectionDict,
}

/// What a reader is shown when the database cannot answer.
#[derive(Template)]
#[template(path = "database_error.html")]
pub struct DatabaseErrorTemplate {
    pub layout: Layout,
}

/// Whether a waiting change asks the reader to roll for the amount.
pub fn is_rolled(change: &Pending) -> bool {
    change.note.as_deref() == Some(crate::core::inventory::DICE)
}

/// The filters the templates call by name.
///
/// Askama resolves `{{ x|french_date }}` to `filters::french_date`, so the one
/// filter this application adds lives here, beside the templates that use it.
pub mod filters {
    use std::convert::Infallible;
    use std::fmt::Display;

    use askama::Values;

    use crate::application::french_date::french_date as read_as_a_french_date;

    /// Print an ISO 8601 instant as a French date.
    ///
    /// # Errors
    ///
    /// Never: an instant that cannot be read prints as nothing, the way the
    /// memorial shows a hero whose stone carries no date.
    pub fn french_date<S: Display>(instant: S, _values: &dyn Values) -> Result<String, Infallible> {
        Ok(read_as_a_french_date(Some(&instant.to_string())))
    }
}
