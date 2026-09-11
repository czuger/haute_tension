# Haute Tension

[![Build Status](https://travis-ci.org/czuger/haute_tension.svg?branch=master)](https://travis-ci.org/czuger/haute_tension)
[![Code Climate](https://codeclimate.com/github/czuger/haute_tension/badges/gpa.svg)](https://codeclimate.com/github/czuger/haute_tension)
[![Test Coverage](https://codeclimate.com/github/czuger/haute_tension/badges/coverage.svg)](https://codeclimate.com/github/czuger/haute_tension/coverage)

Une mise en ligne du livre dont vous êtes le héros : *La Forteresse d'Alamuth*
(saga du Prêtre Jean).

The project has two halves:

1. **A Flask reader** (`haute_tension/`) that serves the book as a browsable
   website, plus a small JSON API. The book is read off disk into memory at
   startup; a SQLite file holds the reading history, the play-throughs and the
   flagged pages, and nothing else.
2. **An offline import pipeline** (`work/`) that scrapes the original pages from
   the web, parses them into JSON, and enriches them (per-choice gains/losses,
   structured combat encounters) into the runtime book data.

---

## Repository layout

```
haute_tension/                 Flask application package
  app.py                       Development-server entry point (port 5001)
  application/                 The web layer — knows nothing about the database
    factory.py                 create_app(): wires blueprints, book title
    web_routes.py              Browser routes: "/" and "/book/<number>"
    routes.py                  API route: "/data/<number>"
    models/story_page.py       TypedDicts: StoryPage, StoryChoice, ElementChange
  application/logs/
    request_trace.py           Every request and its answer, into the log
  core/                        Below the web layer — knows nothing about Flask
    config.py                  .env, APP_ENV, session key, which database file
    story.py                   load_story(): reads and indexes pages.json
    dice.py                    roll_dice(): any dice; roll_2d6() for combat
    character.py               Rolling up Prêtre Jean, once, in one of two modes
    inventory.py               What a gain or a loss does to a hero
    combat.py                  The combat engine — pure, one assault at a time
    db.py                      The engine, the sessions, and every read and write
    logs/
      rotating_log.py          A log file that rotates, on a directory it makes
      general_log.py           note() / event() / failure(), and what they hide
    models/
      base.py                  The declarative base every table is mapped on
      hybrid_document.py       The mixin: a few columns + one JSON blob, to_dict/from_dict
      utc_datetime.py          A timestamp column that keeps its timezone
      page_view.py             PageView: one row per page asked for
      game.py                  Game: a hero, and the fight he is in
      page_inspection.py       PageInspection: a flagged page and its comments
  templates/                   Jinja templates, CSS inline in base.html
  books/<series>/<book>/       Runtime book data (see "Book data" below)

Makefile                       make test / coverage / serve

work/                          Offline import & conversion pipeline
  download_raw_data.rb         Ruby crawler: fetches source HTML into raw_data/
  urls.txt                     Books to crawl (one per line, "#" disables)
  parse_pages.py               HTML -> JSON: page text + outgoing page numbers
  conversions_prompt_actions.txt   LLM prompt: extract choice gains/losses
  conversion_prompt_combat.txt     LLM prompt: detect and structure fights
  new_fable_prompt_for_processing_book.txt  Combined single-pass version
  raw_data/                    Downloaded HTML + per-book YAML index
  parsed_data/                 Generated intermediate JSON/YAML

tests/                         Test suite (pytest, in-memory SQLite, Flask test client)
  conftest.py                  The database every test runs against
.env.example                   APP_ENV and DATABASE_DIR — copy to .env
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
| `GET`  | `/book/<number>`  | `web`     | Reader: a breadcrumb of the last ten pages read, the page's choices, then its text. Unknown page → HTTP 404; unreachable database → HTTP 503. A page with no choices shows "Fin de l'aventure" and a restart link. |
| `GET`  | `/data/<number>`  | `api`     | The raw page object as JSON (text, choices, `fight` when present).                              |
| `GET`  | `/game/new`       | `game`    | The offer to roll a hero up, with a warning when a living one would be left behind. Writes nothing. |
| `POST` | `/game/new`       | `game`    | Rolls up Prêtre Jean and starts a play-through. `mode` in the form picks the difficulty; an unknown one rolls by the book. |
| `GET`  | `/game/resume`    | `game`    | Redirects to wherever the adventure stands: the open fight, the death page, the last page read, page 1, or `/game/new` without a hero. |
| `GET`  | `/game`           | `game`    | The full sheet — Vie, Force, gold, bag, waiting changes, open fight — or the offer to roll a hero up. Read-only. |
| `GET`  | `/heroes`         | `game`    | The heroes who did not come back, most recent first, each with the day he fell.                  |
| `GET`  | `/combat/<number>`| `game`    | The fight a page holds, armed on first arrival. Unknown page, or one whose fight fields nobody → HTTP 404. |
| `POST` | `/combat/<number>/assault` | `game` | Plays one assault and comes back to the fight.                                          |
| `POST` | `/combat/<number>/resolve` | `game` | Closes a decided fight and follows the book to what comes next.                          |
| `POST` | `/book/<number>/choice/<index>` | `game` | Follows one choice, paying what it costs, then redirects to its destination.       |
| `POST` | `/game/pending/<index>/apply` · `/dismiss` · `/game/pending/dismiss` | `game` | The reader's word on a change only he can judge.            |
| `GET`  | `/game/death`     | `game`    | The end of the adventure, and the offer of another hero.                                        |
| `POST` | `/flag-page`      | `inspection` | Files a report on a page: `path`, `title`, `comment` in the form. JSON when the request accepts JSON first (the page's script), otherwise a flash message and a redirect back to the page. A blank comment → HTTP 400. |
| `GET`  | `/flagged-pages`  | `inspection` | The pages flagged for inspection, most recently commented first; `?status=open` or `resolved` narrows it. |
| `GET`  | `/flagged-pages/<id>` | `inspection` | Every remark on one flagged page, and the button to resolve or reopen it. Unknown id → HTTP 404. |
| `POST` | `/flagged-pages/<id>/resolve` · `/reopen` | `inspection` | Changes the state of the file and comes back to it.                       |

`/data/<number>` also appends the requested page to the read history before
looking it up. An unknown page returns a UTF-8 JSON body
`{"success": false, "message": "le numéro N n'a pas été trouvé"}` — note that
this error still carries HTTP 200.

---

## The database

SQLite, through SQLAlchemy — one file per environment, and it holds **three
things: the reading history, the play-throughs and the flagged pages.**

The book does not go in it, deliberately. It is 668 static pages that change only
when the import pipeline is re-run, it fits in memory several times over, and
`core/story.py` reads it off disk at startup. Storing it would buy nothing.

`haute_tension/core/` is everything below the web layer, and nothing there
imports Flask. The layering is strict and the imports only ever go one way:

```
config      environment variables, and which database file
story       the book, read off disk into memory
models      the shape of what is stored — no module here runs a query
db          the engine and the sessions, on top of config and models
```

**`core/db.py` is the only module that talks to SQLite**, and no row object
ever leaves it, which is what keeps the ORM out of the web layer. A new query
belongs here, never in a route.

The engine is opened on the first call rather than at import time, so importing
`core.db` never needs a database file — and it is the single seam the tests
replace. The tables are created on first use from the models: there is no
migration step, the schema is what the models say.

### Which database

`DATABASE_DIR` names the directory the files live in — a relative path is taken
from the repository root, and the directory is made if it is missing.
`APP_ENV` (`dev` or `prod`, default `dev`) picks the file inside it,
`haute_tension_dev.sqlite3` or `haute_tension_prod.sqlite3`, so a dev run never
opens prod data. A `DATABASE_DIR` that names a file rather than a directory is
refused.

### Indexed columns and the JSON blob

Every table follows one pattern, and a new table should too. A field is a
**real column** only when a query filters, sorts or joins on it: the id, the
book every query is scoped to, a timestamp something is ordered by. Everything
else is bundled into **one `data` column holding a JSON object**. A new field
on a hero or a fight is therefore a key in that object and nothing else; a new
query on such a key means promoting it to a column, with an index, in the model.

`core/models/hybrid_document.py` is the mixin every model takes, and its four
methods are the whole contract:

| Method | Does |
| ------ | ---- |
| `to_dict()` | The row as one flat dict — the columns and the blob's keys merged, columns first |
| `from_dict(values)` | The reverse: keys naming a column become that column, the rest is packed into `data` |
| `update_from_dict(values)` | The same onto a loaded row — how `core.db` writes a changed state back, whole |
| `to_json()` | `to_dict()` as text, with instants written as ISO 8601 |

`core.db` never sets a column by hand: it reads a row as a dict, changes the
dict, and writes it back with `update_from_dict()`. An instant kept in a column
goes through `core/models/utc_datetime.py` — aware in, aware out, stored as
fixed-width UTC text so that an `ORDER BY` on it is an order in time. An instant
put in the blob comes back as ISO 8601 text, and is handed out as it is.

### The three tables

| Table | Model | Columns | In the blob |
| ----- | ----- | ------- | ----------- |
| `page_views` | `PageView` | `id` (autoincrement), `book`, `game` (→ `games.id`), `viewed_at` | `page` |
| `games` | `Game` | `id` (a `uuid4().hex`, it travels in the cookie), `book`, `died_at` | the hero, his bag, the pending changes, the open fight, where and of what he died |
| `page_inspections` | `PageInspection` | `id`, `book`, `path`, `status`, `updated_at` | `page_title`, `comments`, `created_at` |

The indexes are the orders the rows are served in: `(book, viewed_at)` and
`(game, viewed_at)` on the history, `(book, died_at)` on the memorial,
`(book, updated_at)` and `status` on the flagged pages, plus the unique
`(book, path)` that keeps two reports on one page from opening two files.
`page_views.game` is a foreign key, enforced — SQLite only checks them when
told to, so the engine turns them on for every connection.

The history is one row per page asked for, scoped by a `book` field written
`"<series>/<book>"`, so a second book never shows up in the first one's history.

- `record_page_view(book, page, game_id)` — notes that a page was asked for, and
  says whether it recorded anything. Asking again for the page one is already on
  is a reload rather than a move, and is dropped; coming back to a page after
  going elsewhere is a loop in the story, and is kept.
- `last_pages(book, limit=10)` / `get_oldest_page(book)` — the bounded reading
  history, oldest first. Bounding happens **on read**, so recording a visit stays
  a plain insert.

### When the database is not there

**A request that needs it fails.** It is not served half-built: a reader handed a
page quietly missing its breadcrumb is worse off than one told the database is
down.

`application/errors.py` registers the one handler that does it, on the
application rather than on a route, so nothing has to remember. It answers
**503** — nothing is wrong with the request or with the application, and the same
request will work once the database is back — as a page for a reader, and as JSON
under the `api` blueprint, because `/data/<number>` is parsed rather than read.
The reason goes to the log with its traceback.

`DatabaseError` is the driver failing — a file that cannot be opened or
written, a constraint refused; `DatabaseUnavailable` is there being nothing
configured to fail. `DatabaseFailure` is both, and is what the handler
catches. `DatabaseUnavailable` subclasses `EnvironmentError` but is a class of
its own, so the handler catches exactly this and not every `OSError` a request
might raise.

What needs no database still works without one: the landing page reads nothing,
and an unknown page is a 404 without a query, because the book is in memory.

---

## The Flask templates

`base.html` carries the layout and the whole stylesheet (serif body, parchment
palette, drop cap on the first paragraph, mobile breakpoint at 36rem).
`index.html` is the landing card; `page.html` renders the breadcrumb, the page
number, the choices block, then the story text.

The breadcrumb (`.trail`) lists the last ten pages read, oldest first, each a
link back except the current one. It can hold ten entries and a revisited page
appears twice, so it scrolls sideways on a narrow screen rather than wrapping.
It is absent, not empty, when there is no history to show — and the page is not
served at all when the database that holds the history cannot be reached.

---

## The game

### Rolling up the hero

Two difficulties, chosen once when the game is created and never changed:

| Mode | Force | Vie | |
| ---- | ----- | --- | - |
| `normal` | 6 + 2D6 → 8–18 | 18 + 2D6 → 20–30 | The rules of the series, from `regles-du-jeu-spj1` |
| `easy` | 12 + 2D4 → 14–20 | 26 + 3D4 → 29–38 | A house rule, not the book's |

The easy mode cannot roll a weakling: the floor of each characteristic is above
the book's average. It barely moves a fight already won — 96% against the tax
collector either way — and decides the ones that are not: against Thalos (Force
18, Vie 22, adjustment +1) it takes the hero from 13% to 45%.

Both throws happen **once**, in `db.start_game()`, and are written; every later
read loads them back, so reopening a game never re-rolls it. `vie_max` is what
was thrown and never moves again; `vie_actuelle` is what is left, and is the only
thing combat writes. The mode is stored with the hero, so the sheet can print the
throw that made him.

A Force of 17 or more earns an *Ajustement-Force* (+1 at 17, +2 from 18 up),
which adds to the damage the hero's blows do — the mirror of the `AJUSTEMENT
DOMMAGES` some adversaries carry. The book's table stops at 18 because its own
Force does; the easy mode reaches 20, so the top step is read as "18 or more"
rather than "18 exactly".

### What the hero carries

He sets out, from `regles-du-jeu-spj1`, with "votre épée et un sac", "4 rations
de provisions", and a purse of two throws of two dice — 4 to 24 pieces of gold.

`core/inventory.py` reads the `gains` and `losses` the import pipeline attached
to every choice. Three elements are the hero rather than his bag:

| Element | Is |
| ------- | -- |
| `life point` | Vie, which **may never rise above the Vie he started with** — "vous ne pourrez le dépasser en aucun cas" |
| `strength point` | Force |
| `gold coin` | the purse |

Everything else is an item, counted: two vials are one line saying two. Nothing
goes below zero, and an item drops out of the bag once none of it is left.

### The menu

The site header is the same four entries on every page, hero or not:

| Entry | Route | What it does |
| ----- | ----- | ------------ |
| **Nouveau** | `GET /game/new` | Offers the two difficulties. It writes nothing: the dice are only thrown by the `POST`, so the page is the confirmation. While a hero is alive it shows him as he stands — Force, Vie, gold, the fight he is in — and says that rolling another leaves him behind: his game stays in the database but the session forgets him, and being abandoned is not dying, so he does not join the fallen. A dead hero is simply mentioned. |
| **Partie en cours** | `GET /game/resume` | Puts the reader back in the story rather than on a sheet: the fight he is in, the death page if he fell, the last paragraph *this* hero read, or page 1 if he has read nothing. Without a hero, the offer to roll one up. It records no visit of its own. |
| **Feuille** | `GET /game` | The sheet, read-only: nothing on it posts. Without a hero, the same offer as **Nouveau**. |
| **Les tombés** | `GET /heroes` | The memorial, or "Personne n'est encore tombé". |

Nothing about the hero lives in the cookie — only the game id — and none of the
four entries writes, so moving between them cannot leave a play-through in two
states.

A fifth entry, **Signaler**, is not about the hero: it opens a dialog to flag
the page being read as having a problem — see *Flagging a page* below.

### Flagging a page

Any page can be flagged from the header's **Signaler**: a dialog with the page's
path and title already filled in and a box for what is wrong. With JavaScript
the report is sent by `fetch` and the answer shown on the page; without it the
dialog still opens (the link lands on its anchor) and the form posts the
ordinary way, coming back with a flash message. Either way it is one route,
`POST /flag-page`, which answers JSON only when asked for it first.

One file per page and per book, in the `page_inspections` table: the first
report opens it, every later one appends a dated comment, and a resolved page
that is flagged again is reopened. A unique constraint on `(book, path)` is the
safety net under that find-or-create. There are no users, so comments carry no
author.

`/flagged-pages` lists the files, most recently commented first, with the last
remark and a filter on state; each file shows the whole thread and can be
marked resolved or reopened. Nothing restricts it: the application has no
accounts, and the list is a tool for whoever maintains the book.

### The sheet

`/game` is the whole of a hero on one page: **Vie** out of the maximum he was
rolled with, drawn as a gauge, **Force** with its Ajustement, **gold**, and the
**bag** — each with the throw that produced it, and the changes still waiting on
the reader's word, each linked to the paragraph they are ruled on. It carries the
way back to the story too, because it is reached mid-adventure rather than
instead of one.

That way back is **scoped to the play-through**, not to the book. The reading
history is a book's, and two heroes of the same book each stopped somewhere of
their own: sending the living one back to where a dead one fell would be worse
than offering nothing.

Every story page carries a banner — Force, Vie, gold — and a link to the sheet
saying how many things are in the bag, and the site header links to it from
everywhere.

### The fallen

A hero is laid to rest the moment his Vie reaches zero: `died_at`, the page he
fell on, and what killed him — the adversary that struck the last blow, or "les
épreuves du chemin" for a paragraph that did it. The first death is the one that
counts; nothing afterwards moves the date on the stone.

`/heroes` remembers them, most recent first, with the day they fell, the
difficulty they chose, what they were worth and what they were still carrying. **Being abandoned is not
dying**: a hero left behind in good health is simply left behind, and does not
join them.

Following a choice is a **POST**, not a link, because it spends rations and gold;
the redirect afterwards is what keeps a reload from spending them twice.

### What the reader has to rule on

**65 of the book's 275 changes carry a condition the code cannot evaluate** —
either a prerequisite ("Si vous avez des provisions dans votre sac") or a
duration ("pendant tout le temps où vous porterez cette cuirasse") — or an amount
the page's text asks to be rolled (`note: "dice"`).

Those are never applied on their own. They wait in the game's `pending` list and
appear on the page under **"À vous de voir"**, each with its condition in the
book's own words and a button to apply or dismiss it. The reader is the one who
knows whether he has eaten; the other 210 changes apply by themselves.

`note: "all"` is the one note the code *can* act on: everything of that element
goes, however much there was.

### Fighting

`core/combat.py` is the engine, and it is pure: no database, no Flask, no clock.
It is handed the state of a fight and gives back the state after one assault.

- **Force d'Attaque** — "jetez deux dés. Ajoutez au résultat votre total de
  Force du moment."
- **Damage is the gap** — "retirez la Force d'Attaque la plus faible de la Force
  d'Attaque la plus élevée." An assault won by one point barely stings; one won
  by ten is close to lethal. Adjustments are added to the gap.
- **Divine judgement** — a hero's double 6 kills outright, an adversary's double
  1 kills the hero. Neither computes a gap or applies an adjustment.
- **Melee** — one hero roll per assault, compared to each adversary's own:
  "lancez deux dés pour vous et deux dés pour chacun de vos adversaires."
  A `sequential` fight is a queue instead, one adversary at a time.

Two things the rules leave open are decided in the engine and marked as choices:
**equal Forces d'Attaque cost nobody anything**, and **a hero's double 6
outranks an adversary's double 1** in the same assault.

### What the engine does not model

Thirteen of the book's forty-seven fights add a rule of their own — a fight
decided on the first assault, a limit of three assaults, a Force halved under
water, a branch on the first wound or on a Vie threshold. They are listed in
`combat.FIGHTS_WITH_SPECIAL_RULES`, the combat page warns the reader on them,
and `TODO.md` says what each one needs.

### State

A play-through lives in the `games` table; the session cookie carries its id
and nothing else. It is the only row that is read, changed and written back
— the book is static and the reading history is a log.

---

## The log

One file, `logs/general.log`, not versioned and made on first write. It is the
server's own trace: what is read when something has gone wrong and the page
itself has nothing to say about it. The directory is made again at every open,
so a `logs/` taken away under a running server — a `git clean -fdx`, a hand —
costs the lines written in the meantime and nothing more, rather than a
traceback on stderr per request until the next restart.

**DEBUG, and on.** `LOG_LEVEL` raises it; a trace one must first go and turn on
is a trace one does not have on the day it is needed. It rotates at 512 KB and
keeps five archives, so yesterday's run is still there today.

Two ways in, and one rule for both — **name every variable and write out its
content**:

```python
note("Assault played", game=game.id, number=3, hero_vie=21)   # a step, DEBUG
event("Hero rolled up", force=15, vie=24)                     # it happened, INFO
failure("The database refused", trouble=error, page="22")     # ERROR + traceback
```

```
2026-09-09 22:06:23  Hero rolled up — game='64e57937…', force=14, vie=27, force_dice=[5, 3]
2026-09-09 22:06:23  Combat armed — game='64e57937…', page='22', enemies=["collecteur d'impots"]
```

`request_trace.py` is wired onto the application itself rather than a blueprint,
so nothing is outside it: every request leaves a `Request` line and an `Answer`
line, and a third when it fails. A JSON answer is written in full; a page is
described (`<12138 bytes of text/html>`); a refusal is read whatever its shape,
because its body carries the sentence explaining it; a streamed answer is left
untouched, since reading it would consume it.

### What it never writes

A field whose name says `secret`, `token`, `password`, `authorization`,
`cookie`, `session` or `key` is replaced by its length — at the top level, and
**inside any body being logged**, because a body is where a secret travels.

`game_id` is the exception this application adds. It is not a secret by name, but
it is the only credential here: whoever holds one can pick up that play-through.
It is written to its first eight characters — enough to follow one reader through
a run, useless to anyone who reads the file.

`LOG_VALUE_LIMIT` cuts a value beyond 2000 characters and says by how much; 0
there writes every answer whole.

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

Dependencies come from `pyproject.toml`: Flask, SQLAlchemy, python-dotenv,
PyYAML, beautifulsoup4/bs4. Do not add a `requirements.txt`. Add `[test]` to
also install pytest and pytest-cov:

```bash
python -m pip install -e ".[test]"
```

### 3. Database

Copy `.env.example` to `.env` (git-ignored) and say where the files go:

```bash
APP_ENV=dev
DATABASE_DIR=data
```

`DATABASE_DIR` is a directory, made on first use if it is missing; `APP_ENV`
picks the file inside it (`data/haute_tension_dev.sqlite3` here, git-ignored),
and the tables are created the first time the file is opened. There is nothing
to install or bring up: SQLite ships with Python.

Only the reading history, the play-throughs and the flagged pages need it. The
book is read off disk, so the landing page works whether or not a database is
configured.

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

There is nothing to bring up: every test runs on an in-memory SQLite of its
own, which is the same engine as the file the application opens.

```bash
make test        # the whole suite, well under ten seconds
python -m pytest # the same
```

| Target | Does |
| ------ | ---- |
| `make test` | Runs the whole suite |
| `make coverage` | The suite plus an HTML report in `htmlcov/` |
| `make serve` | Runs the development server on port 5001 |

`ARGS` passes arguments through to pytest (`make test ARGS="-k history -v"`).

`tests/conftest.py` opens the in-memory database, creates the tables, binds
`core.db` to it and stubs out `core.db.connect_db()` — the one place that would
open a file of its own. Three fixtures are the whole interface:

- `fake_db` — the empty database. Seed a table by assigning to
  `fake_db["page_views"].docs`, a list of flat dicts in the `to_dict()` shape —
  columns and blob keys mixed; reading `.docs` gives the same shape back.
- `books_path` — a small two-page book written to a temporary directory, so no
  test reads the packaged one.
- `client` — a Flask test client serving that book, with a database behind it.

593 tests cover the engine and its guards (`test_core_connection.py`), the
hybrid models (`test_core_models.py`), the reading history (`test_core_db.py`),
how a run picks its database file (`test_core_config.py`), loading a book and every malformed input it rejects
(`test_story.py`), the two dice (`test_core_dice.py`), rolling up a hero in either mode
(`test_core_character.py`), what a gain or a loss does — every one the book
carries (`test_core_inventory.py`), the sheet and the bag through the browser
(`test_inventory_routes.py`), the memorial (`test_fallen_heroes.py`), every combat rule on its own with fixed dice
(`test_core_combat.py`), play-through persistence (`test_core_games.py`), the
data route (`test_routes.py`), browser navigation and the breadcrumb
(`test_web_routes.py`), the character and combat routes (`test_game_routes.py`),
the app factory and entry point (`test_app.py`), what the log writes and what it
must never write (`test_core_logs.py`), the request trace
(`test_request_trace.py`), and what a dead database costs every route
(`test_errors.py`). Current coverage: 100%.

Nothing in the suite is left to chance: `core.dice`, `core.character` and
`core.combat` all take a `random.Random`, and `create_app(rng=...)` threads one
through the whole application, so a rule is asserted against dice chosen for it
rather than against a lucky seed.

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
  query goes in `core/db.py`, never in a route, and no row object leaves that
  module. A new table follows the hybrid pattern: real columns only for what is
  queried, one JSON `data` blob for the rest.
- Code, identifiers and comments in English even when the discussion is in
  French; user-facing French text stays UTF-8.
- No global `try`/`except` around entry points — let tracebacks surface.
- Commit subjects are short imperative sentences (`Page parsing reworked.`); no
  `wip` subjects in review-ready work.

Never commit `.env`, caches or logs.

---

## License

MIT — see `LICENSE`.
