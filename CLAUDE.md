# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

`AGENTS.md` (imported above) holds the coding conventions and the rules that must not be broken; this file adds the commands and the big picture. `README.md` is the long-form reference — routes, the game rules, the log, the book data, the import pipeline — and is kept accurate: update it when behaviour changes.

## Commands

```bash
cargo build                             # the library and both binaries

make test                               # whole suite, every test on an in-memory SQLite of its own (= cargo test)
make test ARGS="history"                # ARGS passes through to cargo test
make lint                               # cargo clippy --all-targets, which stays clean
make coverage                           # suite + HTML report in target/llvm-cov/html/ (needs cargo-llvm-cov)

cargo test --test test_core_combat                      # one file
cargo test --test test_core_combat assault::the_name     # one test, by name filter

make serve                              # dev server on http://127.0.0.1:5001 (= cargo run --bin serve)
make import                             # the import pipeline over work/raw_data (= cargo run --bin parse_pages)
```

Coverage is `cargo llvm-cov`, gated on **lines** at `MIN` (default 95): branches need a nightly toolchain, so the table prints regions beside the lines and the gate reads the lines. **The suite does not clear the gate yet** — 654 tests pass and the crate sits at 89.98% of lines, two fifths of the shortfall being in `pipeline/parse_pages.rs`, `bin/parse_pages.rs` and `bin/serve.rs`, which the application never serves. Without those three it is at 93.38%.

Nothing has to be brought up for the tests: SQLite in memory is the same engine as the file the application opens, so there is one backend and one suite. The application's own files live under `DATABASE_DIR` (`.env`, default `data/`, git-ignored), one per `APP_ENV`.

`cargo fmt` is not enforced — the code is hand-wrapped at 100 columns and running it would rewrite most files. `cargo clippy --all-targets` is the check that must stay clean.

## Architecture

Two halves, unconnected at runtime: the Axum reader in `src/{application,core}` and the offline import pipeline in `src/pipeline/`, run by hand over `work/` and what produced `books/<series>/<book>/pages.json`. The application never calls the pipeline.

The layering is strict and imports go one way only:

```
application/            Axum: factory, routes, templates, session, error pages, request trace
        │  calls
core/db/                THE ONLY module that talks to SQLite (SeaORM) — every read and write,
        │               and the place that loads game state, calls a rule, and writes it back
        ├── core/models/         one type per table plus its SeaORM entity: shape only, no
        │                        queries; a row never leaves core::db
        ├── core/combat.rs, inventory.rs, character.rs, dice.rs   pure rules, no I/O
        ├── core/story.rs        pages.json → an index in memory at startup
        ├── core/logs/           note()/event()/failure() — core::db writes here, so it lives in core
        └── core/config.rs       .env, APP_ENV → database name, session secret
```

Things that only make sense once you have read several files:

- **Handlers hold no logic and catch nothing.** A handler calls one `core::db` function and renders. `core::db` returns plain values (`application/models/game.rs` describes the shapes a template is handed); no row reaches a template. Database failures leave as an `AppError` and reach the single place in `application/errors.rs`, which renders a 503 page (JSON under the API addresses).
- **Every table is hybrid.** A field is a real column only when a query filters, sorts or joins on it or the database constrains it; everything else is in one JSON `data` column. `core/models/hybrid_document.rs` is the trait (`to_dict` / `from_dict` / `update_from_dict` / `to_json`), a dict is a `RowDict` of `Field` (JSON plus `Instant` and `Date`; the `row!` macro writes one), and `core::db` reads a row as one flat dict, changes the dict, and writes it back whole through `db/store.rs`. Child rows ride along: a game's bag is rows of `items`, read and written as the game dict's `items` list. A new field on a fight is a key in the blob; a new or changed column is a migration.
- **The schema is versioned.** `PRAGMA user_version` marks a file; `connect_db()` creates a new file at `SCHEMA_VERSION` and refuses any other version with a 503. Each schema change adds the next numbered SQL script to `migrations/` and raises `SCHEMA_VERSION` in `core/db/schema.rs`; `tests/test_migrations.rs` checks a migrated file matches a fresh one, character for character in `sqlite_master` — which is why the `CREATE TABLE` statements are spelled out in `schema.rs` rather than generated. Apply a script with `sqlite3 -bail <file> < migrations/NNN_*.sql`, after backing the file up.
- **Chance is a trait.** `core::dice::Rng`: `create_app(AppConfig { rng, .. })` threads one through `routes::game` → `core::db` → `character` / `combat`. Tests pin dice with the harness's `FixedDice` through the `fighting_client` helper; nothing reaches for a generator of its own.
- **The play-through is one row and its bag.** `Game` carries the hero's stats (columns `force`, `vie_max`, `vie_actuelle`, `gold`, never below zero), open combat, pending changes and death; its bag is `Item` rows, one per line. `core::db` changes and writes them whole. Anything that can drop Vie to zero must end by calling `lay_to_rest()`.
- **Gains and losses with a `condition` are never applied automatically** — they land in `game.pending` for the reader to rule on (`/game/pending/...` routes). 65 of the book's 275 changes carry one.
- **The book stays on disk.** SQLite holds only `page_views` (reading history), `games`, `items` and `page_inspections`. `DATABASE_DIR` names the directory and `APP_ENV` picks the file in it (`haute_tension_dev.sqlite3` / `_prod`); a new file gets its tables on first use, and a file an older schema wrote must be migrated first. A `DATABASE_DIR` that names a file is refused.
- **The log is part of the contract.** Every request leaves `Request` / `Answer` lines in `logs/general.log` via `application/logs/request_trace.rs`, a layer around the whole router; JSON answers are written in full, secrets are redacted by field name, `game_id` is cut to eight characters, which leaves an integer id whole. Use `note`/`event`/`failure` from `core::logs::general_log` with the `vars!` macro, never `println!`.
- **The templates are checked at compile time.** Askama: one struct in `application/templates.rs` per file in `templates/`, so a field a template reads and nobody fills is a build error. There is no reverse routing — a template writes the address — but the endpoint names live on in `routes::endpoint_of`, which is what the log's `endpoint='web.read_page'` is read off. `src/escape.rs` is the escaper they all use, wired in through `askama.toml`: the pages carry `&#39;` and `&#34;` where Askama's own escaper writes `&#x27;` and `&quot;`.

## Tests

`tests/common/mod.rs` is the whole test infrastructure: `fake_db()` (an in-memory SQLite per test, handed to the routes as a `Db::Bound`), `books_path()` (a tiny two-page book on disk; `books_path_with_fights()` adds fights and conditional changes), `client()` / `fighting_client()` / `hero()` (test clients on top, the last with a hero already rolled), `no_database_client()` / `dead_database_client()` (the two ways the database fails), `fixed()` / `seeded()` (dice), and `env_lock()` / `log_lock()` for the tests that touch the process's environment or the log capture. Seed data with `db.table("games").seed(vec![row!{...}])` — flat dicts in the `to_dict()` shape, columns and blob keys mixed, a game's bag riding along as its `items` list; `docs()` gives the same shape back. `tests/test_migrations.rs` builds a version-0 file by hand and applies `migrations/` to it. Tests run against the real `logs/general.log` too, so lines from the suite in there are normal.

## Language

- **Code is English, whatever language the prompt is in.** Identifiers, function names, comments, doc comments, commit messages and log lines are written in English even when the request or the discussion is in French.
- **User-facing text is French, and stays French.** The book, the templates, the JSON error messages (`"le numéro N n'a pas été trouvé"`), item labels (`label_fr`), the `condition` texts — anything a reader sees is French, UTF-8, accents kept. Do not translate it, and write new user-facing text in French.

## Style

The conventions are in `AGENTS.md` (imported above): absolute paths from the crate root, doc comments on every public item, comments only for the non-obvious, 50-line functions, one public type per snake_case file, thin `mod.rs`. Two rules it states less directly:

- **A failure a caller can provoke is a `Result`, never a panic.** No `unwrap()` or `expect()` on it; an `expect()` is for what cannot happen, and its message says why it cannot. Do not wrap an entry point in a catch-all.
- **Prefer explicit over implicit** in naming, typing and structure, and split a module as soon as it starts doing too much rather than waiting for a refactor pass.
