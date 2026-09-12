//! The SQLite database: the connection, and every read and write.
//!
//! The only module here that talks to the database — `core::config` is pure and
//! `core::models` only describes the tables. Callers get plain values back:
//! nothing outside this module and `core::models` ever holds a row, which is
//! what keeps the routes free of the ORM.
//!
//! **Only the reading history, the play-throughs and the flagged pages are
//! stored.** The book is static — 668 pages that change only when the import
//! pipeline is re-run — so `core::story` reads it off disk into memory at
//! startup, and it is deliberately not in here: putting it in a database would
//! buy nothing.
//!
//! Every table is hybrid (see `core::models::hybrid_document`): a few real
//! columns for what a query filters, sorts or constrains, and one JSON blob for
//! the rest. A row is therefore read as one flat dict, changed as a dict, and
//! written back whole — the functions below never touch a column by hand. A
//! game's bag comes and goes the same way: its lines are rows of their own,
//! carried in the game's dict as `items`.
//!
//! The history is **scoped to a book**, named `"<series>/<book>"`: the functions
//! below take a `book`, and it is part of every query, so a second book never
//! shows up in the first one's history.

pub mod connection;
pub mod schema;
pub mod store;

use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde_json::{Map, Value};

pub use connection::{
    connect_db, connected_to, create_the_schema, engine, open_engine, open_memory_engine,
    opened_file, reset_connection, schema_version, table_names, DatabaseFailure, Db,
};
pub use schema::SCHEMA_VERSION;

use crate::core::character::{damage_adjustment, generate_character, named_mode, Mode, DEFAULT_MODE};
use crate::core::combat::{
    combat_status, has_special_rules, resolve_assault, Assault, Combatant, ENEMY, ONGOING,
};
use crate::core::dice::Rng;
use crate::core::inventory::{read_changes, starting_gold, Change, Holdings, STARTING_ITEMS};
use crate::core::logs::general_log::{event, note};
use crate::core::models::game::{entity as games, Game};
use crate::core::models::hybrid_document::{Field, HybridDocument, RowDict};
use crate::core::models::item::Item;
use crate::core::models::page_inspection::{entity as inspections, PageInspection, OPEN, STATUSES};
use crate::core::models::page_view::{entity as views, PageView};
use crate::core::models::utc_datetime::{to_isoformat, utc_now};
use crate::application::models::game::{
    Assault as AssaultDict, Combat as CombatDict, Enemy, Exchange, FallenHero, GameDict,
    Item as ItemDict, Pending,
};
use crate::application::models::inspection::{CommentDict, InspectionDict};
use crate::{row, vars};

/// How many pages of reading history are kept.
pub const MAX_PAGE_HISTORY: u64 = 10;

/// How many of the fallen the memorial remembers.
pub const MAX_FALLEN_HEROES: u64 = 20;

/// The row as one flat dict — the columns and the blob merged.
pub type State = RowDict;

/// A status neither open nor resolved was asked for.
#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    #[error("unknown inspection status {0:?}")]
    Unknown(String),
    #[error(transparent)]
    Database(#[from] DatabaseFailure),
}

// --- the reading history ------------------------------------------------------

/// Note that a page was asked for, unless it is already the last one read.
///
/// Asking for the page one is already on is a reload, not a move, and a trail
/// reading `22 › 22 › 22` says less than one reading `22`. Only *consecutive*
/// repeats are dropped: coming back to a page after going elsewhere is a loop in
/// the story, which is worth seeing.
///
/// # Returns
///
/// Whether a visit was actually recorded.
pub async fn record_page_view(
    db: &Db,
    book: &str,
    page: &str,
    game_id: Option<i64>,
) -> Result<bool, DatabaseFailure> {
    let connection = db.conn().await?;
    if latest_page(&connection, book).await?.as_deref() == Some(page) {
        return Ok(false);
    }
    let view = PageView::from_dict(&row! {
        "book" => book,
        "game_id" => game_id,
        "page" => page,
        "viewed_at" => utc_now(),
    });
    store::insert_view(&connection, &view).await?;
    Ok(true)
}

/// Return the pages most recently asked for, oldest first.
///
/// Bounding happens here rather than on write, so recording a visit stays a
/// plain insert.
///
/// # Returns
///
/// Up to `limit` page numbers, oldest first.
pub async fn last_pages(
    db: &Db,
    book: &str,
    limit: u64,
) -> Result<Vec<String>, DatabaseFailure> {
    let connection = db.conn().await?;
    let mut pages: Vec<String> = newest_first(views::Entity::find().filter(views::Column::Book.eq(book)))
        .limit(limit)
        .all(&connection)
        .await?
        .into_iter()
        .map(|model| store::view_of(model).page())
        .collect();
    pages.reverse();
    Ok(pages)
}

/// The story page one play-through was on last.
///
/// What the sheet offers as the way back; the sheet itself records no visit, so
/// this is still the last paragraph of the story that was open. Scoped to the
/// play-through and not to the book: two heroes of the same book each stopped
/// somewhere of their own, and sending the living one back to where a dead one
/// fell would be worse than offering nothing.
pub async fn last_page_read(
    db: &Db,
    game_id: i64,
) -> Result<Option<String>, DatabaseFailure> {
    let connection = db.conn().await?;
    Ok(
        newest_first(views::Entity::find().filter(views::Column::GameId.eq(game_id)))
            .limit(1)
            .one(&connection)
            .await?
            .map(|model| store::view_of(model).page()),
    )
}

/// Return the oldest retained page number.
pub async fn get_oldest_page(db: &Db, book: &str) -> Result<Option<String>, DatabaseFailure> {
    Ok(last_pages(db, book, MAX_PAGE_HISTORY)
        .await?
        .first()
        .cloned())
}

/// Return the page most recently read, or nothing when nothing has been.
async fn latest_page(
    connection: &DatabaseConnection,
    book: &str,
) -> Result<Option<String>, DatabaseFailure> {
    Ok(
        newest_first(views::Entity::find().filter(views::Column::Book.eq(book)))
            .limit(1)
            .one(connection)
            .await?
            .map(|model| store::view_of(model).page()),
    )
}

/// The history's one order: the most recent visit first, insertion order under it.
fn newest_first(select: sea_orm::Select<views::Entity>) -> sea_orm::Select<views::Entity> {
    select
        .order_by_desc(views::Column::ViewedAt)
        .order_by_desc(views::Column::Id)
}

// --- the play-through ---------------------------------------------------------

/// Roll up a hero and open a game for him.
///
/// The only place the hero's Force and Vie are ever rolled: every later read
/// loads what was written here, so reopening a game never re-rolls it.
///
/// # Arguments
///
/// * `mode` - How to roll the hero up — by the book, or the easy way.
/// * `rng` - Source of chance, seeded by the tests.
pub async fn start_game(
    db: &Db,
    book: &str,
    mode: Mode,
    rng: Option<&dyn Rng>,
) -> Result<GameDict, DatabaseFailure> {
    let character = generate_character(mode, rng);
    let (gold, gold_throws) = starting_gold(rng);
    let hero: State = row! {
        "book" => book,
        "mode" => mode.name,
        "force" => character.force,
        "vie_max" => character.vie_max,
        "vie_actuelle" => character.vie_actuelle,
        "force_dice" => dice_field(character.force_roll.dice()),
        "vie_dice" => dice_field(character.vie_roll.dice()),
        "gold" => gold,
        "gold_dice" => Field::List(
            gold_throws
                .iter()
                .flat_map(|throw| throw.dice().iter().map(|die| Field::Int(i64::from(*die))))
                .collect::<Vec<Field>>(),
        ),
        "items" => Field::List(
            STARTING_ITEMS
                .iter()
                .map(|(element, label, count)| {
                    Field::Dict(row! {
                        "element" => *element,
                        "label" => *label,
                        "count" => *count,
                    })
                })
                .collect::<Vec<Field>>(),
        ),
        "pending" => Field::List(Vec::new()),
        "combat" => Field::Null,
        "died_at" => Field::Null,
    };
    let connection = db.conn().await?;
    let written = store::insert_game(&connection, &Game::from_dict(&hero)).await?;
    let state = written.to_dict();
    event(
        "Hero rolled up",
        vars! {
            "game" => written.id(),
            "book" => book,
            "mode" => mode.name,
            "force" => character.force,
            "vie" => character.vie_max,
            "force_dice" => dice_field(character.force_roll.dice()),
            "vie_dice" => dice_field(character.vie_roll.dice()),
            "gold" => gold,
        },
    );
    Ok(game_dict(&state))
}

/// Load one game, or nothing — a blank id matches nothing.
pub async fn find_game(
    db: &Db,
    game_id: Option<i64>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let Some(game_id) = game_id.filter(|id| *id != 0) else {
        return Ok(None);
    };
    let connection = db.conn().await?;
    Ok(store::find_game_row(&connection, game_id)
        .await?
        .map(|game| game_dict(&game.to_dict())))
}

/// Put the hero into the fight a page holds, unless he is already in one.
///
/// Re-entering a page mid-fight must not restart it — a reload would otherwise
/// hand back full-health adversaries — so an open combat on the same page is
/// left exactly as it stands.
///
/// # Returns
///
/// The game with a combat on it, or nothing when the game is unknown or the
/// fight has no adversaries to field.
pub async fn begin_combat(
    db: &Db,
    game_id: Option<i64>,
    page: &str,
    fight: &Map<String, Value>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    if let Some(combat) = combat_of(&state) {
        if combat.get("page").and_then(Field::as_str) == Some(page) {
            return Ok(Some(game_dict(&state)));
        }
    }

    let enemies = combat_enemies(fight);
    if enemies.is_empty() {
        return Ok(None);
    }

    let empty = Map::new();
    let outcome = fight
        .get("outcome")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    let special = has_special_rules(page);
    let names: Vec<Field> = enemies
        .iter()
        .map(|enemy| enemy.get("name").cloned().unwrap_or(Field::Null))
        .collect();
    let fight_type = fight
        .get("fight_type")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    state.insert(
        "combat".to_string(),
        Field::Dict(row! {
            "page" => page,
            "fight_type" => fight_type.clone(),
            "enemies" => Field::List(enemies.into_iter().map(Field::Dict).collect::<Vec<Field>>()),
            "assaults" => Field::List(Vec::new()),
            "status" => ONGOING,
            "on_victory" => outcome_branch(outcome.get("on_victory")),
            "on_defeat" => outcome_branch(outcome.get("on_defeat")),
            "on_flee" => outcome_branch(outcome.get("on_flee")),
            "has_special_rules" => special,
        }),
    );
    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    event(
        "Combat armed",
        vars! {
            "game" => written.id(),
            "page" => page,
            "fight_type" => fight_type,
            "enemies" => Field::List(names),
            "special_rules" => special,
        },
    );
    Ok(Some(game_dict(&written.to_dict())))
}

/// Play one assault of the open fight and write the result back.
///
/// # Returns
///
/// The game after the assault, or nothing when there is no fight to advance —
/// no game, no combat, or one already decided.
pub async fn play_assault(
    db: &Db,
    game_id: Option<i64>,
    rng: Option<&dyn Rng>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    let Some(combat) = combat_of(&state) else {
        return Ok(None);
    };
    if combat.get("status").and_then(Field::as_str) != Some(ONGOING) {
        return Ok(None);
    }
    let mut combat = combat.clone();

    let stored_enemies: Vec<RowDict> = combat
        .get("enemies")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .iter()
        .map(|entry| entry.as_dict().cloned().unwrap_or_default())
        .collect();
    let mut assaults: Vec<Field> = combat
        .get("assaults")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .to_vec();

    let (hero, fought, assault) = resolve_assault(
        &hero_combatant(&state),
        &stored_enemies
            .iter()
            .map(enemy_combatant)
            .collect::<Vec<Combatant>>(),
        combat
            .get("fight_type")
            .and_then(Field::as_str)
            .unwrap_or(""),
        assaults.len() as i64 + 1,
        rng,
    );

    state.insert(
        "vie_actuelle".to_string(),
        Field::Int(hero.vie_actuelle),
    );
    let enemies: Vec<Field> = stored_enemies
        .iter()
        .zip(fought.iter())
        .map(|(stored, fought)| {
            let mut enemy = stored.clone();
            enemy.insert("vie_actuelle".to_string(), Field::Int(fought.vie_actuelle));
            Field::Dict(enemy)
        })
        .collect();
    assaults.push(Field::Dict(stored_assault(&assault)));
    let status = combat_status(&hero, &fought);
    combat.insert("enemies".to_string(), Field::List(enemies));
    combat.insert("assaults".to_string(), Field::List(assaults));
    combat.insert("status".to_string(), Field::Str(status.to_string()));
    let page = combat
        .get("page")
        .and_then(Field::as_str)
        .map(str::to_string);
    lay_to_rest(&mut state, page.as_deref(), &killed_by(&assault));
    state.insert("combat".to_string(), Field::Dict(combat.clone()));

    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    log_assault(written.id(), &combat, &assault, hero.vie_actuelle);
    Ok(Some(game_dict(&written.to_dict())))
}

/// Write an assault to the log, and the fight's outcome when it decided it.
fn log_assault(game_id: Option<i64>, combat: &RowDict, assault: &Assault, hero_vie: i64) {
    let page = combat.get("page").cloned().unwrap_or(Field::Null);
    note(
        "Assault played",
        vars! {
            "game" => game_id,
            "page" => page.clone(),
            "number" => assault.number,
            "hero_attack_force" => assault.hero_attack_force,
            "hero_vie" => hero_vie,
            "exchanges" => Field::List(
                assault
                    .exchanges
                    .iter()
                    .map(|exchange| {
                        Field::Dict(row! {
                            "enemy" => exchange.enemy_name.clone(),
                            "attack_force" => exchange.enemy_attack_force,
                            "winner" => exchange.winner,
                            "damage" => exchange.damage,
                            "divine_judgement" => exchange.divine_judgement,
                        })
                    })
                    .collect::<Vec<Field>>(),
            ),
        },
    );
    let status = combat.get("status").and_then(Field::as_str).unwrap_or("");
    if status != ONGOING {
        event(
            "Combat decided",
            vars! {
                "game" => game_id,
                "page" => page,
                "outcome" => status,
                "assaults" => combat
                    .get("assaults")
                    .and_then(Field::as_list)
                    .map_or(0, <[Field]>::len) as i64,
                "hero_vie" => hero_vie,
            },
        );
    }
}

/// Take the hero out of the fight, leaving his Vie where the fight left it.
pub async fn end_combat(
    db: &Db,
    game_id: Option<i64>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    let decided = combat_of(&state)
        .and_then(|combat| combat.get("status").cloned())
        .unwrap_or(Field::Null);
    state.insert("combat".to_string(), Field::Null);
    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    note(
        "Combat left",
        vars! {
            "game" => written.id(),
            "outcome" => decided,
            "hero_vie" => state.get("vie_actuelle").cloned().unwrap_or(Field::Null),
        },
    );
    Ok(Some(game_dict(&written.to_dict())))
}

/// The game as a row, for the functions that write it back.
///
/// The one place inside this module that fetches a row by id: everything public
/// converts before returning, so no caller ever holds one.
async fn game_row(
    connection: &DatabaseConnection,
    game_id: Option<i64>,
) -> Result<Option<Game>, DatabaseFailure> {
    let Some(game_id) = game_id.filter(|id| *id != 0) else {
        return Ok(None);
    };
    Ok(store::find_game_row(connection, game_id).await?)
}

/// The open fight in a game's state, or nothing while the hero is reading.
fn combat_of(state: &State) -> Option<&RowDict> {
    state.get("combat").and_then(Field::as_dict)
}

/// The hero as the combat engine fights him.
fn hero_combatant(state: &State) -> Combatant {
    let force = number(state.get("force"));
    Combatant::new(
        "Prêtre Jean",
        force,
        number(state.get("vie_max")),
        number(state.get("vie_actuelle")),
        damage_adjustment(force),
    )
}

/// One stored adversary as the combat engine fights it.
fn enemy_combatant(enemy: &RowDict) -> Combatant {
    Combatant::new(
        enemy
            .get("name")
            .cloned()
            .unwrap_or(Field::Null)
            .to_string(),
        number(enemy.get("force")),
        number(enemy.get("vie_max")),
        number(enemy.get("vie_actuelle")),
        number(enemy.get("damage_adjustment")),
    )
}

/// One field as the whole number it holds, absent reading as zero.
fn number(value: Option<&Field>) -> i64 {
    match value {
        Some(Field::Int(number)) => *number,
        Some(Field::Float(number)) => *number as i64,
        Some(Field::Bool(carried)) => i64::from(*carried),
        Some(Field::Str(text)) => text.parse().unwrap_or(0),
        _ => 0,
    }
}

/// A roll as the blob keeps it.
fn dice_field(dice: &[u32]) -> Field {
    Field::List(dice.iter().map(|die| Field::Int(i64::from(*die))).collect())
}

/// A list of whole numbers out of the blob.
fn numbers(value: Option<&Field>) -> Vec<i64> {
    value
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .iter()
        .map(|entry| number(Some(entry)))
        .collect()
}

/// A field as the text it holds, or nothing when it holds none.
fn optional_text(value: Option<&Field>) -> Option<String> {
    match value {
        None | Some(Field::Null) => None,
        Some(Field::Str(text)) => Some(text.clone()),
        Some(other) => Some(other.to_string()),
    }
}

/// Turn a game's stored state into what the routes and templates read.
fn game_dict(state: &State) -> GameDict {
    let mode = named_mode(optional_text(state.get("mode")).as_deref());
    let force = number(state.get("force"));
    GameDict {
        id: number(state.get("id")),
        book: optional_text(state.get("book")).unwrap_or_default(),
        mode: mode.name.to_string(),
        mode_label: mode.label.to_string(),
        force_throw: mode.force.notation(),
        vie_throw: mode.vie.notation(),
        force_base: mode.force.base,
        vie_base: mode.vie.base,
        force,
        vie_max: number(state.get("vie_max")),
        vie_actuelle: number(state.get("vie_actuelle")),
        force_dice: numbers(state.get("force_dice")),
        vie_dice: numbers(state.get("vie_dice")),
        damage_adjustment: damage_adjustment(force),
        gold: number(state.get("gold")),
        gold_dice: numbers(state.get("gold_dice")),
        bag: bag(state),
        pending: state
            .get("pending")
            .and_then(Field::as_list)
            .unwrap_or(&[])
            .iter()
            .enumerate()
            .map(|(index, waiting)| {
                pending_dict(index, waiting.as_dict().cloned().unwrap_or_default())
            })
            .collect(),
        combat: combat_of(state).map(combat_dict),
        is_dead: is_dead(state),
        died_on_page: optional_text(state.get("died_on_page")),
        died_of: optional_text(state.get("died_of")),
    }
}

/// What the hero carries, one line per kind of thing.
fn bag(state: &State) -> Vec<ItemDict> {
    state
        .get("items")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(Field::as_dict)
        .map(|item| ItemDict {
            element: optional_text(item.get("element")).unwrap_or_default(),
            label: optional_text(item.get("label")).unwrap_or_default(),
            count: number(item.get("count")),
        })
        .collect()
}

/// Whether the hero of this state has been laid to rest.
fn is_dead(state: &State) -> bool {
    state.get("died_at").is_some_and(|died_at| !died_at.is_null())
}

/// Turn an open fight into what the combat template reads.
fn combat_dict(combat: &RowDict) -> CombatDict {
    CombatDict {
        page: optional_text(combat.get("page")).unwrap_or_default(),
        fight_type: optional_text(combat.get("fight_type")).unwrap_or_default(),
        status: optional_text(combat.get("status")).unwrap_or_default(),
        on_victory: optional_text(combat.get("on_victory")),
        on_defeat: optional_text(combat.get("on_defeat")),
        on_flee: optional_text(combat.get("on_flee")),
        has_special_rules: matches!(combat.get("has_special_rules"), Some(Field::Bool(true))),
        enemies: combat
            .get("enemies")
            .and_then(Field::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(Field::as_dict)
            .map(|enemy| Enemy {
                name: optional_text(enemy.get("name")).unwrap_or_default(),
                force: number(enemy.get("force")),
                vie_max: number(enemy.get("vie_max")),
                vie_actuelle: number(enemy.get("vie_actuelle")),
                damage_adjustment: number(enemy.get("damage_adjustment")),
            })
            .collect(),
        assaults: combat
            .get("assaults")
            .and_then(Field::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(Field::as_dict)
            .map(|assault| AssaultDict {
                number: number(assault.get("number")),
                hero_dice: numbers(assault.get("hero_dice")),
                hero_attack_force: number(assault.get("hero_attack_force")),
                exchanges: assault
                    .get("exchanges")
                    .and_then(Field::as_list)
                    .unwrap_or(&[])
                    .iter()
                    .filter_map(Field::as_dict)
                    .map(|exchange| Exchange {
                        enemy_name: optional_text(exchange.get("enemy_name")).unwrap_or_default(),
                        enemy_force: number(exchange.get("enemy_force")),
                        enemy_dice: numbers(exchange.get("enemy_dice")),
                        enemy_attack_force: number(exchange.get("enemy_attack_force")),
                        winner: optional_text(exchange.get("winner")),
                        damage: number(exchange.get("damage")),
                        divine_judgement: matches!(
                            exchange.get("divine_judgement"),
                            Some(Field::Bool(true))
                        ),
                    })
                    .collect(),
            })
            .collect(),
    }
}

/// Field one adversary per body the fight puts in front of the hero.
///
/// A `count` on an adversary means the book prints one set of statistics for
/// several identical creatures ("LEPREUX ... count: 2"), so it is expanded into
/// that many adversaries, each with its own Vie to whittle down.
fn combat_enemies(fight: &Map<String, Value>) -> Vec<RowDict> {
    let mut enemies = Vec::new();
    let Some(entries) = fight.get("enemies").and_then(Value::as_array) else {
        return enemies;
    };
    for entry in entries {
        let Some(entry) = entry.as_object() else {
            continue;
        };
        let vie = whole(entry.get("vie"));
        let count = whole(entry.get("count"));
        for copy_number in 0..count.max(1) {
            enemies.push(row! {
                "name" => enemy_name(entry, copy_number, count),
                "force" => whole(entry.get("force")),
                "vie_max" => vie,
                "vie_actuelle" => vie,
                "damage_adjustment" => whole(entry.get("damage_adjustment")),
            });
        }
    }
    enemies
}

/// Name one adversary, numbering the copies of a repeated one apart.
fn enemy_name(entry: &Map<String, Value>, copy_number: i64, count: i64) -> String {
    let name = entry
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .unwrap_or("adversaire")
        .to_string();
    if count < 2 {
        return name;
    }
    format!("{name} {}", copy_number + 1)
}

/// One number a book page carries, absent or unreadable reading as zero.
fn whole(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(Value::String(text)) => text.parse().unwrap_or(0),
        Some(Value::Bool(carried)) => i64::from(*carried),
        _ => 0,
    }
}

/// The page (or `"death"`) an outcome points at, or nothing when it has none.
fn outcome_branch(value: Option<&Value>) -> Field {
    match value {
        None | Some(Value::Null) => Field::Null,
        Some(Value::String(text)) => Field::Str(text.clone()),
        Some(other) => Field::Str(other.to_string()),
    }
}

/// Turn an assault the engine produced into what the blob keeps of it.
fn stored_assault(assault: &Assault) -> RowDict {
    row! {
        "number" => assault.number,
        "hero_dice" => dice_field(assault.hero_roll.dice()),
        "hero_attack_force" => assault.hero_attack_force,
        "exchanges" => Field::List(
            assault
                .exchanges
                .iter()
                .map(|exchange| {
                    Field::Dict(row! {
                        "enemy_name" => exchange.enemy_name.clone(),
                        "enemy_force" => exchange.enemy_force,
                        "enemy_dice" => dice_field(exchange.enemy_roll.dice()),
                        "enemy_attack_force" => exchange.enemy_attack_force,
                        "winner" => exchange.winner,
                        "damage" => exchange.damage,
                        "divine_judgement" => exchange.divine_judgement,
                    })
                })
                .collect::<Vec<Field>>(),
        ),
    }
}

// --- what a choice costs and gives --------------------------------------------

/// Apply what following a choice costs and gives, and say what is left over.
///
/// The unconditional changes are applied here and now. A change carrying a
/// condition — or an amount the page asks to be rolled — is not: it goes to
/// `pending`, where the reader decides. See `core::inventory`.
pub async fn follow_choice(
    db: &Db,
    game_id: Option<i64>,
    page: &str,
    choice: &Map<String, Value>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let (waiting, applied): (Vec<Change>, Vec<Change>) = read_changes(choice)
        .into_iter()
        .partition(Change::is_conditional);

    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    if !applied.is_empty() {
        let before = holdings(&state);
        write_holdings(&mut state, before, &applied);
        lay_to_rest(&mut state, Some(page), "les épreuves du chemin");
    }
    state.insert(
        "pending".to_string(),
        Field::List(
            waiting
                .iter()
                .map(|change| Field::Dict(pending_entry(change, page)))
                .collect(),
        ),
    );
    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    note(
        "Choice followed",
        vars! {
            "game" => written.id(),
            "page" => page,
            "goto" => choice.get("goto").and_then(Value::as_str),
            "applied" => described(&applied),
            "pending" => described(&waiting),
        },
    );
    Ok(Some(game_dict(&written.to_dict())))
}

/// Every change as a reader reads it, for the log.
fn described(changes: &[Change]) -> Field {
    Field::List(
        changes
            .iter()
            .map(|change| Field::Str(change.described()))
            .collect(),
    )
}

/// Apply one waiting change, because the reader says it applies.
///
/// # Returns
///
/// The game afterwards, or nothing when the game or the change is unknown.
pub async fn apply_pending(
    db: &Db,
    game_id: Option<i64>,
    index: usize,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    let mut pending: Vec<Field> = state
        .get("pending")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .to_vec();
    if index >= pending.len() {
        return Ok(None);
    }

    let waiting = pending.remove(index).as_dict().cloned().unwrap_or_default();
    let change = pending_change(&waiting);
    let before = holdings(&state);
    write_holdings(&mut state, before, std::slice::from_ref(&change));
    lay_to_rest(
        &mut state,
        optional_text(waiting.get("page")).as_deref(),
        "les épreuves du chemin",
    );
    state.insert("pending".to_string(), Field::List(pending));
    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    note(
        "Pending change applied",
        vars! { "game" => written.id(), "element" => change.element.clone() },
    );
    Ok(Some(game_dict(&written.to_dict())))
}

/// Wave a waiting change away, or all of them.
///
/// # Arguments
///
/// * `index` - Which one; `None` for every one of them.
pub async fn dismiss_pending(
    db: &Db,
    game_id: Option<i64>,
    index: Option<usize>,
) -> Result<Option<GameDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let Some(game) = game_row(&connection, game_id).await? else {
        return Ok(None);
    };
    let mut state = game.to_dict();
    let mut pending: Vec<Field> = state
        .get("pending")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .to_vec();
    match index {
        None => pending.clear(),
        Some(index) if index < pending.len() => {
            pending.remove(index);
        }
        Some(_) => return Ok(None),
    }
    state.insert("pending".to_string(), Field::List(pending));
    let mut changed = game.clone();
    changed.update_from_dict(&state);
    let written = store::save_game(&connection, &game, &changed).await?;
    Ok(Some(game_dict(&written.to_dict())))
}

/// What killed the hero this assault, named for the memorial.
///
/// # Returns
///
/// The adversary that struck the last blow, or the fight itself when no exchange
/// settled it.
fn killed_by(assault: &Assault) -> String {
    assault
        .exchanges
        .iter()
        .rev()
        .find(|exchange| exchange.winner == Some(ENEMY))
        .map_or_else(|| "un combat".to_string(), |exchange| exchange.enemy_name.clone())
}

/// Stamp a hero's death, once, the moment his Vie reaches zero.
///
/// Called after every write that can empty it. A hero already laid to rest is
/// left alone: the first death is the one that counts, and nothing afterwards
/// should move the date on the stone.
fn lay_to_rest(state: &mut State, page: Option<&str>, cause: &str) {
    if number(state.get("vie_actuelle")) > 0 || is_dead(state) {
        return;
    }
    state.insert("died_at".to_string(), Field::Instant(utc_now()));
    state.insert(
        "died_on_page".to_string(),
        page.map_or(Field::Null, |page| Field::Str(page.to_string())),
    );
    state.insert("died_of".to_string(), Field::Str(cause.to_string()));
    event(
        "Hero fell",
        vars! {
            "game" => state.get("id").cloned().unwrap_or(Field::Null),
            "page" => page,
            "cause" => cause,
            "force" => state.get("force").cloned().unwrap_or(Field::Null),
            "gold" => number(state.get("gold")),
        },
    );
}

/// The heroes who did not come back, most recent first.
pub async fn fallen_heroes(
    db: &Db,
    book: &str,
    limit: u64,
) -> Result<Vec<FallenHero>, DatabaseFailure> {
    let connection = db.conn().await?;
    let select = games::Entity::find()
        .filter(games::Column::Book.eq(book))
        .filter(games::Column::DiedAt.is_not_null())
        .order_by_desc(games::Column::DiedAt)
        .order_by_desc(games::Column::Id)
        .limit(limit);
    Ok(store::game_rows(&connection, select)
        .await?
        .iter()
        .map(|game| fallen_dict(&game.to_dict()))
        .collect())
}

/// One dead hero as the memorial lists him, `died_at` being set.
fn fallen_dict(state: &State) -> FallenHero {
    FallenHero {
        id: number(state.get("id")),
        mode_label: named_mode(optional_text(state.get("mode")).as_deref())
            .label
            .to_string(),
        force: number(state.get("force")),
        vie_max: number(state.get("vie_max")),
        gold: number(state.get("gold")),
        bag: bag(state),
        died_on_page: optional_text(state.get("died_on_page")),
        died_of: optional_text(state.get("died_of")),
        died_at: state
            .get("died_at")
            .and_then(Field::as_instant)
            .map(to_isoformat),
    }
}

/// What the hero is and carries, as `core::inventory` reads it.
fn holdings(state: &State) -> Holdings {
    Holdings {
        force: number(state.get("force")),
        vie_max: number(state.get("vie_max")),
        vie_actuelle: number(state.get("vie_actuelle")),
        gold: number(state.get("gold")),
        items: state
            .get("items")
            .and_then(Field::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(Field::as_dict)
            .map(|item| {
                (
                    optional_text(item.get("element")).unwrap_or_default(),
                    (
                        optional_text(item.get("label")).unwrap_or_default(),
                        number(item.get("count")),
                    ),
                )
            })
            .collect(),
    }
}

/// Apply changes to a hero and write the result onto his state.
fn write_holdings(state: &mut State, before: Holdings, changes: &[Change]) {
    let mut after = before;
    for change in changes {
        after = after.with_change(change);
    }
    state.insert("force".to_string(), Field::Int(after.force));
    state.insert("vie_actuelle".to_string(), Field::Int(after.vie_actuelle));
    state.insert("gold".to_string(), Field::Int(after.gold));
    // Matched by element, so a line still in the bag stays the row it was.
    let carried: IndexMap<String, RowDict> = state
        .get("items")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .iter()
        .filter_map(Field::as_dict)
        .map(|item| {
            (
                optional_text(item.get("element")).unwrap_or_default(),
                item.clone(),
            )
        })
        .collect();
    state.insert(
        "items".to_string(),
        Field::List(
            after
                .items
                .iter()
                .map(|(element, (label, count))| {
                    let mut line = carried.get(element).cloned().unwrap_or_default();
                    line.insert("element".to_string(), Field::Str(element.clone()));
                    line.insert("label".to_string(), Field::Str(label.clone()));
                    line.insert("count".to_string(), Field::Int(*count));
                    Field::Dict(line)
                })
                .collect(),
        ),
    );
}

/// Keep a change the reader has yet to rule on.
fn pending_entry(change: &Change, page: &str) -> RowDict {
    row! {
        "element" => change.element.clone(),
        "label" => change.label.clone(),
        "amount" => change.amount,
        "condition" => change.condition.clone(),
        "note" => change.note.clone(),
        "sign" => change.sign,
        "page" => page,
    }
}

/// Read a waiting change back as `core::inventory` understands it.
fn pending_change(waiting: &RowDict) -> Change {
    Change {
        element: optional_text(waiting.get("element")).unwrap_or_default(),
        label: optional_text(waiting.get("label")).unwrap_or_default(),
        amount: waiting.get("amount").and_then(Field::as_int),
        condition: optional_text(waiting.get("condition")),
        note: optional_text(waiting.get("note")),
        sign: match waiting.get("sign").and_then(Field::as_int) {
            Some(0) | None => 1,
            Some(sign) => sign,
        },
    }
}

/// A waiting change as the page shows it, numbered so it can be acted on.
fn pending_dict(index: usize, waiting: RowDict) -> Pending {
    let change = pending_change(&waiting);
    Pending {
        index,
        element: change.element.clone(),
        label: change.label.clone(),
        amount: change.amount,
        condition: change.condition.clone(),
        note: change.note.clone(),
        sign: change.sign,
        page: optional_text(waiting.get("page")),
        described: change.described(),
    }
}

// --- the pages flagged for inspection -----------------------------------------

/// Flag a page for inspection, or add to the file already open on it.
///
/// Find-or-create on `(book, path)`: the first report opens the file, every
/// later one appends a comment. A resolved page that is flagged again is
/// reopened — a new remark means someone still sees a problem. The file's
/// `updated_at` moves with the new comment on its own.
///
/// # Arguments
///
/// * `text` - The remark, already checked to be non-blank by the caller.
pub async fn flag_page(
    db: &Db,
    book: &str,
    path: &str,
    page_title: Option<&str>,
    text: &str,
) -> Result<InspectionDict, DatabaseFailure> {
    let now = utc_now();
    let connection = db.conn().await?;
    let existing = inspections::Entity::find()
        .filter(inspections::Column::Book.eq(book))
        .filter(inspections::Column::Path.eq(path))
        .one(&connection)
        .await?
        .map(store::inspection_of);

    let mut state: State = match &existing {
        Some(inspection) => inspection.to_dict(),
        None => row! {
            "book" => book,
            "path" => path,
            "comments" => Field::List(Vec::new()),
        },
    };
    let kept = optional_text(state.get("page_title"));
    state.insert(
        "page_title".to_string(),
        page_title
            .map(str::to_string)
            .or(kept)
            .map_or(Field::Null, Field::Str),
    );
    state.insert("status".to_string(), Field::Str(OPEN.to_string()));
    let mut comments: Vec<Field> = state
        .get("comments")
        .and_then(Field::as_list)
        .unwrap_or(&[])
        .to_vec();
    comments.push(Field::Dict(row! {
        "text" => text,
        "created_at" => now,
    }));
    let count = comments.len() as i64;
    state.insert("comments".to_string(), Field::List(comments));

    let written = match existing {
        None => store::insert_inspection(&connection, &PageInspection::from_dict(&state)).await?,
        Some(before) => {
            let mut changed = before.clone();
            changed.update_from_dict(&state);
            store::save_inspection(&connection, &before, &changed).await?
        }
    };
    event(
        "Page flagged",
        vars! {
            "inspection" => written.id(),
            "book" => book,
            "path" => path,
            "comments" => count,
            "text" => text,
        },
    );
    Ok(inspection_dict(&written.to_dict()))
}

/// The pages flagged in one book, most recently commented first.
///
/// # Arguments
///
/// * `status` - Keep only the files in that state; `None` lists them all.
pub async fn flagged_pages(
    db: &Db,
    book: &str,
    status: Option<&str>,
) -> Result<Vec<InspectionDict>, DatabaseFailure> {
    let connection = db.conn().await?;
    let mut select = inspections::Entity::find().filter(inspections::Column::Book.eq(book));
    if let Some(status) = status {
        select = select.filter(inspections::Column::Status.eq(status));
    }
    Ok(select
        .order_by_desc(inspections::Column::UpdatedAt)
        .order_by_desc(inspections::Column::Id)
        .all(&connection)
        .await?
        .into_iter()
        .map(|model| inspection_dict(&store::inspection_of(model).to_dict()))
        .collect())
}

/// Load the file on one flagged page, or nothing when there is none.
pub async fn find_inspection(
    db: &Db,
    inspection_id: Option<i64>,
) -> Result<Option<InspectionDict>, DatabaseFailure> {
    let Some(inspection_id) = inspection_id.filter(|id| *id != 0) else {
        return Ok(None);
    };
    let connection = db.conn().await?;
    Ok(inspections::Entity::find_by_id(inspection_id)
        .one(&connection)
        .await?
        .map(|model| inspection_dict(&store::inspection_of(model).to_dict())))
}

/// Mark a flagged page resolved, or open it again.
///
/// Setting the state a file already has writes nothing, and so leaves it where
/// it stands in the list.
///
/// # Errors
///
/// If the status is neither open nor resolved.
pub async fn set_inspection_status(
    db: &Db,
    inspection_id: i64,
    status: &str,
) -> Result<Option<InspectionDict>, StatusError> {
    if !STATUSES.contains(&status) {
        return Err(StatusError::Unknown(status.to_string()));
    }
    let connection = db.conn().await.map_err(StatusError::Database)?;
    let found = inspections::Entity::find_by_id(inspection_id)
        .one(&connection)
        .await
        .map_err(|trouble| StatusError::Database(trouble.into()))?;
    let Some(before) = found.map(store::inspection_of) else {
        return Ok(None);
    };
    let mut state = before.to_dict();
    state.insert("status".to_string(), Field::Str(status.to_string()));
    let mut changed = before.clone();
    changed.update_from_dict(&state);
    let written = store::save_inspection(&connection, &before, &changed)
        .await
        .map_err(|trouble| StatusError::Database(trouble.into()))?;
    event(
        "Flagged page status changed",
        vars! {
            "inspection" => written.id(),
            "path" => written.path(),
            "status" => status,
        },
    );
    Ok(Some(inspection_dict(&written.to_dict())))
}

/// One flagged page as the routes read it.
///
/// The comments' instants are already ISO 8601 text in the blob; the file's own
/// two are columns, and are printed here.
fn inspection_dict(state: &State) -> InspectionDict {
    InspectionDict {
        id: number(state.get("id")),
        book: optional_text(state.get("book")).unwrap_or_default(),
        path: optional_text(state.get("path")).unwrap_or_default(),
        page_title: optional_text(state.get("page_title")),
        status: optional_text(state.get("status")).unwrap_or_default(),
        comments: state
            .get("comments")
            .and_then(Field::as_list)
            .unwrap_or(&[])
            .iter()
            .filter_map(Field::as_dict)
            .map(|comment| CommentDict {
                text: optional_text(comment.get("text")).unwrap_or_default(),
                created_at: optional_text(comment.get("created_at")).unwrap_or_default(),
            })
            .collect(),
        created_at: printed_instant(state.get("created_at")),
        updated_at: printed_instant(state.get("updated_at")),
    }
}

/// A column instant as the routes hand it out.
fn printed_instant(value: Option<&Field>) -> String {
    value
        .and_then(Field::as_instant)
        .map(to_isoformat)
        .unwrap_or_default()
}

/// The mode a name asks for, for the callers that only have a name.
pub fn mode_named(name: Option<&str>) -> Mode {
    named_mode(name)
}

/// The mode a game is rolled with when nothing says otherwise.
pub const fn default_mode() -> Mode {
    DEFAULT_MODE
}

/// Delete one game and everything that hangs off it, which only a test needs.
pub async fn delete_game(db: &Db, game_id: i64) -> Result<(), DatabaseFailure> {
    let connection = db.conn().await?;
    store::delete_game(&connection, game_id).await?;
    Ok(())
}

/// Every stored instant, for the callers that print one.
pub fn as_isoformat(instant: DateTime<Utc>) -> String {
    to_isoformat(instant)
}

/// One line of a bag, for the tests that seed one.
pub fn bag_line(element: &str, label: &str, count: i64) -> Item {
    Item::from_dict(&row! {
        "element" => element,
        "label" => label,
        "count" => count,
    })
}
