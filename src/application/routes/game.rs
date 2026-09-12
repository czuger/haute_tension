//! The routes a play-through needs: creating a hero, his sheet, and the fights.
//!
//! The site header is a menu of four entries, and three of them land here:
//! **Nouveau** (`/game/new`) offers to roll a hero up, **Partie en cours**
//! (`/game/resume`) puts the reader back where his adventure stands, **Feuille**
//! (`/game`) is the sheet, and **Les tombés** (`/heroes`) the memorial.
//!
//! The session cookie carries nothing but a game id; everything else is loaded
//! from the database on each request, so two tabs of the same browser are the
//! same hero and nothing about him lives in the cookie.
//!
//! Unlike the reader, these routes **need** the database — a fight cannot be
//! rolled without somewhere to keep its wounds — so they report a failure rather
//! than degrading. `/book/<n>` stays readable without one; only combat does not.

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use serde_json::Value;

use crate::application::errors::AppError;
use crate::application::models::game::{Combat, GameDict, SESSION_KEY};
use crate::application::routes::web::OPENING_PAGE;
use crate::application::routes::{whole_number, AppState, FormData, Reader};
use crate::application::templates::{
    CharacterTemplate, CombatTemplate, DeathTemplate, HeroesTemplate, NewGameTemplate,
};
use crate::core::character::{named_mode, MODES};
use crate::core::combat::{DEFEAT, ONGOING, VICTORY};
use crate::core::db::{self, Db, MAX_FALLEN_HEROES};

pub const DEATH: &str = "death";

/// Roll up Prêtre Jean and show the sheet the dice made.
///
/// The mode comes off the form. An unknown one rolls a hero by the book rather
/// than refusing to roll one: a posted value is whatever was posted.
pub async fn create_game(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: FormData,
) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game/new");
    let mode = named_mode(form.get("mode"));
    match db::start_game(&state.db, &state.book, mode, state.dice()).await {
        Err(trouble) => reader.refuse(&AppError::Database(trouble)),
        Ok(game) => {
            reader.session.set(SESSION_KEY, Value::from(game.id));
            reader.redirect("/game?rolled=1")
        }
    }
}

/// Offer to roll a hero up, saying what it costs when one is alive.
///
/// The menu's "Nouveau". Nothing is written here: the dice are only thrown by
/// the POST, so the page is the confirmation. A living hero is shown as he
/// stands, because creating another one leaves him behind — his game stays in
/// the database, but the session forgets him.
pub async fn offer_game(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game/new");
    match offer_a_hero(&state, &mut reader).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// Put the reader back where his adventure stands.
///
/// The menu's "Partie en cours": the fight he is in, the death page if he fell,
/// the last paragraph he read, or the opening page of a hero who has read
/// nothing. Without a hero, the offer to roll one up.
pub async fn resume_game(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game/resume");
    match where_it_stands(&state, &mut reader).await {
        Ok(destination) => reader.redirect(&destination),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The address the adventure is picked up at.
async fn where_it_stands(state: &AppState, reader: &mut Reader) -> Result<String, AppError> {
    let Some(game) = current_game(state, reader).await? else {
        return Ok("/game/new".to_string());
    };
    if let Some(combat) = &game.combat {
        return Ok(format!("/combat/{}", combat.page));
    }
    if game.is_dead {
        return Ok("/game/death".to_string());
    }
    let page = db::last_page_read(&state.db, game.id)
        .await?
        .unwrap_or_else(|| OPENING_PAGE.to_string());
    Ok(format!("/book/{page}"))
}

/// What the sheet was reached with.
#[derive(serde::Deserialize, Default)]
pub struct SheetQuery {
    pub rolled: Option<String>,
}

/// Show the hero's sheet, or offer to roll one up.
///
/// Everything about him on one page — his Vie, his Force, his purse and his bag
/// — because that is what a sheet is. It carries the way back to the story too:
/// it is reached mid-adventure, not instead of one.
pub async fn show_character(
    State(state): State<AppState>,
    Query(query): Query<SheetQuery>,
    headers: HeaderMap,
) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game");
    match render_sheet(&state, &mut reader, query.rolled.as_deref() == Some("1")).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The sheet, or the offer of a hero when the session has none.
async fn render_sheet(
    state: &AppState,
    reader: &mut Reader,
    rolled: bool,
) -> Result<String, AppError> {
    let Some(game) = current_game(state, reader).await? else {
        return offer_a_hero(state, reader).await;
    };
    let back_page = db::last_page_read(&state.db, game.id).await?;
    let gauge = gauge_of(&game);
    let layout = reader.layout(format!("Feuille de personnage · {}", state.book_title));
    Ok(CharacterTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        game,
        rolled,
        back_page,
        gauge,
    }
    .render()?)
}

/// How full the Vie gauge is drawn, rounded the way the page rounded it.
///
/// Half a point rounds up, which is what the template's `round` filter did; a
/// hero with no Vie to start with has no gauge to draw.
fn gauge_of(game: &GameDict) -> i64 {
    if game.vie_max <= 0 {
        return 0;
    }
    let share = 100.0 * game.vie_actuelle as f64 / game.vie_max as f64;
    (share + 0.5).floor() as i64
}

/// Remember the heroes who did not come back.
pub async fn show_fallen(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let mut reader = Reader::of(&state, &headers, "/heroes");
    match render_memorial(&state, &mut reader).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The memorial, and who is reading it.
async fn render_memorial(state: &AppState, reader: &mut Reader) -> Result<String, AppError> {
    let fallen = db::fallen_heroes(&state.db, &state.book, MAX_FALLEN_HEROES).await?;
    let game = current_game(state, reader).await?;
    let layout = reader.layout(format!("Ceux qui vous ont précédé · {}", state.book_title));
    Ok(HeroesTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        fallen,
        game,
    }
    .render()?)
}

/// Show the fight a page holds, arming it on first arrival.
pub async fn show_combat(
    State(state): State<AppState>,
    Path(number): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/combat/{number}");
    let mut reader = Reader::of(&state, &headers, &path);
    match render_combat(&state, &mut reader, &number).await {
        Ok(Some(body)) => reader.html(body),
        Ok(None) => reader.redirect("/game"),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The fight, or nothing when the reader has no hero to fight it with.
async fn render_combat(
    state: &AppState,
    reader: &mut Reader,
    number: &str,
) -> Result<Option<String>, AppError> {
    let Some(number) = whole_number(number) else {
        return Err(AppError::NotFound);
    };
    let number = number.to_string();
    let Some(page) = state.story.get(&number) else {
        return Err(AppError::NotFound);
    };
    if !page.fight_has_enemies() {
        return Err(AppError::NotFound);
    }
    let Some(game) = current_game(state, reader).await? else {
        return Ok(None);
    };
    let fight = page.fight().cloned().unwrap_or_default();
    let armed = db::begin_combat(&state.db, Some(game.id), &number, &fight).await?;
    let Some(game) = armed else {
        return Err(AppError::NotFound);
    };
    let Some(combat) = game.combat.clone() else {
        return Err(AppError::NotFound);
    };
    let layout = reader.layout(format!(
        "Combat · page {} · {}",
        combat.page, state.book_title
    ));
    Ok(Some(
        CombatTemplate {
            layout,
            book_series: state.book_series.clone(),
            book_title: state.book_title.clone(),
            page_number: combat.page.clone(),
            game,
            combat,
        }
        .render()?,
    ))
}

/// Play one assault, then come back to the fight.
///
/// The fight is armed here too, not only on the way in: `begin_combat` leaves an
/// open fight on the same page untouched, so this costs nothing in the browser
/// and keeps the route from depending on a GET having happened first.
pub async fn fight_assault(
    State(state): State<AppState>,
    Path(number): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/combat/{number}/assault");
    let mut reader = Reader::of(&state, &headers, &path);
    match play_one_assault(&state, &mut reader, &number).await {
        Ok(()) => reader.redirect(&format!("/combat/{number}")),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// Advance the fight the page holds, when there is one and a hero to fight it.
async fn play_one_assault(
    state: &AppState,
    reader: &mut Reader,
    number: &str,
) -> Result<(), AppError> {
    let Some(number) = whole_number(number) else {
        return Err(AppError::NotFound);
    };
    let number = number.to_string();
    let game = current_game(state, reader).await?;
    let page = state.story.get(&number);
    let (Some(game), Some(page)) = (game, page) else {
        return Ok(());
    };
    if !page.has_fight() {
        return Ok(());
    }
    let fight = page.fight().cloned().unwrap_or_default();
    if db::begin_combat(&state.db, Some(game.id), &number, &fight)
        .await?
        .is_some()
    {
        db::play_assault(&state.db, Some(game.id), state.dice()).await?;
    }
    Ok(())
}

/// Close a decided fight and follow the book to what comes next.
pub async fn leave_combat(
    State(state): State<AppState>,
    Path(number): Path<String>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/combat/{number}/resolve");
    let mut reader = Reader::of(&state, &headers, &path);
    match follow_the_book(&state, &mut reader, &number).await {
        Ok(destination) => reader.redirect(&destination),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// Where the book sends the hero once this fight is left behind.
async fn follow_the_book(
    state: &AppState,
    reader: &mut Reader,
    number: &str,
) -> Result<String, AppError> {
    let Some(number) = whole_number(number) else {
        return Err(AppError::NotFound);
    };
    let back_to_the_page = format!("/book/{number}");
    let game = current_game(state, reader).await?;
    let Some(combat) = game.as_ref().and_then(|game| game.combat.clone()) else {
        return Ok(back_to_the_page);
    };
    let game = game.expect("a combat only ever hangs off a game");

    let destination = decided_destination(&combat);
    if combat.status != ONGOING {
        db::end_combat(&state.db, Some(game.id)).await?;
    }
    Ok(match destination.as_deref() {
        Some(DEATH) => "/game/death".to_string(),
        None => back_to_the_page,
        Some(page) => format!("/book/{page}"),
    })
}

/// Where the book sends the hero once this fight is decided.
///
/// # Returns
///
/// A page number, `"death"`, or nothing when the book records no branch — which
/// the parser left empty often enough to be worth handling.
fn decided_destination(combat: &Combat) -> Option<String> {
    if combat.status == VICTORY {
        return combat.on_victory.clone();
    }
    if combat.status == DEFEAT {
        return Some(
            combat
                .on_defeat
                .clone()
                .unwrap_or_else(|| DEATH.to_string()),
        );
    }
    None
}

/// Follow one choice of a page, paying what it costs.
///
/// A POST and not a link: following a choice spends rations and gold, and the
/// redirect afterwards is what keeps a reload from spending them twice.
pub async fn take_choice(
    State(state): State<AppState>,
    Path((number, index)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let path = format!("/book/{number}/choice/{index}");
    let mut reader = Reader::of(&state, &headers, &path);
    match pay_for_the_choice(&state, &mut reader, &number, &index).await {
        Ok(destination) => reader.redirect(&destination),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// Charge one choice to the hero, and say where it leads.
async fn pay_for_the_choice(
    state: &AppState,
    reader: &mut Reader,
    number: &str,
    index: &str,
) -> Result<String, AppError> {
    let (Some(number), Some(index)) = (whole_number(number), whole_number(index)) else {
        return Err(AppError::NotFound);
    };
    let number = number.to_string();
    let Some(page) = state.story.get(&number) else {
        return Err(AppError::NotFound);
    };
    let choices = page.choices();
    let Some(choice) = usize::try_from(index).ok().and_then(|index| choices.get(index)) else {
        return Err(AppError::NotFound);
    };
    let choice = (*choice).clone();
    if let Some(game) = current_game(state, reader).await? {
        db::follow_choice(&state.db, Some(game.id), &number, &choice).await?;
    }
    let goto = choice
        .get("goto")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    Ok(format!("/book/{goto}"))
}

/// Apply a waiting change, because the reader says it applies.
pub async fn apply_change(
    State(state): State<AppState>,
    Path(index): Path<String>,
    headers: HeaderMap,
    form: FormData,
) -> Response {
    let path = format!("/game/pending/{index}/apply");
    let mut reader = Reader::of(&state, &headers, &path);
    let ruling = rule_on_a_change(&state, &mut reader, &index, true).await;
    answer_the_ruling(&reader, ruling, &form)
}

/// Wave one waiting change away.
pub async fn dismiss_change(
    State(state): State<AppState>,
    Path(index): Path<String>,
    headers: HeaderMap,
    form: FormData,
) -> Response {
    let path = format!("/game/pending/{index}/dismiss");
    let mut reader = Reader::of(&state, &headers, &path);
    let ruling = rule_on_a_change(&state, &mut reader, &index, false).await;
    answer_the_ruling(&reader, ruling, &form)
}

/// Wave every waiting change away.
pub async fn dismiss_all_changes(
    State(state): State<AppState>,
    headers: HeaderMap,
    form: FormData,
) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game/pending/dismiss");
    let ruling = match current_game(&state, &mut reader).await {
        Err(trouble) => Err(trouble),
        Ok(None) => Ok(()),
        Ok(Some(game)) => db::dismiss_pending(&state.db, Some(game.id), None)
            .await
            .map(|_| ())
            .map_err(AppError::Database),
    };
    answer_the_ruling(&reader, ruling, &form)
}

/// Apply or dismiss one waiting change, when there is a hero to rule for.
async fn rule_on_a_change(
    state: &AppState,
    reader: &mut Reader,
    index: &str,
    applies: bool,
) -> Result<(), AppError> {
    let Some(index) = whole_number(index).and_then(|index| usize::try_from(index).ok()) else {
        return Err(AppError::NotFound);
    };
    let Some(game) = current_game(state, reader).await? else {
        return Ok(());
    };
    if applies {
        db::apply_pending(&state.db, Some(game.id), index).await?;
    } else {
        db::dismiss_pending(&state.db, Some(game.id), Some(index)).await?;
    }
    Ok(())
}

/// Send the reader back where he ruled from, or say the database refused.
fn answer_the_ruling(reader: &Reader, ruling: Result<(), AppError>, form: &FormData) -> Response {
    match ruling {
        Ok(()) => reader.redirect(&back_to_the_reader(form)),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The page the reader was on, or the sheet when the form said nothing.
///
/// The waiting changes are shown on a story page and acted on from there, so the
/// answer to a form posted from it is that same page.
fn back_to_the_reader(form: &FormData) -> String {
    let number = form.trimmed("page");
    match whole_number(&number) {
        Some(number) => format!("/book/{number}"),
        None => "/game".to_string(),
    }
}

/// Tell the reader the adventure is over, and offer another hero.
pub async fn show_death(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let mut reader = Reader::of(&state, &headers, "/game/death");
    match render_death(&state, &mut reader).await {
        Ok(body) => reader.html(body),
        Err(trouble) => reader.refuse(&trouble),
    }
}

/// The end of the adventure, and the dice for the next one.
async fn render_death(state: &AppState, reader: &mut Reader) -> Result<String, AppError> {
    let game = current_game(state, reader).await?;
    let layout = reader.layout(format!("Mort · {}", state.book_title));
    Ok(DeathTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        game,
        modes: MODES.to_vec(),
    }
    .render()?)
}

/// Render the page that rolls a hero up.
async fn offer_a_hero(state: &AppState, reader: &mut Reader) -> Result<String, AppError> {
    let game = current_game(state, reader).await?;
    let fallen = db::fallen_heroes(&state.db, &state.book, MAX_FALLEN_HEROES).await?;
    let layout = reader.layout(format!("Créer votre héros · {}", state.book_title));
    Ok(NewGameTemplate {
        layout,
        book_series: state.book_series.clone(),
        book_title: state.book_title.clone(),
        game,
        modes: MODES.to_vec(),
        fallen,
    }
    .render()?)
}

/// The game the session is on, or nothing when it has none or it is stale.
async fn current_game(
    state: &AppState,
    reader: &mut Reader,
) -> Result<Option<GameDict>, AppError> {
    let game = find_the_session_game(&state.db, reader).await?;
    if game.is_none() {
        reader.session.remove(SESSION_KEY);
    }
    Ok(game)
}

/// The game an id in the session names.
async fn find_the_session_game(
    db: &Db,
    reader: &Reader,
) -> Result<Option<GameDict>, AppError> {
    Ok(db::find_game(db, reader.session.get_id(SESSION_KEY)).await?)
}
