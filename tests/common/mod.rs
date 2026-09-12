//! The fixtures every test file shares: the database, and an app on top of it.
//!
//! Every test runs against an in-memory SQLite database of its own: [`fake_db`]
//! opens one, creates the tables and hands the routes a `Db` bound to it — the
//! one seam that would otherwise open a file of its own — so nothing here
//! follows `.env`. A SQLite file and an in-memory SQLite are the same engine, so
//! there is no second backend to run the suite against.

#![allow(dead_code)]

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use axum::body::Body;
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::Router;
use bytes::Bytes;
use chrono::{DateTime, TimeZone, Utc};
use haute_tension::application::factory::{create_app, AppConfig};
use haute_tension::application::session::{Session, COOKIE_NAME};
use haute_tension::core::db::{store, Db};
use haute_tension::core::dice::Rng;
use haute_tension::core::models::game::{entity as games, Game};
use haute_tension::core::models::hybrid_document::{Field, HybridDocument, RowDict};
use haute_tension::core::models::item::{entity as items, Item};
use haute_tension::core::models::page_inspection::{entity as inspections, PageInspection};
use haute_tension::core::models::page_view::{entity as views, PageView};
use haute_tension::core::models::utc_datetime::to_text;
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryOrder,
    Statement,
};
use serde_json::Value;
use tower::ServiceExt;

pub const BOOK: &str = "pretre_jean/forteresse_alamuth";
pub const TEST_SECRET: &[u8] = b"test-secret";

/// A `random.Random` stand-in handing out faces in a set order.
pub struct FixedDice(Mutex<Vec<u32>>);

impl FixedDice {
    /// Build dice that come up in this order.
    pub fn new(faces: &[u32]) -> Self {
        Self(Mutex::new(faces.to_vec()))
    }
}

impl Rng for FixedDice {
    fn randint(&self, _low: u32, _high: u32) -> u32 {
        let mut faces = self.0.lock().expect("the dice outlive a panic");
        if faces.is_empty() {
            panic!("the fixed dice ran out of faces");
        }
        faces.remove(0)
    }
}

/// Dice handed out in a set order.
pub fn fixed(faces: &[u32]) -> FixedDice {
    FixedDice::new(faces)
}

/// A seeded generator, the way a test pins a run.
pub fn seeded(seed: u64) -> haute_tension::core::dice::SeededRng {
    haute_tension::core::dice::SeededRng::new(seed)
}

/// The lock every test that reads the environment takes.
///
/// Environment variables belong to the process, so two tests changing them at
/// once would read each other's.
pub fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The lock every test that listens to the log takes, for the same reason.
pub fn log_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// An environment variable set for the length of a test, and put back after.
pub struct EnvVar {
    name: String,
    was: Option<String>,
}

impl EnvVar {
    /// Set one variable.
    pub fn set(name: &str, value: &str) -> Self {
        let was = std::env::var(name).ok();
        std::env::set_var(name, value);
        Self {
            name: name.to_string(),
            was,
        }
    }

    /// Take one variable out of the environment.
    pub fn unset(name: &str) -> Self {
        let was = std::env::var(name).ok();
        std::env::remove_var(name);
        Self {
            name: name.to_string(),
            was,
        }
    }
}

impl Drop for EnvVar {
    fn drop(&mut self) {
        match &self.was {
            Some(value) => std::env::set_var(&self.name, value),
            None => std::env::remove_var(&self.name),
        }
    }
}

/// `.env` is read once per process; keep tests independent of the real one.
pub fn clean_env() -> Vec<EnvVar> {
    haute_tension::core::config::load_dotenv();
    haute_tension::core::db::reset_connection();
    vec![EnvVar::unset("APP_ENV"), EnvVar::unset("DATABASE_DIR")]
}

/// The database, indexed by table name.
pub struct FakeDb {
    pub connection: DatabaseConnection,
}

impl FakeDb {
    /// One table, seen as the list of flat dicts tests set up.
    pub fn table(&self, name: &'static str) -> FakeTable<'_> {
        FakeTable {
            connection: &self.connection,
            name,
        }
    }

    /// The database as the routes hold it.
    pub fn db(&self) -> Db {
        Db::Bound(self.connection.clone())
    }
}

/// Bind the routes to an in-memory database of their own, empty.
pub async fn fake_db() -> FakeDb {
    let connection = haute_tension::core::db::open_memory_engine()
        .await
        .expect("an in-memory database always opens");
    haute_tension::core::db::create_the_schema(&connection)
        .await
        .expect("a new database always takes the schema");
    FakeDb { connection }
}

/// One table, seen as the list of flat dicts tests set up.
///
/// `docs` is the whole interface: reading gives every row as `to_dict()`, the
/// columns and the blob merged; writing replaces the table's contents. A row
/// keyed by an id the new list also carries is updated in place rather than
/// deleted and re-inserted, so the reading history that points at a game
/// survives that game being changed under it. A game's dict carries its `items`,
/// so seeding a game seeds its bag: an item with an id is that row, one without
/// is a new row, and one left out is deleted.
pub struct FakeTable<'a> {
    connection: &'a DatabaseConnection,
    name: &'static str,
}

impl FakeTable<'_> {
    /// Every row of the table, as one flat dict each.
    pub async fn docs(&self) -> Vec<RowDict> {
        match self.name {
            "games" => {
                let select = games::Entity::find().order_by_asc(games::Column::Id);
                store::game_rows(self.connection, select)
                    .await
                    .expect("reading a table always works")
                    .iter()
                    .map(HybridDocument::to_dict)
                    .collect()
            }
            "items" => items::Entity::find()
                .order_by_asc(items::Column::Id)
                .all(self.connection)
                .await
                .expect("reading a table always works")
                .into_iter()
                .map(|model| store::item_row(model).to_dict())
                .collect(),
            "page_views" => views::Entity::find()
                .order_by_asc(views::Column::Id)
                .all(self.connection)
                .await
                .expect("reading a table always works")
                .into_iter()
                .map(|model| store::view_of(model).to_dict())
                .collect(),
            "page_inspections" => inspections::Entity::find()
                .order_by_asc(inspections::Column::Id)
                .all(self.connection)
                .await
                .expect("reading a table always works")
                .into_iter()
                .map(|model| store::inspection_of(model).to_dict())
                .collect(),
            other => panic!("no table named {other}"),
        }
    }

    /// Replace the table's contents, refusing whatever the database refuses.
    ///
    /// # Errors
    ///
    /// Whatever the database says: a constraint refused, a reference that leads
    /// nowhere.
    pub async fn set_docs(&self, docs: Vec<RowDict>) -> Result<(), DbErr> {
        match self.name {
            "games" => self.write_games(docs).await,
            "items" => self.write_items(docs).await,
            "page_views" => self.write_views(docs).await,
            "page_inspections" => self.write_inspections(docs).await,
            other => panic!("no table named {other}"),
        }
    }

    /// Replace the table's contents, and expect the database to accept them.
    pub async fn seed(&self, docs: Vec<RowDict>) {
        self.set_docs(docs)
            .await
            .expect("the seeded rows are ones the database accepts");
    }

    /// The one row a test seeded, for the many tests that seed exactly one.
    pub async fn one(&self) -> RowDict {
        let docs = self.docs().await;
        assert_eq!(docs.len(), 1, "expected exactly one row in {}", self.name);
        docs.into_iter().next().expect("just counted one")
    }

    async fn write_games(&self, docs: Vec<RowDict>) -> Result<(), DbErr> {
        let kept: Vec<i64> = docs.iter().filter_map(named_id).collect();
        for existing in games::Entity::find().all(self.connection).await? {
            if !kept.contains(&existing.id) {
                store::delete_game(self.connection, existing.id).await?;
            }
        }
        for doc in docs {
            let after = Game::from_dict(&doc);
            match named_id(&doc) {
                Some(id) => match store::find_game_row(self.connection, id).await? {
                    Some(before) => {
                        store::save_game(self.connection, &before, &after).await?;
                    }
                    None => {
                        store::insert_game(self.connection, &after).await?;
                    }
                },
                None => {
                    store::insert_game(self.connection, &after).await?;
                }
            }
        }
        Ok(())
    }

    async fn write_items(&self, docs: Vec<RowDict>) -> Result<(), DbErr> {
        let kept: Vec<i64> = docs.iter().filter_map(named_id).collect();
        for existing in items::Entity::find().all(self.connection).await? {
            if !kept.contains(&existing.id) {
                items::Entity::delete_by_id(existing.id)
                    .exec(self.connection)
                    .await?;
            }
        }
        for doc in docs {
            store::insert_item(self.connection, &Item::from_dict(&doc)).await?;
        }
        Ok(())
    }

    async fn write_views(&self, docs: Vec<RowDict>) -> Result<(), DbErr> {
        let kept: Vec<i64> = docs.iter().filter_map(named_id).collect();
        for existing in views::Entity::find().all(self.connection).await? {
            if !kept.contains(&existing.id) {
                views::Entity::delete_by_id(existing.id)
                    .exec(self.connection)
                    .await?;
            }
        }
        for doc in docs {
            store::insert_view(self.connection, &PageView::from_dict(&doc)).await?;
        }
        Ok(())
    }

    async fn write_inspections(&self, docs: Vec<RowDict>) -> Result<(), DbErr> {
        let kept: Vec<i64> = docs.iter().filter_map(named_id).collect();
        for existing in inspections::Entity::find().all(self.connection).await? {
            if !kept.contains(&existing.id) {
                inspections::Entity::delete_by_id(existing.id)
                    .exec(self.connection)
                    .await?;
            }
        }
        for doc in docs {
            let after = PageInspection::from_dict(&doc);
            match named_id(&doc) {
                Some(id) => match inspections::Entity::find_by_id(id)
                    .one(self.connection)
                    .await?
                    .map(store::inspection_of)
                {
                    Some(before) => {
                        store::save_inspection(self.connection, &before, &after).await?;
                    }
                    None => {
                        store::insert_inspection(self.connection, &after).await?;
                    }
                },
                None => {
                    store::insert_inspection(self.connection, &after).await?;
                }
            }
        }
        Ok(())
    }
}

/// The id a seeded dict names, when it names one.
fn named_id(doc: &RowDict) -> Option<i64> {
    doc.get("id").and_then(Field::as_int)
}

/// Run one statement against a test's database, for what has no query builder.
pub async fn execute(connection: &DatabaseConnection, sql: &str) -> Result<(), DbErr> {
    connection
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql.to_string(),
        ))
        .await
        .map(|_| ())
}

/// One value read straight out of a test's database.
pub async fn scalar(connection: &DatabaseConnection, sql: &str) -> Option<String> {
    connection
        .query_one(Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            sql.to_string(),
        ))
        .await
        .ok()
        .flatten()
        .and_then(|row| row.try_get_by_index::<String>(0).ok())
}

/// A small navigable book on disk, so no test reads the packaged one.
pub fn books_path() -> tempfile::TempDir {
    let directory = tempfile::tempdir().expect("a temporary directory is always made");
    let book_path = directory.path().join(BOOK);
    std::fs::create_dir_all(&book_path).expect("the book directory is always made");
    std::fs::write(
        book_path.join("pages.json"),
        serde_json::to_string(&serde_json::json!([
            {
                "page": "1",
                "language": "fr",
                "text": ["Le début de l'aventure.", "Deuxième ligne."],
                "file_path": "raw_data/1.html",
                "choices": [{"goto": "2", "gains": [], "losses": []}],
            },
            {
                "page": "2",
                "language": "fr",
                "text": ["La fin de l'aventure."],
                "file_path": "raw_data/2.html",
                "choices": [],
            },
        ]))
        .expect("the book is always written"),
    )
    .expect("the book file is always written");
    directory
}

/// The same book, with a fight on page 1 and a melee on page 2.
pub fn books_path_with_fights() -> tempfile::TempDir {
    let directory = books_path();
    let path = directory.path().join(BOOK).join("pages.json");
    let mut pages: Vec<Value> = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("the book was just written"),
    )
    .expect("the book was just written");
    pages[0]["fight"] = serde_json::json!({
        "fight_type": "single",
        "enemies": [{"name": "collecteur", "force": 6, "vie": 10}],
        "outcome": {"on_victory": "2", "on_defeat": "death", "on_flee": null},
    });
    pages[0]["choices"] = serde_json::json!([
        {
            "goto": "2",
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
            "losses": [{"element": "gold coin", "label_fr": "pièce d'or", "amount": 3}],
        },
        {
            "goto": "4",
            "gains": [
                {
                    "element": "strength point",
                    "label_fr": "point de Force",
                    "amount": 1,
                    "condition": "pendant tout le temps où vous les porterez",
                }
            ],
            "losses": [],
        },
    ]);
    pages[1]["fight"] = serde_json::json!({
        "fight_type": "simultaneous",
        "enemies": [{"name": "lepreux", "force": 6, "vie": 6, "count": 2}],
        "outcome": {"on_victory": "1", "on_defeat": "death", "on_flee": null},
    });
    pages.push(serde_json::json!({
        "page": "3",
        "language": "fr",
        "text": ["Une page dont le combat est mal analysé."],
        "file_path": "raw_data/3.html",
        "choices": [],
        // Truthy, so the page looks like it holds a fight, but nothing in it
        // can be fielded — which the parser does produce.
        "fight": {"fight_type": "single", "enemies": ["orc"], "outcome": {}},
    }));
    pages.push(serde_json::json!({
        "page": "4",
        "language": "fr",
        "text": ["Une page paisible."],
        "file_path": "raw_data/4.html",
        "choices": [],
    }));
    std::fs::write(
        &path,
        serde_json::to_string(&pages).expect("the book is always written"),
    )
    .expect("the book file is always written");
    directory
}

/// One answer, as a test reads it.
pub struct TestResponse {
    pub status: StatusCode,
    pub headers: HeaderMap,
    pub body: Bytes,
}

impl TestResponse {
    /// The answer's body as text.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The answer's body, parsed.
    pub fn json(&self) -> Value {
        serde_json::from_slice(&self.body).unwrap_or(Value::Null)
    }

    /// One header of the answer.
    pub fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    }

    /// Where a redirect sends.
    pub fn location(&self) -> String {
        self.header("location").unwrap_or_default()
    }

    /// The kind of thing the answer is, without its charset.
    pub fn mimetype(&self) -> String {
        self.header("content-type")
            .unwrap_or_default()
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    }

    /// The whole `Content-Type`, charset and all.
    pub fn content_type(&self) -> String {
        self.header("content-type").unwrap_or_default()
    }
}

/// A test client on top of the router, keeping its cookies between requests.
pub struct TestClient {
    router: Router,
    state: haute_tension::application::routes::AppState,
    cookies: Mutex<HashMap<String, String>>,
    secret: Vec<u8>,
    /// Held so the book on disk outlives the client reading it.
    _books: Option<tempfile::TempDir>,
}

impl TestClient {
    /// Ask for one address.
    pub async fn get(&self, path: &str) -> TestResponse {
        self.send(Method::GET, path, None, &[]).await
    }

    /// Ask for one address, saying what kind of answer is wanted.
    pub async fn get_with(&self, path: &str, headers: &[(&str, &str)]) -> TestResponse {
        self.send(Method::GET, path, None, headers).await
    }

    /// Post nothing at all, the way a button with no fields does.
    pub async fn post(&self, path: &str) -> TestResponse {
        self.post_form(path, &[]).await
    }

    /// Post a form.
    pub async fn post_form(&self, path: &str, fields: &[(&str, &str)]) -> TestResponse {
        self.post_form_with(path, fields, &[]).await
    }

    /// Post a form, saying what kind of answer is wanted.
    pub async fn post_form_with(
        &self,
        path: &str,
        fields: &[(&str, &str)],
        headers: &[(&str, &str)],
    ) -> TestResponse {
        let body = serde_urlencoded::to_string(fields).unwrap_or_default();
        let mut all = vec![("content-type", "application/x-www-form-urlencoded")];
        all.extend_from_slice(headers);
        self.send(Method::POST, path, Some(Body::from(body)), &all)
            .await
    }

    /// Post a body of some other kind entirely.
    pub async fn post_raw(
        &self,
        path: &str,
        body: Vec<u8>,
        content_type: &str,
    ) -> TestResponse {
        self.send(
            Method::POST,
            path,
            Some(Body::from(body)),
            &[("content-type", content_type)],
        )
        .await
    }

    /// Post JSON.
    pub async fn post_json(&self, path: &str, body: Value) -> TestResponse {
        self.post_raw(path, body.to_string().into_bytes(), "application/json")
            .await
    }

    /// Send one request and keep whatever cookie comes back.
    pub async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<Body>,
        headers: &[(&str, &str)],
    ) -> TestResponse {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        if let Some(cookie) = self.cookie_header() {
            request = request.header(header::COOKIE, cookie);
        }
        let request = request
            .body(body.unwrap_or_else(Body::empty))
            .expect("the request is always built");
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .expect("the router always answers");
        let (parts, body) = response.into_parts();
        self.keep_the_cookies(&parts.headers);
        let body = axum::body::to_bytes(body, usize::MAX)
            .await
            .unwrap_or_default();
        TestResponse {
            status: parts.status,
            headers: parts.headers,
            body,
        }
    }

    /// The state the router was built with, for a test that wires its own.
    pub fn state(&self) -> haute_tension::application::routes::AppState {
        self.state.clone()
    }

    /// The session the browser is carrying.
    pub fn session(&self) -> Session {
        Session::decode(self.cookie_header().as_deref(), &self.secret)
    }

    /// Write one value into the session the browser carries.
    pub fn set_session(&self, key: &str, value: Value) {
        let mut session = self.session();
        session.set(key, value);
        let cookie = session.encode(&self.secret);
        let value = cookie
            .split(';')
            .next()
            .and_then(|pair| pair.split_once('='))
            .map(|(_, value)| value.to_string())
            .unwrap_or_default();
        self.cookies
            .lock()
            .expect("the cookies outlive a panic")
            .insert(COOKIE_NAME.to_string(), value);
    }

    /// The `Cookie` header this browser sends.
    fn cookie_header(&self) -> Option<String> {
        let cookies = self.cookies.lock().expect("the cookies outlive a panic");
        if cookies.is_empty() {
            return None;
        }
        Some(
            cookies
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<String>>()
                .join("; "),
        )
    }

    /// Keep whatever the answer set.
    fn keep_the_cookies(&self, headers: &HeaderMap) {
        for value in headers.get_all(header::SET_COOKIE) {
            let Ok(written) = value.to_str() else { continue };
            let Some(pair) = written.split(';').next() else {
                continue;
            };
            let Some((name, value)) = pair.split_once('=') else {
                continue;
            };
            self.cookies
                .lock()
                .expect("the cookies outlive a panic")
                .insert(name.trim().to_string(), value.trim().to_string());
        }
    }
}

/// Build a test client for a book on disk, with a database behind it.
pub fn client_for(
    db: Db,
    books: tempfile::TempDir,
    rng: Option<Arc<dyn Rng>>,
) -> TestClient {
    let books_root = books.path().to_path_buf();
    let config = AppConfig {
        book: BOOK.to_string(),
        books_path: books_root.clone(),
        rng: rng.clone(),
        db: db.clone(),
        secret: TEST_SECRET.to_vec(),
    };
    let router = create_app(config).expect("the test book is always readable");
    let story = haute_tension::core::story::load_story(&books_root.join(BOOK))
        .expect("the test book is always readable");
    let state = haute_tension::application::routes::AppState {
        db,
        story: Arc::new(story),
        book: BOOK.to_string(),
        book_series: haute_tension::application::factory::BOOK_SERIES.to_string(),
        book_title: haute_tension::application::factory::BOOK_TITLE.to_string(),
        rng,
        secret: Arc::new(TEST_SECRET.to_vec()),
    };
    TestClient {
        router,
        state,
        cookies: Mutex::new(HashMap::new()),
        secret: TEST_SECRET.to_vec(),
        _books: Some(books),
    }
}

/// A test client for an app serving the small book, with a database behind it.
pub async fn client() -> (FakeDb, TestClient) {
    let db = fake_db().await;
    let client = client_for(db.db(), books_path(), None);
    (db, client)
}

/// A client whose book has fights, with the dice fixed for the whole app.
pub async fn fighting_client() -> (FakeDb, TestClient) {
    let db = fake_db().await;
    let client = client_for(
        db.db(),
        books_path_with_fights(),
        Some(Arc::new(seeded(1))),
    );
    (db, client)
}

/// A client that already has a hero rolled up.
pub async fn hero() -> (FakeDb, TestClient) {
    let (db, client) = fighting_client().await;
    client.post("/game/new").await;
    (db, client)
}

/// A client whose every read and write fails the way an unopenable file looks.
pub fn dead_database_client(books: tempfile::TempDir) -> TestClient {
    client_for(
        Db::Failing("unable to open database file".to_string()),
        books,
        None,
    )
}

/// A client with nothing configured to reach in the first place.
pub fn no_database_client(books: tempfile::TempDir) -> TestClient {
    client_for(
        Db::Unavailable("Missing DATABASE_DIR environment variable.".to_string()),
        books,
        None,
    )
}

/// One instant, spelled out.
pub fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("the instant is a real one")
}

/// The fixed-width text one instant is stored as.
pub fn stored(instant: DateTime<Utc>) -> String {
    to_text(instant)
}

/// The directory the packaged book lives in.
pub fn packaged_books() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("books")
}
