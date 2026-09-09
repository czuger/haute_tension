# Haute Tension

[![Build Status](https://travis-ci.org/czuger/haute_tension.svg?branch=master)](https://travis-ci.org/czuger/haute_tension)
[![Code Climate](https://codeclimate.com/github/czuger/haute_tension/badges/gpa.svg)](https://codeclimate.com/github/czuger/haute_tension)
[![Test Coverage](https://codeclimate.com/github/czuger/haute_tension/badges/coverage.svg)](https://codeclimate.com/github/czuger/haute_tension/coverage)

Une mise en ligne du livre dont vous êtes le héros : *La Forteresse d'Alamuth*
(saga du Prêtre Jean).

The project has two halves:

1. **A Flask reader** (`haute_tension/`) that serves the book as a browsable
   website, plus a small JSON API. The book is read off disk into memory at
   startup; MongoDB holds the reading history, and nothing else.
2. **An offline import pipeline** (`work/`) that scrapes the original pages from
   the web, parses them into JSON, and enriches them (per-choice gains/losses,
   structured combat encounters) into the runtime book data.

---

## Repository layout

```
haute_tension/                 Flask application package
  app.py                       Development-server entry point (port 5001)
  application/                 The web layer — knows nothing about Mongo
    factory.py                 create_app(): wires blueprints, book title
    web_routes.py              Browser routes: "/" and "/book/<number>"
    routes.py                  API route: "/data/<number>"
    models/story_page.py       TypedDicts: StoryPage, StoryChoice, ElementChange
  core/                        Below the web layer — knows nothing about Flask
    config.py                  .env, APP_ENV, and which database is used
    story.py                   load_story(): reads and indexes pages.json
    db.py                      The connection, and every read and write
    models/page_view.py        PageView: one row per page asked for
  templates/                   Jinja templates (base / index / page), CSS inline
  books/<series>/<book>/       Runtime book data (see "Book data" below)

Makefile                       make test / test-fast / coverage / serve

work/                          Offline import & conversion pipeline
  download_raw_data.rb         Ruby crawler: fetches source HTML into raw_data/
  urls.txt                     Books to crawl (one per line, "#" disables)
  parse_pages.py               HTML -> JSON: page text + outgoing page numbers
  conversions_prompt_actions.txt   LLM prompt: extract choice gains/losses
  conversion_prompt_combat.txt     LLM prompt: detect and structure fights
  new_fable_prompt_for_processing_book.txt  Combined single-pass version
  raw_data/                    Downloaded HTML + per-book YAML index
  parsed_data/                 Generated intermediate JSON/YAML

tests/                         Test suite (pytest, mongomock, Flask test client)
  conftest.py                  The fake database every test runs against
.env.example                   APP_ENV and MONGO_URI — copy to .env
AGENTS.md                      Coding conventions for this repository
pyproject.toml                 Package metadata, dependencies, pytest/coverage
```

---

## The Flask application

### Entry point and factory

`haute_tension/app.py` builds the app with `create_app()` and runs the
development server on `127.0.0.1:5001` with debug enabled. It binds loopback
only, so the debugger is never exposed to the local network; change `host` in
`main()` if you want to read the book from another device on your wifi.

`application/factory.py` holds all the wiring:

| Constant        | Value                                                   |
| --------------- | ------------------------------------------------------- |
| `BOOK`          | `pretre_jean/forteresse_alamuth`                         |
| `BOOKS_PATH`    | `haute_tension/books`                                    |
| `TEMPLATE_PATH` | `haute_tension/templates`                                |
| `BOOK_SERIES`   | `Prêtre Jean`                                            |
| `BOOK_TITLE`    | `La Forteresse d'Alamuth`                                |

`create_app(book, books_path)` accepts both, which is how the tests run against
fixture books. It reads the book **once, at startup**, and holds it in memory.

### Routes

| Method | Path              | Blueprint | Description                                                                                   |
| ------ | ----------------- | --------- | --------------------------------------------------------------------------------------------- |
| `GET`  | `/`               | `web`     | Landing page: series, title, and a link to the opening page (page 1).                          |
| `GET`  | `/book/<number>`  | `web`     | Reader: a breadcrumb of the last ten pages read, the page's choices, then its text. Unknown page → HTTP 404. A page with no choices shows "Fin de l'aventure" and a restart link. |
| `GET`  | `/data/<number>`  | `api`     | The raw page object as JSON (text, choices, `fight` when present).                              |

`/data/<number>` also appends the requested page to the read history before
looking it up. An unknown page returns a UTF-8 JSON body
`{"success": false, "message": "le numéro N n'a pas été trouvé"}` — note that
this error still carries HTTP 200.

---

## The database

MongoDB, through mongoengine — and it holds **one thing: the reading history.**

The book does not go in it, deliberately. It is 668 static pages that change only
when the import pipeline is re-run, it fits in memory several times over, and
`core/story.py` reads it off disk at startup. Storing it would buy nothing, and
would cost the ability to serve a page without a reachable server.

`haute_tension/core/` is everything below the web layer, and nothing there
imports Flask. The layering is strict and the imports only ever go one way:

```
config      environment variables, and which database
story       the book, read off disk into memory
models      the shape of what is stored — no module here runs a query
db          the database, on top of config and models
```

**`core/db.py` is the only module that talks to Mongo**, and no document object
ever leaves it, which is what keeps the ORM out of the web layer. A new query
belongs here, never in a route.

The connection is opened on the first call rather than at import time, so
importing `core.db` never needs a reachable server — and it is the single seam
the tests replace.

### Which database

`APP_ENV` (`dev` or `prod`, default `dev`) suffixes the base name, giving
`haute_tension_dev` or `haute_tension_prod`. `MONGO_URI` says where the server
is, and must **not** name a database: one given there would silently win over
`APP_ENV`, so `connect_db()` refuses it.

### The one collection

| Collection   | Model      | Keyed by                                    |
| ------------ | ---------- | ------------------------------------------- |
| `page_views` | `PageView` | An ObjectId — a visit has no id of its own  |

One row per page asked for, scoped by a `book` field written
`"<series>/<book>"`, so a second book never shows up in the first one's history.

- `record_page_view(book, page)` — notes that a page was asked for, and says
  whether it recorded anything. Asking again for the page one is already on is a
  reload rather than a move, and is dropped; coming back to a page after going
  elsewhere is a loop in the story, and is kept.
- `last_pages(book, limit=10)` / `get_oldest_page(book)` — the bounded reading
  history, oldest first. Bounding happens **on read**, so recording a visit stays
  a plain insert.

### When the database is not there

`DatabaseError` is what a caller catches when a database call fails.
`HistoryUnavailable` is wider by one case, and it is the case that matters: a
reader who has never configured a server at all gets an `EnvironmentError` out of
`connect_db()`, not a driver error.

The reader route catches the wider one. The breadcrumb is the only thing on the
page that needs Mongo, and the book is not — so an unavailable history costs the
breadcrumb and nothing else, and the book stays readable with no `.env` at all.

---

## The Flask templates

`base.html` carries the layout and the whole stylesheet (serif body, parchment
palette, drop cap on the first paragraph, mobile breakpoint at 36rem).
`index.html` is the landing card; `page.html` renders the breadcrumb, the page
number, the choices block, then the story text.

The breadcrumb (`.trail`) lists the last ten pages read, oldest first, each a
link back except the current one. It can hold ten entries and a revisited page
appears twice, so it scrolls sideways on a narrow screen rather than wrapping.
It is absent, not empty, when there is no history to show.

---

## Book data

Runtime books live in `haute_tension/books/<series>/<book>/`:

- **`pages.json`** — the file the application loads, named by
  `core.story.PAGES_FILE`. A JSON list of 668 page objects for *La Forteresse
  d'Alamuth*, 68 of them carrying a `fight`. Read into memory at startup.

A page object looks like this:

```json
{
  "page": "22",
  "language": "fr",
  "text": ["Vous renversez d'un violent coup de pied la table…"],
  "file_path": "raw_data/pretre_jean_forteresse_alamuth/3777178a….html",
  "choices": [
    { "goto": "621", "gains": [], "losses": [] }
  ],
  "fight": {
    "fight_type": "single",
    "enemies": [
      { "name": "collecteur d'impots",
        "original_name": "COLLECTEUR D'IMPOTS",
        "force": 6, "vie": 10 }
    ],
    "outcome": { "on_victory": "621", "on_defeat": "death", "on_flee": null },
    "needs_review": false
  }
}
```

The shapes are declared as `TypedDict`s in
`application/models/story_page.py`: `StoryPage` (with optional `fight`),
`StoryChoice` (`goto`, `gains`, `losses`) and `ElementChange`
(`element`, `amount`).

---

## Getting started

### 1. Environment

The repository pins its interpreter through `.python-version`, which names a
pyenv virtualenv rather than a version:

```bash
pyenv virtualenv <python-version> haute_tension   # >= 3.10
pyenv local haute_tension
```

### 2. Install

```bash
python -m pip install -e .
```

Dependencies come from `pyproject.toml`: Flask, mongoengine/pymongo,
python-dotenv, PyYAML, beautifulsoup4/bs4. Do not add a `requirements.txt`. Add
`[test]` to also install pytest, pytest-cov and mongomock:

```bash
python -m pip install -e ".[test]"
```

### 3. Database

Copy `.env.example` to `.env` (git-ignored) and point it at a mongod:

```bash
APP_ENV=dev
MONGO_URI=mongodb://localhost:27017
```

Leave the database name out of `MONGO_URI` — `APP_ENV` is what picks it, and a
URI naming one is refused rather than silently obeyed. Any mongod will do:

```bash
docker run -d --rm --name ht-mongo -p 27017:27017 mongo:7
```

Only the reading history needs it. The book is read off disk, so browsing works
whether or not a server is up.

### 4. Run the server

```bash
cd haute_tension && PYTHONPATH=.. python app.py
```

`PYTHONPATH=..` puts the repository root on the import path so `haute_tension.*`
resolves.

Then open <http://localhost:5001/>.

### 5. Manual checks

```bash
curl http://localhost:5001/data/1
curl http://localhost:5001/data/99999      # unknown page, HTTP 200 + JSON error
```

---

## Tests

All tests live in the repository-root `tests/` directory and run under pytest,
configured in `pyproject.toml` to measure branch coverage of `haute_tension` and
to fail below 95%.

The same suite runs against two backends, and no test is written to care which:

```bash
make test        # a real MongoDB in a container — the pass that counts
make test-fast   # the in-memory server, no container, well under a second
python -m pytest # the same as make test-fast
```

`make test` brings up `mongo:7` on **port 27019**, waits for it to actually
answer, and points the suite at it. mongomock is a reimplementation, so a driver
behaviour it does not share is a bug only the real server shows — this repository
has already had one. The container stays up between runs, which makes a series of
`make test` fast; `make mongo-stop` removes it.

Two separations keep it away from the application's data, and both matter: it
listens on its own port, **and** it works in its own database,
`haute_tension_test`. The port alone would not be enough — nothing stops a
`MONGO_URI` from pointing the application at that very container, and only the
distinct database name then keeps `make test` from dropping a reading history.

| Target | Does |
| ------ | ---- |
| `make test` | Brings up MongoDB and runs the whole suite against it |
| `make test-fast` | The same suite on the in-memory server |
| `make coverage` | The suite plus an HTML report in `htmlcov/` |
| `make mongo` / `make mongo-stop` | Brings the test container up / removes it |
| `make serve` | Runs the development server on port 5001 |

`ARGS` passes arguments through to pytest (`make test ARGS="-k history -v"`).

**No test needs a running mongod** — `make test-fast` is the whole suite with
nothing brought up. `tests/conftest.py` binds the models to mongomock, or to
`MONGO_URI_TEST` when the Makefile sets it, and stubs out `core.db.connect_db()`
either way — the one place that would reach for a server of its own. Three
fixtures are the whole interface:

- `fake_db` — the empty database. Seed a collection by assigning to
  `fake_db["page_views"].docs`.
- `books_path` — a small two-page book written to a temporary directory, so no
  test reads the packaged one.
- `client` — a Flask test client serving that book, with a database behind it.

73 tests cover the connection and its guards (`test_core_connection.py`), the
reading history (`test_core_db.py`), how a run picks its database
(`test_core_config.py`), loading a book and every malformed input it rejects
(`test_story.py`), the data route and its history side effect (`test_routes.py`),
browser navigation, the breadcrumb and what a dead database costs it
(`test_web_routes.py`), and the app factory and entry point (`test_app.py`).
Current coverage: 100%.

---

## The import pipeline

The pipeline is run by hand, book by book; nothing here is wired into the app.

**1. Crawl.** `work/urls.txt` lists books as
`path|start_url|rules_url`, with `#` marking the ones already done.
From `work/`:

```bash
ruby download_raw_data.rb        # needs open-uri, nokogiri, yaml
```

It follows `div.ob-text` links breadth-first from the start URL, saves each page
as `raw_data/<book>/<sha256-of-path>.html`, and writes an index
`raw_data/<book>.yaml` mapping every URL to its file path, origin URL and hash.

**2. Parse.** `work/parse_pages.py` treats every sub-directory of
`work/raw_data/` as one book and writes `work/parsed_data/<book>.json`. It never
reads the YAML index nor the network: the page number comes from the page's
`h2`, the text from the `div.ob-text` blocks (one entry per paragraph, `<br>`
or `<pre>` line), and each link is resolved first against the `og:url` of the
local pages, then by the number in its URL, then by matching text slugs.

```bash
python work/parse_pages.py            # -n inlines "<link text> <number>", -v logs details
```

Output: `{ "<page>": { "text": [...], "numbers": [...], "file_path": "..." } }`,
plus a per-book report (failed files, pages without numbers, remapped links).

**3. Enrich.** The three `*.txt` files in `work/` are prompts written to be
handed to an LLM agent, which transforms the parsed JSON in place rather than
emitting a script:

- `conversions_prompt_actions.txt` — turn each `numbers` entry into a `choices`
  entry carrying `gains` / `losses` (English element keys, later localized
  through `translated_elements.json`).
- `conversion_prompt_combat.txt` — detect encounters from
  `NAME FORCE : X VIE : Y` stat blocks plus combat vocabulary, and emit the
  `fight` object with `fight_type` (`single` / `sequential` / `simultaneous`),
  `enemies`, and `outcome`.
- `new_fable_prompt_for_processing_book.txt` — both passes in one run.

**4. Install.** Copy the enriched result to
`haute_tension/books/<series>/<book>/pages.json` and restart the server.

---

## Conventions

`AGENTS.md` is the authority; the essentials:

- Four-space indentation, PEP 8 naming, fully typed functions including
  `-> None`; avoid `Any`.
- Google-style docstrings on every function and method; comments only for
  genuinely non-obvious logic.
- Absolute imports from the repository root
  (`from haute_tension.core.story import load_story`), `pathlib.Path` for paths, one
  public class per snake_case file, models under `models/`.
- The database layer stays in `core/` and the web layer stays out of it: a new
  query goes in `core/db.py`, never in a route, and no document object leaves
  that module.
- Code, identifiers and comments in English even when the discussion is in
  French; user-facing French text stays UTF-8.
- No global `try`/`except` around entry points — let tracebacks surface.
- Commit subjects are short imperative sentences (`Page parsing reworked.`); no
  `wip` subjects in review-ready work.

Never commit `.env`, caches or logs.

---

## License

MIT — see `LICENSE`.
