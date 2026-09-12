//! Reading a row as one flat dict, and writing a changed one back whole.
//!
//! The plumbing under every function of `core::db`: it never sets a column by
//! hand, it loads a row, turns it into a dict, changes the dict and hands it
//! back here. What that costs is spelled out once:
//!
//! - a row whose columns and blob are unchanged is **not written**, so its
//!   `updated_at` does not move;
//! - a row that is written moves its `updated_at` to now, unless the dict names
//!   one itself;
//! - a row inserted is stamped `created_at`, and `updated_at` at that same
//!   instant, so a row nobody has changed says so by the two being equal;
//! - a game's bag rides along: a line naming an id is that row, a line without
//!   one is a new row, and a line left out is deleted.

use chrono::{DateTime, Utc};
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{
    ActiveValue, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
};
use serde_json::Value;

use crate::core::models::game::{entity as games, Game};
use crate::core::models::hybrid_document::{Field, HybridDocument, RowDict};
use crate::core::models::item::{entity as items, Item};
use crate::core::models::page_inspection::{entity as inspections, PageInspection};
use crate::core::models::page_view::{entity as views, PageView};
use crate::core::models::utc_datetime::{to_text, utc_now};

/// One column of a row, as the driver binds it.
type Bound = ActiveValue<Option<String>>;

/// Read a stored column back into the flat dict.
fn text_field(value: Option<String>) -> Field {
    value.map_or(Field::Null, Field::Str)
}

/// Read a stored timestamp back as the instant it is.
fn instant_field(value: Option<String>) -> Field {
    match value {
        None => Field::Null,
        Some(text) => crate::core::models::utc_datetime::from_text(&text)
            .map_or(Field::Str(text), Field::Instant),
    }
}

/// Read a stored whole number back into the flat dict.
fn int_field(value: Option<i64>) -> Field {
    value.map_or(Field::Null, Field::Int)
}

/// Read a stored blob back into the flat dict.
fn blob(value: Option<Value>) -> RowDict {
    match value {
        Some(Value::Object(entries)) => entries
            .iter()
            .map(|(key, entry)| (key.clone(), Field::from_json(entry)))
            .collect(),
        _ => RowDict::new(),
    }
}

/// Write the blob as the file stores it: JSON, accents kept.
fn blob_value(data: &RowDict) -> Value {
    Field::Dict(data.clone()).to_json()
}

/// Bind a column that holds text.
fn bind_text(value: &Field) -> Option<String> {
    match value {
        Field::Null => None,
        Field::Str(text) => Some(text.clone()),
        other => Some(other.to_string()),
    }
}

/// Bind a column that holds a timestamp, an instant written at one width.
fn bind_instant(value: &Field) -> Option<String> {
    match value {
        Field::Null => None,
        Field::Instant(instant) => Some(to_text(*instant)),
        other => Some(other.to_string()),
    }
}

/// Bind a column that holds a whole number.
fn bind_int(value: &Field) -> Option<i64> {
    match value {
        Field::Null => None,
        Field::Int(number) => Some(*number),
        Field::Float(number) => Some(*number as i64),
        Field::Bool(carried) => Some(i64::from(*carried)),
        Field::Str(text) => text.parse().ok(),
        _ => None,
    }
}

/// What a row is stamped with when it is written.
struct Stamps {
    created_at: Option<String>,
    updated_at: Option<String>,
}

/// The two instants a freshly inserted row carries.
///
/// Both are the same instant, so a row nobody has changed says so; a dict that
/// names either keeps what it named.
fn stamps_for_insert(columns: &RowDict, now: DateTime<Utc>) -> Stamps {
    let created_at = columns
        .get("created_at")
        .filter(|value| !value.is_null())
        .map_or_else(|| to_text(now), |value| bind_instant(value).unwrap_or_else(|| to_text(now)));
    let updated_at = columns
        .get("updated_at")
        .filter(|value| !value.is_null())
        .map_or_else(|| created_at.clone(), |value| {
            bind_instant(value).unwrap_or_else(|| created_at.clone())
        });
    Stamps {
        created_at: Some(created_at),
        updated_at: Some(updated_at),
    }
}

/// The `updated_at` a row being changed carries.
///
/// Moved to now, unless the dict names one of its own — which is what an update
/// that sets the column explicitly does.
fn stamp_for_update(before: &Field, after: &Field, now: DateTime<Utc>) -> Option<String> {
    if !after.is_null() && after != before {
        return bind_instant(after);
    }
    Some(to_text(now))
}

/// Whether a failure is the database refusing a row rather than a query failing.
pub fn is_integrity_error(trouble: &DbErr) -> bool {
    let written = trouble.to_string();
    written.contains("constraint failed") || written.contains("UNIQUE constraint")
}

// --- games ------------------------------------------------------------------

/// One game as a row, its bag with it.
pub async fn find_game_row(
    connection: &DatabaseConnection,
    game_id: i64,
) -> Result<Option<Game>, DbErr> {
    let Some(model) = games::Entity::find_by_id(game_id).one(connection).await? else {
        return Ok(None);
    };
    Ok(Some(game_of(model, bag_of(connection, game_id).await?)))
}

/// Every game a query names, their bags with them.
pub async fn game_rows(
    connection: &DatabaseConnection,
    select: sea_orm::Select<games::Entity>,
) -> Result<Vec<Game>, DbErr> {
    let models = select.all(connection).await?;
    let mut rows = Vec::with_capacity(models.len());
    for model in models {
        let bag = bag_of(connection, model.id).await?;
        rows.push(game_of(model, bag));
    }
    Ok(rows)
}

/// The lines of one game's bag, in the order they were first carried.
async fn bag_of(connection: &DatabaseConnection, game_id: i64) -> Result<Vec<Item>, DbErr> {
    Ok(items::Entity::find()
        .filter(items::Column::GameId.eq(game_id))
        .order_by_asc(items::Column::Id)
        .all(connection)
        .await?
        .into_iter()
        .map(item_of)
        .collect())
}

/// One stored game as the flat dict the callers read.
fn game_of(model: games::Model, bag: Vec<Item>) -> Game {
    let mut row = Game::empty();
    row.set_column("id", &Field::Int(model.id));
    row.set_column("book", &text_field(model.book));
    row.set_column("force", &int_field(model.force));
    row.set_column("vie_max", &int_field(model.vie_max));
    row.set_column("vie_actuelle", &int_field(model.vie_actuelle));
    row.set_column("gold", &int_field(model.gold));
    row.set_column("died_at", &instant_field(model.died_at));
    row.set_column("created_at", &instant_field(model.created_at));
    row.set_column("updated_at", &instant_field(model.updated_at));
    row.set_data(blob(model.data));
    row.items = bag;
    row
}

/// One stored bag line as the flat dict the callers read, for a caller outside.
pub fn item_row(model: items::Model) -> Item {
    item_of(model)
}

/// One stored bag line as the flat dict the callers read.
fn item_of(model: items::Model) -> Item {
    let mut row = Item::empty();
    row.set_column("id", &Field::Int(model.id));
    row.set_column("game_id", &int_field(model.game_id));
    row.set_column("created_at", &instant_field(model.created_at));
    row.set_column("updated_at", &instant_field(model.updated_at));
    row.set_data(blob(model.data));
    row
}

/// Write a game the database has never seen, its bag with it.
pub async fn insert_game(
    connection: &DatabaseConnection,
    game: &Game,
) -> Result<Game, DbErr> {
    let now = utc_now();
    let columns = one_dict(game);
    let stamps = stamps_for_insert(&columns, now);
    let active = games::ActiveModel {
        id: match game.id() {
            Some(id) => ActiveValue::Set(id),
            None => ActiveValue::NotSet,
        },
        book: ActiveValue::Set(bind_text(&game.column("book"))),
        force: ActiveValue::Set(bind_int(&game.column("force"))),
        vie_max: ActiveValue::Set(bind_int(&game.column("vie_max"))),
        vie_actuelle: ActiveValue::Set(bind_int(&game.column("vie_actuelle"))),
        // The purse is the one stat with a default: an unset one is an empty one.
        gold: ActiveValue::Set(Some(bind_int(&game.column("gold")).unwrap_or(0))),
        died_at: ActiveValue::Set(bind_instant(&game.column("died_at"))),
        created_at: ActiveValue::Set(stamps.created_at),
        updated_at: ActiveValue::Set(stamps.updated_at),
        data: ActiveValue::Set(Some(blob_value(game.data()))),
    };
    let written = games::Entity::insert(active)
        .exec_with_returning(connection)
        .await?;
    let id = written.id;
    write_the_bag(connection, id, &game.items, &[], now).await?;
    Ok(game_of(written, bag_of(connection, id).await?))
}

/// Write a game back, and its bag with it, when either has changed.
pub async fn save_game(
    connection: &DatabaseConnection,
    before: &Game,
    after: &Game,
) -> Result<Game, DbErr> {
    let id = after
        .id()
        .or_else(|| before.id())
        .ok_or_else(|| DbErr::Custom("a stored game always carries its id".to_string()))?;
    let now = utc_now();
    if columns_changed(before, after, games_columns()) || before.data() != after.data() {
        let active = games::ActiveModel {
            id: ActiveValue::Unchanged(id),
            book: set_text(before, after, "book"),
            force: set_int(before, after, "force"),
            vie_max: set_int(before, after, "vie_max"),
            vie_actuelle: set_int(before, after, "vie_actuelle"),
            gold: set_int(before, after, "gold"),
            died_at: set_instant(before, after, "died_at"),
            created_at: set_instant(before, after, "created_at"),
            updated_at: ActiveValue::Set(stamp_for_update(
                &before.column("updated_at"),
                &after.column("updated_at"),
                now,
            )),
            data: if before.data() == after.data() {
                ActiveValue::NotSet
            } else {
                ActiveValue::Set(Some(blob_value(after.data())))
            },
        };
        games::Entity::update(active).exec(connection).await?;
    }
    write_the_bag(connection, id, &after.items, &before.items, now).await?;
    find_game_row(connection, id)
        .await?
        .ok_or_else(|| DbErr::Custom("the game just written is gone".to_string()))
}

/// The columns of a game, for the comparison that says whether to write.
///
/// `updated_at` is among them: a dict that names one of its own is asking for
/// that instant to be written, which is a change like any other.
fn games_columns() -> &'static [&'static str] {
    &[
        "book",
        "force",
        "vie_max",
        "vie_actuelle",
        "gold",
        "died_at",
        "created_at",
        "updated_at",
    ]
}

/// Delete one game and everything that hangs off it.
pub async fn delete_game(connection: &DatabaseConnection, id: i64) -> Result<(), DbErr> {
    items::Entity::delete_many()
        .filter(items::Column::GameId.eq(id))
        .exec(connection)
        .await?;
    views::Entity::update_many()
        .col_expr(views::Column::GameId, SimpleExpr::Value(sea_orm::Value::BigInt(None)))
        .filter(views::Column::GameId.eq(id))
        .exec(connection)
        .await?;
    games::Entity::delete_by_id(id).exec(connection).await?;
    Ok(())
}

/// Bring one game's bag into step with the lines it now carries.
///
/// A line naming an id is that row, a line without one is a new row, and a line
/// left out is deleted — the cascade the relationship used to do.
async fn write_the_bag(
    connection: &DatabaseConnection,
    game_id: i64,
    lines: &[Item],
    before: &[Item],
    now: DateTime<Utc>,
) -> Result<(), DbErr> {
    let kept: Vec<i64> = lines.iter().filter_map(Item::id).collect();
    for line in before {
        if let Some(id) = line.id() {
            if !kept.contains(&id) {
                items::Entity::delete_by_id(id).exec(connection).await?;
            }
        }
    }
    for line in lines {
        match line.id() {
            None => {
                let columns = one_dict(line);
                let stamps = stamps_for_insert(&columns, now);
                items::Entity::insert(items::ActiveModel {
                    id: ActiveValue::NotSet,
                    game_id: ActiveValue::Set(Some(
                        bind_int(&line.column("game_id")).unwrap_or(game_id),
                    )),
                    created_at: ActiveValue::Set(stamps.created_at),
                    updated_at: ActiveValue::Set(stamps.updated_at),
                    data: ActiveValue::Set(Some(blob_value(line.data()))),
                })
                .exec(connection)
                .await?;
            }
            Some(id) => {
                let was = before.iter().find(|line| line.id() == Some(id));
                let unchanged = was.is_some_and(|was| {
                    was.data() == line.data()
                        && was.column("game_id") == line.column("game_id")
                        && was.column("created_at") == line.column("created_at")
                });
                if unchanged {
                    continue;
                }
                let empty = Item::empty();
                let was = was.unwrap_or(&empty);
                items::Entity::update(items::ActiveModel {
                    id: ActiveValue::Unchanged(id),
                    game_id: ActiveValue::Set(Some(
                        bind_int(&line.column("game_id")).unwrap_or(game_id),
                    )),
                    created_at: set_instant(was, line, "created_at"),
                    updated_at: ActiveValue::Set(stamp_for_update(
                        &was.column("updated_at"),
                        &line.column("updated_at"),
                        now,
                    )),
                    data: ActiveValue::Set(Some(blob_value(line.data()))),
                })
                .exec(connection)
                .await?;
            }
        }
    }
    Ok(())
}

/// Write one bag line the database has never seen, on its own.
pub async fn insert_item(connection: &DatabaseConnection, line: &Item) -> Result<Item, DbErr> {
    let now = utc_now();
    let columns = one_dict(line);
    let stamps = stamps_for_insert(&columns, now);
    let written = items::Entity::insert(items::ActiveModel {
        id: match line.id() {
            Some(id) => ActiveValue::Set(id),
            None => ActiveValue::NotSet,
        },
        game_id: ActiveValue::Set(bind_int(&line.column("game_id"))),
        created_at: ActiveValue::Set(stamps.created_at),
        updated_at: ActiveValue::Set(stamps.updated_at),
        data: ActiveValue::Set(Some(blob_value(line.data()))),
    })
    .exec_with_returning(connection)
    .await?;
    Ok(item_of(written))
}

// --- page views --------------------------------------------------------------

/// One stored visit as the flat dict the callers read.
pub fn view_of(model: views::Model) -> PageView {
    let mut row = PageView::empty();
    row.set_column("id", &Field::Int(model.id));
    row.set_column("book", &text_field(model.book));
    row.set_column("game_id", &int_field(model.game_id));
    row.set_column("viewed_at", &instant_field(model.viewed_at));
    row.set_data(blob(model.data));
    row
}

/// Write a visit, which is only ever an insert.
pub async fn insert_view(
    connection: &DatabaseConnection,
    view: &PageView,
) -> Result<PageView, DbErr> {
    let written = views::Entity::insert(views::ActiveModel {
        id: match view.id() {
            Some(id) => ActiveValue::Set(id),
            None => ActiveValue::NotSet,
        },
        book: ActiveValue::Set(bind_text(&view.column("book"))),
        game_id: ActiveValue::Set(bind_int(&view.column("game_id"))),
        viewed_at: ActiveValue::Set(bind_instant(&view.column("viewed_at"))),
        data: ActiveValue::Set(Some(blob_value(view.data()))),
    })
    .exec_with_returning(connection)
    .await?;
    Ok(view_of(written))
}

// --- flagged pages -----------------------------------------------------------

/// One stored flagged page as the flat dict the callers read.
pub fn inspection_of(model: inspections::Model) -> PageInspection {
    let mut row = PageInspection::empty();
    row.set_column("id", &Field::Int(model.id));
    row.set_column("book", &text_field(model.book));
    row.set_column("path", &text_field(model.path));
    row.set_column("status", &text_field(model.status));
    row.set_column("created_at", &instant_field(model.created_at));
    row.set_column("updated_at", &instant_field(model.updated_at));
    row.set_data(blob(model.data));
    row
}

/// Write a flagged page the database has never seen.
pub async fn insert_inspection(
    connection: &DatabaseConnection,
    inspection: &PageInspection,
) -> Result<PageInspection, DbErr> {
    let now = utc_now();
    let columns = one_dict(inspection);
    let stamps = stamps_for_insert(&columns, now);
    let written = inspections::Entity::insert(inspections::ActiveModel {
        id: match inspection.id() {
            Some(id) => ActiveValue::Set(id),
            None => ActiveValue::NotSet,
        },
        book: ActiveValue::Set(bind_text(&inspection.column("book"))),
        path: ActiveValue::Set(bind_text(&inspection.column("path"))),
        status: ActiveValue::Set(bind_text(&inspection.column("status"))),
        created_at: ActiveValue::Set(stamps.created_at),
        updated_at: ActiveValue::Set(stamps.updated_at),
        data: ActiveValue::Set(Some(blob_value(inspection.data()))),
    })
    .exec_with_returning(connection)
    .await?;
    Ok(inspection_of(written))
}

/// Write a flagged page back, when anything about it has changed.
pub async fn save_inspection(
    connection: &DatabaseConnection,
    before: &PageInspection,
    after: &PageInspection,
) -> Result<PageInspection, DbErr> {
    let id = after
        .id()
        .or_else(|| before.id())
        .ok_or_else(|| DbErr::Custom("a stored file always carries its id".to_string()))?;
    let columns = ["book", "path", "status", "created_at", "updated_at"];
    if !columns_changed(before, after, &columns) && before.data() == after.data() {
        return inspections::Entity::find_by_id(id)
            .one(connection)
            .await?
            .map(inspection_of)
            .ok_or_else(|| DbErr::Custom("the file just read is gone".to_string()));
    }
    let written = inspections::Entity::update(inspections::ActiveModel {
        id: ActiveValue::Unchanged(id),
        book: set_text(before, after, "book"),
        path: set_text(before, after, "path"),
        status: set_text(before, after, "status"),
        created_at: set_instant(before, after, "created_at"),
        updated_at: ActiveValue::Set(stamp_for_update(
            &before.column("updated_at"),
            &after.column("updated_at"),
            utc_now(),
        )),
        data: ActiveValue::Set(Some(blob_value(after.data()))),
    })
    .exec(connection)
    .await?;
    Ok(inspection_of(written))
}

// --- what every write shares -------------------------------------------------

/// Whether any of these columns differs between the row loaded and the row written.
fn columns_changed<M: HybridDocument>(before: &M, after: &M, columns: &[&str]) -> bool {
    columns
        .iter()
        .any(|name| before.column(name) != after.column(name))
}

/// One row's columns alone, for the stamping that reads them.
fn one_dict<M: HybridDocument>(row: &M) -> RowDict {
    M::indexed_columns()
        .iter()
        .map(|name| ((*name).to_string(), row.column(name)))
        .collect()
}

/// Bind a text column, leaving it alone when it has not moved.
fn set_text<M: HybridDocument>(before: &M, after: &M, name: &str) -> Bound {
    if before.column(name) == after.column(name) {
        return ActiveValue::NotSet;
    }
    ActiveValue::Set(bind_text(&after.column(name)))
}

/// Bind a timestamp column, leaving it alone when it has not moved.
fn set_instant<M: HybridDocument>(before: &M, after: &M, name: &str) -> Bound {
    if before.column(name) == after.column(name) {
        return ActiveValue::NotSet;
    }
    ActiveValue::Set(bind_instant(&after.column(name)))
}

/// Bind a number column, leaving it alone when it has not moved.
fn set_int<M: HybridDocument>(before: &M, after: &M, name: &str) -> ActiveValue<Option<i64>> {
    if before.column(name) == after.column(name) {
        return ActiveValue::NotSet;
    }
    ActiveValue::Set(bind_int(&after.column(name)))
}

/// Read a limit the way a query takes one.
pub fn limited<E: EntityTrait>(select: sea_orm::Select<E>, limit: Option<u64>) -> sea_orm::Select<E> {
    match limit {
        Some(limit) => select.limit(limit),
        None => select,
    }
}
