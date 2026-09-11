# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

@AGENTS.md

`AGENTS.md` (imported above) holds the coding conventions and the rules that must not be broken; this file adds the commands and the big picture. `README.md` is the long-form reference — routes, the game rules, the log, the book data, the import pipeline — and is kept accurate: update it when behaviour changes.

## Commands

```bash
python -m pip install -e ".[test]"      # install, with pytest / pytest-cov

make test                               # whole suite, every test on an in-memory SQLite of its own (= python -m pytest)
make test ARGS="-k history -v"          # ARGS passes through to pytest
make coverage                           # suite + HTML report in htmlcov/

python -m pytest tests/test_core_combat.py --no-cov                         # one file
python -m pytest tests/test_core_combat.py::TestAssault::test_name --no-cov # one test

make serve                              # dev server on http://127.0.0.1:5001 (= cd haute_tension && PYTHONPATH=.. python app.py)
```

`pyproject.toml` adds `--cov --cov-fail-under=95` to every pytest run, so a subset of the suite "fails" on coverage even when every test passes — pass `--no-cov` when running less than the whole suite. The gate is at 95%; the suite currently sits at 100% and new code is expected to keep it there.

Nothing has to be brought up for the tests: SQLite in memory is the same engine as the file the application opens, so there is one backend and one suite. The application's own files live under `DATABASE_DIR` (`.env`, default `data/`, git-ignored), one per `APP_ENV`.

No linter or formatter is configured.

## Architecture

Two halves, unconnected at runtime: the Flask reader in `haute_tension/` and the offline import pipeline in `work/` that produced `haute_tension/books/<series>/<book>/pages.json`. The app never touches `work/`.

Inside `haute_tension/` the layering is strict and imports go one way only:

```
application/            Flask: factory, blueprints (web, api, game), error pages, request trace
        │  calls
core/db.py              THE ONLY module that talks to SQLite (SQLAlchemy sessions) — every read and
        │               write, and the place that loads game state, calls a rule, and writes it back
        ├── core/models/         SQLAlchemy models: shape only, no queries; a row never leaves db.py
        ├── core/combat.py, inventory.py, character.py, dice.py   pure rules, no I/O
        ├── core/story.py        pages.json → dict in memory at startup
        ├── core/logs/           note()/event()/failure() — db.py writes here, so it lives in core
        └── core/config.py       .env, APP_ENV → database name, session secret
```

Things that only make sense once you have read several files:

- **Routes hold no logic and catch nothing.** A route calls one `db.*` function and renders. `db.py` returns plain dicts (`application/models/game.py` TypedDicts describe them); no row object reaches a template. Database failures propagate to the single handler in `application/errors.py`, which renders a 503 page (JSON under the `api` blueprint).
- **Every table is hybrid.** A field is a real column only when a query filters, sorts or joins on it; everything else is in one JSON `data` column. `core/models/hybrid_document.py` is the mixin (`to_dict` / `from_dict` / `update_from_dict` / `to_json`), and `db.py` reads a row as one flat dict, changes the dict, and writes it back whole. A new field on a hero is a key in the blob, not a migration; a new query on a blob key means promoting it to a column.
- **Chance is injected.** `create_app(rng=random.Random(...))` threads one `Random` through `game_routes` → `db.py` → `character` / `combat`. Tests pin dice through the `fighting_client` fixture; nothing calls `random` directly.
- **The play-through is one row.** `Game` carries hero stats, bag, open combat, pending changes and death; `db.py` changes and writes it whole. Anything that can drop Vie to zero must end by calling `db._lay_to_rest()`.
- **Gains and losses with a `condition` are never applied automatically** — they land in `game.pending` for the reader to rule on (`/game/pending/...` routes). 65 of the book's 275 changes carry one.
- **The book stays on disk.** SQLite holds only `page_views` (reading history), `games` and `page_inspections`. `DATABASE_DIR` names the directory and `APP_ENV` picks the file in it (`haute_tension_dev.sqlite3` / `_prod`); tables are created on first use, there is no migration step. A `DATABASE_DIR` that names a file is refused.
- **The log is part of the contract.** Every request leaves `Request` / `Answer` lines in `logs/general.log` via `application/logs/request_trace.py`; JSON answers are written in full, secrets are redacted by field name, `game_id` is cut to eight characters. Use `note`/`event`/`failure` from `core.logs.general_log`, never `logging` or `print`.

## Tests

`tests/conftest.py` is the whole test infrastructure: `fake_db` (an in-memory SQLite per test, bound through the `connect_db()` seam), `books_path` (a tiny two-page book on disk; `books_path_with_fights` adds fights and conditional changes), `client` / `fighting_client` / `hero` (Flask test clients on top, the last with a hero already rolled). Seed data by assigning to `fake_db["games"].docs` — flat dicts in the `to_dict()` shape, columns and blob keys mixed; reading `.docs` gives the same shape back. Tests run against the real `logs/general.log` too, so lines from the suite in there are normal.

## Language

- **Code is English, whatever language the prompt is in.** Identifiers, function names, comments, docstrings, commit messages and log lines are written in English even when the request or the discussion is in French.
- **User-facing text is French, and stays French.** The book, the templates, the JSON error messages (`"le numéro N n'a pas été trouvé"`), item labels (`label_fr`), the `condition` texts — anything a reader sees is French, UTF-8, accents kept. Do not translate it, and write new user-facing text in French.

## Style

The conventions are in `AGENTS.md` (imported above): absolute imports, every function fully typed with Google-style docstrings, comments only for the non-obvious, 50-line functions, one public class per snake_case file, thin `__init__.py` that re-export nothing. Two rules it states less directly:

- **No `try`/`except` unless the failure is expected and recoverable**, or the call is known to raise and the catch does something with it. Never to swallow, never as control flow; unexpected exceptions propagate with their traceback.
- **Prefer explicit over implicit** in naming, typing and structure, and split a module as soon as it starts doing too much rather than waiting for a refactor pass.
