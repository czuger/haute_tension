# Migrating Haute Tension from Flask to Axum

The Rust port lives in `rust/`, beside the Python application rather than on top
of it: the Python suite is the specification this was checked against, and
deleting it would have thrown the specification away. Both halves read the same
two things on disk — `haute_tension/books/<series>/<book>/pages.json` and
`migrations/*.sql` — so the book and the schema have exactly one copy.

```
cd rust
cargo test                  # the whole suite
cargo clippy --all-targets  # clean
cargo llvm-cov              # the suite under coverage; `make rust-coverage` from the root
cargo run --bin serve       # the development server, on 127.0.0.1:5001
cargo run --bin parse_pages -- -i ../work/raw_data -o ../work/parsed_data
```

## Where things went

| Python | Rust |
| --- | --- |
| `haute_tension/core/{dice,character,combat,inventory}.py` | `rust/src/core/` — unchanged in substance, pure, no I/O |
| `haute_tension/core/story.py` | `rust/src/core/story.rs` |
| `haute_tension/core/config.py` | `rust/src/core/config.rs` |
| `haute_tension/core/models/*.py` | `rust/src/core/models/*.rs` — the hybrid document, plus a SeaORM entity per table in each file's `entity` module |
| `haute_tension/core/db.py` (1190 lines) | `rust/src/core/db/{mod,connection,store,schema}.rs` |
| `haute_tension/core/logs/` | `rust/src/core/logs/` |
| `haute_tension/application/{web,routes,game,inspection}_routes.py` | `rust/src/application/routes/{web,api,game,inspection}.rs` |
| `haute_tension/application/factory.py` | `rust/src/application/factory.rs` |
| `haute_tension/application/errors.py` | `rust/src/application/errors.rs` |
| `haute_tension/application/logs/request_trace.py` | `rust/src/application/logs/request_trace.rs` |
| `haute_tension/templates/*.html` (Jinja) | `rust/templates/*.html` (Askama) |
| `haute_tension/app.py` | `rust/src/bin/serve.rs` |
| `work/parse_pages.py` (BeautifulSoup) | `rust/src/pipeline/parse_pages.rs` (scraper) + `rust/src/bin/parse_pages.rs` |
| `tests/conftest.py` | `rust/tests/common/mod.rs` |

The layering rule holds in Rust the way it held in Python: nothing under
`application/` knows what a request is, `core/db/` is the only place that talks
to SQLite, and the imports go one way.

## The schema is the same schema

`PRAGMA user_version` is still 1 and `migrations/001_integer_ids_items_and_stats.sql`
is untouched. `rust/src/core/db/schema.rs` spells the `CREATE TABLE` and
`CREATE INDEX` statements out as constants rather than generating them, because
`tests/test_migrations.rs` compares the SQL in `sqlite_master` between a migrated
file and a freshly created one — a builder emitting its own spacing would fail
that comparison without anything being wrong. The test passes, so a file either
half writes is a file the other half reads.

## Judgement calls

Each of these was a place where the Python had no Rust idiom to map onto.

**The `connect_db()` seam became a `Db` handle.** The Python tests replaced
`core.db.connect_db` with a stub and bound a module global to an in-memory
engine. Rust has no monkeypatching, so `Db` is an enum the routes hold:
`Environment` (the file `APP_ENV` picks, opened once into a process-global, as
before), `Bound` (a connection handed over ready-made — what `fake_db` uses),
`Unavailable` and `Failing` (the two shapes of "the database cannot answer",
which were the `no_database` and `dead_database` fixtures). Every `db::*`
function calls `db.conn().await?` first, exactly where Python called
`connect_db()`.

**The hybrid document needed a value type.** `to_dict()` had to keep giving a
`datetime` back for a *column* while giving ISO 8601 *text* back for the same
instant inside the blob — `tests/test_core_models.py` asserts both. So
`Field` is JSON plus `Instant` and `Date`, and `RowDict` is an `IndexMap` of
them; `serde_json` is built with `preserve_order` so a blob's keys survive a
round trip in the order they were written. `json_default` is `Field -> Result`.

**Timestamps are stamped in the writer.** SQLAlchemy's `onupdate=utc_now` and
"an update that changes nothing is not written" live in `store.rs`: a row whose
columns and blob are unchanged is not written at all, a row that is written
moves `updated_at` to now unless the dict names one itself, and an inserted row
gets `created_at` and `updated_at` at the same instant. `updated_at` counts as a
column for the change comparison, which is what lets a test set one explicitly.

**Chance is a trait.** `random.Random` was duck-typed, so the tests passed a
`FixedDice`. `core::dice::Rng` is that seam: `SeededRng` for a reproducible run,
`FixedDice` in the harness for faces chosen for one rule. Dice are consumed in
the same order, so every fixed-dice test transfers unchanged. **The generator
itself is different** (`StdRng`, not Mersenne Twister), so a test that depended
on a particular seed's *sequence* would not transfer — one did, see below.

**Sessions are a signed cookie of my own.** Flask signs with itsdangerous;
`application/session.rs` signs a base64url JSON payload with HMAC-SHA256 and the
same `FLASK_SECRET_KEY`. Cookie name, `HttpOnly`, `Path=/` and `SameSite=Lax`
are the standard Axum-ecosystem shape. It carries the same two things: the game
id and the flash messages. Cookies from the Flask app are **not** readable by
the Rust app and vice versa — a reader mid-session would be handed a fresh one,
which the code already handles (a cookie naming no game is a reader with no
hero).

**`url_for` became literal paths.** Askama has no reverse routing. The endpoint
names survive in `routes::endpoint_of`, which is what the log line
`endpoint='web.read_page'` is read off.

**Escaping had to be Jinja's.** Askama writes `&#x27;` for an apostrophe and
`&quot;` for a quote; Jinja writes `&#39;` and `&#34;`. The book is French and
full of apostrophes, so `src/escape.rs` is a Jinja-compatible escaper wired in
through `askama.toml`.

**Route arguments are parsed, not extracted.** Flask's `<int:number>` made
`/book/abc` match no route at all, which is a 404. Axum's `Path<i64>` would
answer 400, so the handlers take `Path<String>` and call
`routes::whole_number`, which answers 404 for anything that is not digits.

**The error handler became a response extension.** Flask's handler ran inside
the request context and so knew the method, the path and the endpoint. In Axum
the middleware is what knows those, so a refusal carries a `DatabaseTrouble` on
the response and `request_trace` writes the
`The database could not answer, refusing the request` line. The answer itself is
identical: 503, JSON under `/data/<n>` or for a caller that puts JSON first in
`Accept`, the page otherwise.

**The pipeline reads the document instead of rewriting it.** `work/parse_pages.py`
marked links and split `<br>` by editing the parsed tree in place;
`scraper`/`ego-tree` is a read-only DOM. So the tree is walked once and the
markers are written into the text as it is collected — the same text, and the
document left alone.

**`src/pipeline/` is in the library, not a bin-only module.** `work/` was
outside the Python package entirely. A Rust binary cannot be tested from an
integration test, and the pipeline had no tests at all, so it sits in the
library with a thin binary over it and now has 24.

## The output is byte-identical

Every one of the eleven templates was rendered from both halves against the
packaged book and the same SQLite file, and diffed:

| page | address | result |
| --- | --- | --- |
| `index.html` | `/` | byte-identical |
| `page.html` | `/book/1`, `/book/22` | byte-identical |
| `character.html` | `/game` | byte-identical |
| `combat.html` | `/combat/22` | byte-identical |
| `new_game.html` | `/game/new` | byte-identical |
| `death.html` | `/game/death` | byte-identical |
| `heroes.html` | `/heroes` | byte-identical |
| `flagged_pages.html` | `/flagged-pages` | byte-identical |
| `flagged_page.html` | `/flagged-pages/1` | byte-identical |
| `base.html` | (every page above) | byte-identical |
| `database_error.html` | `/book/1` with no database | covered by `test_errors.rs` |

Getting there cost one deliberate oddity: `base.html` keeps a
`{% let messages = layout.flashes %}` and a line holding only whitespace where
Jinja's `{% with %}` / `{% endwith %}` pair used to be, because each of those
tags left a line break in the output.

## Test parity

662 Python tests collected, 654 Rust tests. The two numbers differ for reasons
that are all accounted for below; **one** Python test has no faithful Rust
equivalent.

`@pytest.mark.parametrize` collapses: a parametrised Python test becomes one
Rust test looping over the same cases, with the case in the assertion message.
That is 662 collected → **624 distinct Python test functions**, which is the
number the Rust suite is measured against. The arithmetic to 654:

```
624  Python test functions
 -1  no faithful Rust equivalent (the self-referential structure, below)
+24  the import pipeline, which had no Python tests at all
 +2  resurrected from the shadowed `TestTheSheet` (see below)
 +1  a fresh file gets the whole schema (`test_core_connection.rs`)
 +1  the landing page carries no breadcrumb (`test_web_routes.rs`)
 +1  a row is a dict of fields (`test_core_logs.rs`)
 +1  a client can be built on a book of its own (`test_inspection.rs`)
 +1  a migrated file lives in a directory of its own (`test_migrations.rs`)
---
654  Rust tests
```

| Python file | functions | Rust file | tests | difference |
| --- | --- | --- | --- | --- |
| `test_app.py` | 9 | `test_app.rs` + `src/bin/serve.rs` | 8 + 1 | — |
| `test_core_character.py` | 29 | `test_core_character.rs` | 29 | — |
| `test_core_combat.py` | 37 | `test_core_combat.rs` | 37 | — |
| `test_core_config.py` | 17 | `test_core_config.rs` | 17 | — |
| `test_core_connection.py` | 22 | `test_core_connection.rs` | 23 | +1 added |
| `test_core_db.py` | 12 | `test_core_db.rs` | 12 | — |
| `test_core_dice.py` | 11 | `test_core_dice.rs` | 11 | — |
| `test_core_games.py` | 57 | `test_core_games.rs` | 57 | — |
| `test_core_inventory.py` | 44 | `test_core_inventory.rs` | 44 | — |
| `test_core_logs.py` | 57 | `test_core_logs.rs` | 57 | −1 dropped, +1 added |
| `test_core_models.py` | 60 | `test_core_models.rs` | 60 | — |
| `test_errors.py` | 13 | `test_errors.rs` | 13 | — |
| `test_fallen_heroes.py` | 21 | `test_fallen_heroes.rs` + `test_memorial_page.rs` | 15 + 6 | — |
| `test_game_routes.py` | 43 | `test_game_routes.rs` | 43 | — |
| `test_inspection.py` | 40 | `test_inspection.rs` | 41 | +1 added |
| `test_inventory_routes.py` | 44 | `test_inventory_routes.rs` | 46 | +2 resurrected |
| `test_menu.py` | 29 | `test_menu.rs` | 29 | — |
| `test_migrations.py` | 25 | `test_migrations.rs` | 26 | +1 added |
| `test_request_trace.py` | 23 | `test_request_trace.rs` | 23 | — |
| `test_routes.py` | 6 | `test_routes.rs` | 6 | — |
| `test_story.py` | 8 | `test_story.rs` | 8 | — |
| `test_web_routes.py` | 17 | `test_web_routes.rs` | 18 | +1 added |
| — | 0 | `test_parse_pages.rs` | 24 | new |

### The one test with no faithful Rust equivalent

- **`test_core_logs.py::TestReadable::test_a_structure_that_refers_to_itself_falls_back_too`.**
  It builds a list that contains itself and checks that `readable()` falls back
  to `repr` rather than recursing for ever. `Field` is an owned, acyclic tree —
  a `Field::List` cannot contain itself — so the state the test describes cannot
  be constructed, and a test asserting it would be asserting nothing. The
  fallback path it exercises **is** covered, by
  `something_nothing_can_write_falls_back_to_its_own_description`.

### Tests ported with a substitution

- **`test_core_logs.py::TestReadable::test_something_nothing_can_write_falls_back_to_repr`**
  used a dict with a non-string key, which JSON refuses and Python's `repr` does
  not. Rust dict keys are `String`, so the Rust test reaches the same fallback
  with a float that is not a number (`f64::NAN`), which JSON also refuses.
- **`test_core_logs.py::TestRotatingLog::test_nothing_is_written_to_stderr_when_it_disappears`**
  captured stderr, because Python's logging prints a traceback per lost line.
  `RotatingLog` has no such fallback by construction; the Rust test
  (`nothing_is_reported_when_it_disappears`) asserts the observable half — the
  directory is remade and the later lines land.
- **`test_core_logs.py::TestSecrets::test_a_tuple_comes_back_a_list`** asserted
  that a tuple walked by `sanitised()` comes back a list. `Field` has no tuple,
  so the Rust test (`a_list_comes_back_a_list`) asserts the same invariant for
  the one sequence it does have.
- **`test_core_models.py::TestJsonDefault::test_anything_else_is_refused`** passed
  a `set`. The Rust `json_default` takes a `Field`, so the test passes a
  `Field::List` — a value JSON writes itself and the hook therefore refuses,
  naming its type, which is the contract.
- **`test_core_models.py::TestStatColumns::test_each_is_a_small_integer`** read
  SQLAlchemy's column type object. The Rust test asserts the same property
  against `schema.rs`, which is where the column type now lives.
- **`test_app.py::TestCreateApp::test_every_blueprint_is_registered`** counted
  Flask blueprints. Axum has none, so the Rust test asks one address of each of
  the four families and checks it routes.
- **`test_request_trace.py::TestFailures::test_a_raising_route_is_logged_with_its_traceback`**
  monkeypatched `render_template` to raise and caught the exception out of the
  test client. A Rust handler returns `Result` rather than unwinding, so the
  test wires the same trace layer around a one-route router whose handler fails
  to render, and asserts the `Request failed` line.
- **`test_request_trace.py::TestBodiesNotRead`** built a streamed Flask response
  and a `send_file` response. The Rust equivalent is a body whose
  `size_hint().exact()` is `None`, which is what the trace describes instead of
  reading.
- **`test_inventory_routes.py::TestTheSheet::test_the_vie_gauge_matches_the_vie`**
  set `vie_actuelle = vie_max // 2` and asserted `width: 50%`, which only holds
  when the rolled `vie_max` happens to be even. The Rust generator is not
  Mersenne Twister, so the seed that made it even no longer does. The Rust test
  sets `vie_max = 24, vie_actuelle = 12` and asserts the same 50%.

### Tests the Python suite was not running

`tests/test_inventory_routes.py` declares `class TestTheSheet` **twice**. The
second declaration shadows the first, so three tests never ran:

- `test_the_purse_shows_its_throws` — ported, passes.
- `test_the_bag_is_listed` — ported, passes.
- `test_an_emptied_bag_says_so` — **not ported.** It asserts that an emptied bag
  says `"Votre sac est vide"`. `character.html` says
  `"Vous n'avez plus rien. Ni provisions, ni arme, ni bourse."` and never has
  said otherwise, so this test would fail against either half. The behaviour it
  meant to check is covered by
  `the_sheet::an_empty_bag_says_so`, which asserts the text the template
  actually carries.

## Test pass/fail report

654 passed, 0 failed, run three times over with no flakes. `cargo clippy
--all-targets` is clean.

| Rust test binary | Python origin | result |
| --- | --- | --- |
| `src/bin/serve.rs` (1) | `test_app.py::TestMain` | 1 passed |
| `test_app.rs` (8) | `test_app.py::TestCreateApp` | 8 passed |
| `test_core_character.rs` (29) | `test_core_character.py` | 29 passed |
| `test_core_combat.rs` (37) | `test_core_combat.py` | 37 passed |
| `test_core_config.rs` (17) | `test_core_config.py` | 17 passed |
| `test_core_connection.rs` (23) | `test_core_connection.py` | 23 passed |
| `test_core_db.rs` (12) | `test_core_db.py` | 12 passed |
| `test_core_dice.rs` (11) | `test_core_dice.py` | 11 passed |
| `test_core_games.rs` (57) | `test_core_games.py` | 57 passed |
| `test_core_inventory.rs` (44) | `test_core_inventory.py` | 44 passed |
| `test_core_logs.rs` (57) | `test_core_logs.py` | 57 passed |
| `test_core_models.rs` (60) | `test_core_models.py` | 60 passed |
| `test_errors.rs` (13) | `test_errors.py` | 13 passed |
| `test_fallen_heroes.rs` (15) | `test_fallen_heroes.py` (database half) | 15 passed |
| `test_game_routes.rs` (43) | `test_game_routes.py` | 43 passed |
| `test_inspection.rs` (41) | `test_inspection.py` | 41 passed |
| `test_inventory_routes.rs` (46) | `test_inventory_routes.py` | 46 passed |
| `test_memorial_page.rs` (6) | `test_fallen_heroes.py::TestTheMemorialPage` | 6 passed |
| `test_menu.rs` (29) | `test_menu.py` | 29 passed |
| `test_migrations.rs` (26) | `test_migrations.py` | 26 passed |
| `test_parse_pages.rs` (24) | none — new | 24 passed |
| `test_request_trace.rs` (23) | `test_request_trace.py` | 23 passed |
| `test_routes.rs` (6) | `test_routes.py` | 6 passed |
| `test_story.rs` (8) | `test_story.py` | 8 passed |
| `test_web_routes.rs` (18) | `test_web_routes.py` | 18 passed |

## How the harness matches the Python one

`tests/common/mod.rs` is `tests/conftest.py`:

- `fake_db()` opens an in-memory SQLite of its own per test, creates the tables
  through the real `create_the_schema`, and hands the routes a `Db::Bound` to it.
- `db.table("games").docs()` / `.seed(...)` is `fake_db["games"].docs`, with the
  same contract — flat dicts, columns and blob keys mixed, a game's bag riding
  along as its `items` list, an `id` updating that row and none inserting a new
  one, a row left out deleted.
- `books_path()` / `books_path_with_fights()` write the same two-page book.
- `client()` / `fighting_client()` / `hero()` are the same three clients, driven
  through `tower::ServiceExt::oneshot`, keeping cookies between requests the way
  Flask's test client does.
- `FixedDice` is `tests/test_core_dice.py::FixedDice`.
- `env_lock()` and `log_lock()` are new: the environment and the log capture
  belong to the process, and Rust runs a file's tests in parallel threads.

## Coverage

`cargo-llvm-cov` is `pytest-cov`, and `make rust-coverage` is `make coverage`: the
suite runs once under `-C instrument-coverage`, the HTML report is written to
`rust/target/llvm-cov/html/` (the build directory, git-ignored, as `htmlcov/` is)
and the terminal gets the per-file table with the uncovered lines named. `MIN` is
the gate — `--fail-under-lines`, which is `--cov-fail-under` — and it defaults to
95, the number `pyproject.toml` holds the Python half to. The tool is installed
once:

```bash
brew install cargo-llvm-cov            # or: cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
```

The gate is on **lines** rather than branches: `--branch` warns that it needs a
nightly toolchain and measures nothing on stable, so the table prints regions —
the finer count, and the closer thing to what `branch = true` gave coverage.py —
beside the lines, and the gate reads the lines.

**The suite does not clear 95% yet.** All 654 tests pass under instrumentation
and the library sits at **89.98% of lines, 88.45% of regions**, so
`make rust-coverage` fails on the gate until the gaps are closed or `MIN` is
lowered. That number is not the Python 100% being lost — the two are not counting
the same thing:

- **The scope is wider.** `--cov=haute_tension` never saw `work/`, which had no
  tests at all. Here the pipeline is in the library: `pipeline/parse_pages.rs`
  (66.87%), `bin/parse_pages.rs` (0%) and `bin/serve.rs` (25%) are two fifths of
  the whole shortfall between them. Without those three the library is at
  **93.38% of lines, 91.88% of regions**.
- **llvm-cov counts code coverage.py never had.** An arm of an exhaustive `match`
  no test reaches (`Field::type_name` in `hybrid_document.rs`), an accessor or a
  SeaORM `RelationTrait::def` nothing calls (`models/item.rs`,
  `models/page_view.rs`) — Python had no such code to leave uncovered.
- **A `?` is a region.** The error paths that Python raised through are explicit
  returns here, and the ones no test provokes are counted.

## Not carried over

- **`|truncate(160)`** on the flagged-pages list. Jinja's `truncate` has a
  five-character leeway and Askama's does not, so a remark 161 to 165 characters
  long is cut here and was not there. No test covers that range.
