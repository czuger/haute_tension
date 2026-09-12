# Repository Guidelines

## Project Structure & Module Organization

The application is an Axum service in `src/`. `src/bin/serve.rs` is the
development-server entry point, `src/application/` contains the app factory and
the handlers, and `templates/` contains the browser UI, rendered by Askama.

`src/core/` is everything below the web layer, and nothing in it knows about
Axum or HTTP. It is strictly layered and the imports only go one way: `config.rs`
(environment variables, session key, which database file), `story.rs` (the book,
read off disk), `dice.rs` / `character.rs` / `combat.rs` (the game rules, all
pure), `models/` (one type per table, describing shapes and running no query),
then `db/` (the connection, and every read and write). `core::db` is the only
module that talks to SQLite, and no row ever leaves it: callers get plain values
back. Keep it that way: a new query belongs in `core::db`, never in a handler.

Every table is **hybrid**: a real column only for what a query filters, sorts or
joins on or what the database must constrain, and one JSON `data` column for
everything else. `core/models/hybrid_document.rs` is the trait every model
implements — `to_dict()` merges columns, child rows and blob into one flat dict,
`from_dict()` splits one back, `update_from_dict()` writes a changed dict onto a
loaded row, `to_json()` is the dict as text. A dict is a `RowDict`, an `IndexMap`
of `Field` — JSON plus the `Instant` and `Date` that JSON has no shape for — and
the `row!` macro writes one literally. A game's bag is rows of `items`, one per
line, carried in the game's dict as `items`: a line with an `id` updates that
row, a line without is a new row, a line left out is deleted. `core::db` reads a
row as a dict, changes the dict, and writes it back whole through `db/store.rs`;
it never sets a column by hand. Adding a field to a fight is adding a key to the
blob. The hero's `force`, `vie_max`, `vie_actuelle` and `gold` are `SMALLINT`
columns with a `CHECK` holding each at zero or above, so `inventory.rs` must
never produce a negative one: the database refuses it. Ids are integers the
database counts up and never reuses.

**Every schema change comes with a migration.** A file's `PRAGMA user_version`
says which schema wrote it; `connect_db()` gives a new file the current tables
and `SCHEMA_VERSION`, and refuses a file carrying any other version. Changing a
table, a constraint or an index means adding the next numbered SQL script to
`migrations/` — one transaction, guarded against the wrong starting version,
checking every row came across before it drops anything — and raising
`SCHEMA_VERSION` in `core/db/schema.rs` to its number. The statements in
`schema.rs` are spelled out rather than generated because
`tests/test_migrations.rs` compares the SQL in `sqlite_master` between a migrated
file and a fresh one, character for character.

`core/logs/` is the log — `note()` for a step at DEBUG, `event()` for something
that happened at INFO, `failure()` for what went wrong at ERROR — and
`application/logs/request_trace.rs` is the layer that wraps the router. It sits
in `core` because `core::db` writes to it. **Name every variable and write out
its content** — the `vars!` macro is how — and never write a secret: a field
whose name says token, secret, password, authorization, cookie, session or key is
hidden by `general_log`, at the top level and inside any body. `game_id` is
written to its first eight characters: an integer id comes out whole, and an old
uuid left in a cookie is cut. Use `note` / `event` / `failure`, never `println!`.

Character creation has two modes, `character::NORMAL` (the book) and
`character::EASY` (a house rule). A mode is a `Throw` for Force and one for Vie —
a base plus `count`D`faces` — so a new one is an entry in `MODES`, not a branch,
and the sheet prints whatever formula made the hero.

`core/inventory.rs` says what a gain or a loss does; it is pure like the combat
engine, and `core::db` loads, calls and writes. **Never apply a change carrying a
`condition`.** The conditions are free French — a prerequisite or a duration —
and 65 of the book's 275 changes have one; they go to the game's `pending` list
and the reader rules on them. Vie may never rise above `vie_max`, and nothing
goes below zero.

A hero is laid to rest by `lay_to_rest()` in `core::db`, called after every write
that can empty his Vie — combat, a choice followed, a pending change applied.
Add a call there if you add another such write, and never move a stamped
`died_at`: the first death is the one that counts. A hero abandoned alive is not
among the fallen.

The game rules stay pure and stay out of `core::db`: `combat.rs` is handed a
fight and gives back the fight after one assault, and `core::db` is what loads
it, calls it and writes the result. Every function that needs chance takes a
`&dyn Rng`, threaded from `create_app`, so a rule can be tested against dice
chosen for it. Nothing reaches for a generator of its own.

`combat::FIGHTS_WITH_SPECIAL_RULES` lists the thirteen fights whose page text
adds a rule the engine does not model; `TODO.md` says what each needs. Do not
quietly widen the engine for one of them without updating both.

**Only the reading history, the play-throughs and the flagged pages are stored.**
The book is static, fits in memory, and is read off disk at startup by
`core/story.rs`; putting it in the database would buy nothing. Do not move it
there.

An instant is stored through `core/models/utc_datetime.rs` when it is a column —
aware in, aware out, sortable as text — and as ISO 8601 text when it is in the
blob. `core::db` hands a column instant back as a `DateTime` and a blob one
through as it is. `db/store.rs` gives `created_at` and `updated_at` to the tables
whose rows change: both stamped on insert, `updated_at` moved on every write that
changes the row, and no write at all for a row that is unchanged. Do not stamp
them by hand in `core::db`.

**Never swallow a database failure.** A page that needs the database and cannot
reach it must fail, not be served half-built: a reader given a page quietly
missing part of itself, after a three-second wait, is worse off than one told the
server is down. A handler returns an `AppError` and `application/errors.rs` is
the one place that turns a `DatabaseFailure` — the driver errors plus
`Unavailable`, which is nothing being configured — into a 503 page, or JSON under
the API addresses. Handlers catch nothing themselves. A page that genuinely needs
no database (the landing page) must not touch one, so that it keeps working when
there is none.

The engine is opened lazily by `db::connect_db()`, and a `Db` handle is what a
handler holds — which is the seam a test binds an in-memory database through.
`DATABASE_DIR` names the directory the SQLite files live in, and `APP_ENV`
(`dev` / `prod`) picks the file, so a dev run never touches prod data.

Runtime books live under `books/<series>/<book>/`; `pages.json` is the file the
application loads, named by `core::story::PAGES_FILE`.

The import pipeline is `src/pipeline/`, with `src/bin/parse_pages.rs` as a thin
command line over it. Its sources live in `work/`: crawled HTML under
`work/raw_data/`, and generated output in `work/parsed_data/`. The application
never calls it.

## Build, Test, and Development Commands

- `rustup` provides the toolchain; the crate needs Rust 1.85 or later and nothing
  else — SQLite is linked in.
- `cargo build` builds the library and both binaries.
- `make help` lists the repository's targets; `make test` is how the suite is run
  before pushing, and `make lint` is `cargo clippy --all-targets`, which stays
  clean.
- Copy `.env.example` to `.env`; `DATABASE_DIR` is the directory the SQLite files
  live in (default `data`, relative to the repository root, git-ignored) and
  `APP_ENV` picks the file. The file and its tables are created on first use.
  Only the reading history, the play-throughs and the flagged pages need it; the
  landing page works without one.
- `cargo run --bin serve` starts the development server on port 5001.
- `cargo run --bin parse_pages` runs the import pipeline over `work/raw_data`.

Declare dependencies in `Cargo.toml`, with an exact major version and the
features actually used; `Cargo.lock` is versioned.

## Coding Style & Naming Conventions

- Use four-space indentation and Rust's own conventions: `snake_case` modules and
  functions, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants.
  `cargo clippy --all-targets` stays clean. No formatter is enforced: the code is
  hand-wrapped at 100 columns, so make focused, style-consistent edits.
- Name what a value is. Prefer a named type or an enum over a bare `bool` or a
  tuple when the caller has to remember what the positions mean.
- Keep functions to a soft limit of 50 lines. Split larger ones into well-named,
  single-responsibility helpers, and prefer composition of small functions over
  nested logic.
- Keep handlers small: one `core::db` call and a render. Split modules by topic
  when they take on too many responsibilities, and keep `mod.rs` files thin.
- Define one public type per snake_case-named file. Plain-container structs and
  small, tightly coupled supporting types such as result objects, enums or
  configuration structs may share the main type's file.
- Put each model in its own file under `core/models/`, with its SeaORM entity in
  that file's `entity` module. Import a model from its defining module.
- Use absolute paths from the crate root (`use haute_tension::core::story::load_story`
  in a test, `use crate::core::story::load_story` inside the crate); no `super::`
  chains beyond a file's own test module.
- Use `std::path::Path` / `PathBuf` for file paths.
- Give every command-line parameter a unique one-letter short option alongside
  its long form.
- A failure that a caller can provoke is a `Result`, not a panic: no `unwrap()`
  or `expect()` on it. An `expect()` is for what cannot happen, and its message
  says why it cannot.
- Do not wrap an entry point in a catch-all: let a panic surface with its
  backtrace.
- Even when prompts or discussions are in French, keep code, identifiers, and
  code comments in English. Preserve UTF-8 French user-facing text.
- Favor readability, explicit behavior, and small testable units over cleverness.

## Doc comments

Add concise Google-style doc comments to every public item, and to a private one
whose reason is not on its face. Include `# Arguments`, `# Returns`, `# Errors`
and `# Panics` sections when they say something the signature does not, but do
not restate the signature. Add more detail only when it provides useful context.

## Comments

Reserve comments for genuinely complex, non-obvious, or easily misread logic.
Keep them brief and focused on intent or reasoning; do not narrate
straightforward code or caption the obvious.

## Testing Guidelines

Run the suite with `cargo test`. All tests must live in the repository-root
`tests/` directory, one binary per file; do not add a `tests/` directory of your
own elsewhere. Name test files `test_*.rs`, and drive the routes through the
harness's test client rather than over a socket. Coverage is measured by
`cargo llvm-cov` (`make coverage`), gated on lines at `MIN`, default 95; keep new
code covered.

Every test runs on an in-memory SQLite of its own — the same engine as the file
the application opens, so there is one backend and nothing to bring up.
`tests/common/mod.rs` opens it, creates the tables through the real
`create_the_schema` and hands the routes a `Db::Bound`; take `fake_db()`,
`books_path()` or `client()` rather than standing up your own. A test that reads
the environment or the log capture takes `env_lock()` or `log_lock()`: both belong
to the process, and cargo runs a file's tests in parallel threads. Run
`make test` before pushing.

Seed a table with `db.table("<name>").seed(vec![...])`, flat dicts in the
`to_dict()` shape — columns and blob keys mixed, a game's bag as its `items`
list, an `id` to update that row and none to insert a new one; `docs()` gives the
same shape back, and the `row!` macro writes one. Document manual checks for
`/data/<number>` in the pull request.

## Commit & Pull Request Guidelines

History uses short sentence-style subjects such as `Page parsing reworked.` Use a
specific imperative subject and avoid `wip` commits in review-ready work. Pull
requests should describe user-visible behavior, list commands or manual checks
performed, identify generated-data changes, link related issues, and include
screenshots for template/UI updates.

## Security & Generated Files

Never commit real credentials or `.env`. Do not commit `target/`, caches, logs,
or OS/editor artifacts.
