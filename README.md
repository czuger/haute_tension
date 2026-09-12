# Haute Tension

Une mise en ligne du livre dont vous êtes le héros : *La Forteresse d'Alamuth*
(saga du Prêtre Jean).

The project has two halves:

1. **An Axum reader** (`src/`) that serves the book as a browsable website, plus
   a small JSON API. The book is read off disk into memory at startup; a SQLite
   file holds the reading history, the play-throughs and the flagged pages, and
   nothing else.
2. **An offline import pipeline** (`src/pipeline/`, run over `work/`) that parses
   the original pages — crawled by `work/download_raw_data.rb` — into JSON and
   enriches them (per-choice gains/losses, structured combat encounters) into the
   runtime book data.

Rust 1.85 or later, edition 2021. Nothing else has to be installed: SQLite is
linked in.

---

## Repository layout

```
Cargo.toml                     The crate: the library, and two binaries
askama.toml                    Names the crate's own HTML escaper (see src/escape.rs)

src/
  lib.rs                       The two halves: application over core
  bin/serve.rs                 Development server (127.0.0.1:5001)
  bin/parse_pages.rs           The import pipeline, run by hand
  application/                 The web layer — knows nothing about the database
    factory.rs                 create_app(): the router, the book, the trace
    routes/web.rs              Browser routes: "/" and "/book/{number}"
    routes/api.rs              API route: "/data/{number}"
    routes/game.rs             Rolling a hero up, fighting, following a choice
    routes/inspection.rs       Flagging a page, and reviewing what was flagged
    routes/mod.rs              What every handler shares: state, forms, refusals
    templates.rs               One struct per template, checked at compile time
    session.rs                 The signed cookie carrying a play-through id
    errors.rs                  The one place a failure becomes an answer
    french_date.rs             A date in French, for the memorial
    models/story_page.rs       The book's shapes: StoryPage, StoryChoice, ElementChange
    models/game.rs             The shapes a template is handed: GameDict, Combat, Pending
    logs/request_trace.rs      Every request and its answer, into the log
  core/                        Below the web layer — knows nothing about HTTP
    config.rs                  .env, APP_ENV, session key, which database file
    story.rs                   load_story(): reads and indexes pages.json
    dice.rs                    The Rng seam: roll_2d6() and the dice behind it
    character.rs               Rolling up Prêtre Jean, once, in one of two modes
    inventory.rs               What a gain or a loss does to a hero
    combat.rs                  The combat engine — pure, one assault at a time
    db/mod.rs                  Every read and write, as plain values
    db/connection.rs           Opening the engine, and the schema version it needs
    db/store.rs                A row as one flat dict, and a changed one written back
    db/schema.rs               The tables, spelled out, and SCHEMA_VERSION
    logs/rotating_log.rs       A log file that rotates, on a directory it makes
    logs/general_log.rs        note() / event() / failure(), and what they hide
    models/hybrid_document.rs  The trait: columns, child rows and one JSON blob
    models/utc_datetime.rs     A timestamp column that keeps its timezone
    models/page_view.rs        PageView: one row per page asked for
    models/game.rs             Game: a hero, and the fight he is in
    models/item.rs             Item: one line of a hero's bag
    models/page_inspection.rs  PageInspection: a flagged page and its comments
  escape.rs                    HTML escaping as the pages carry it
  pipeline/parse_pages.rs      HTML -> JSON: page text + outgoing page numbers

templates/                     Askama templates, CSS inline in base.html
books/<series>/<book>/         Runtime book data (see "Book data" below)
migrations/                    SQL scripts moving an existing database file to the current schema
tests/                         The suite; tests/common/mod.rs is the harness
Makefile                       make test / coverage / lint / serve / import

work/                          Offline import & conversion sources
  download_raw_data.rb         Ruby crawler: fetches source HTML into raw_data/
  urls.txt                     Books to crawl (one per line, "#" disables)
  conversions_prompt_actions.txt   LLM prompt: extract choice gains/losses
  conversion_prompt_combat.txt     LLM prompt: detect and structure fights
  new_fable_prompt_for_processing_book.txt  Combined single-pass version
  raw_data/                    Downloaded HTML + per-book YAML index
  parsed_data/                 Generated intermediate JSON/YAML

.env.example                   APP_ENV and DATABASE_DIR — copy to .env
AGENTS.md                      Coding conventions for this repository
```

---

## The application

### Entry point and factory

`src/bin/serve.rs` builds the app with `create_app()` and serves it on
`127.0.0.1:5001`. It binds loopback only, so nothing on the local network reaches
a development run; change `ADDRESS` if you want to read the book from another
device on your wifi.

`application/factory.rs` holds all the wiring:

| Constant / function | Value                            |
| ------------------- | -------------------------------- |
| `BOOK`              | `pretre_jean/forteresse_alamuth` |
| `books_path()`      | `<repository root>/books`        |
| `BOOK_SERIES`       | `Prêtre Jean`                    |
| `BOOK_TITLE`        | `La Forteresse d'Alamuth`        |

`create_app(AppConfig)` takes the book, the directory it lives in, the database,
the source of chance and the key the session is signed with — which is how the
tests run against fixture books, fixed dice and a database of their own. It reads
the book **once, at startup**, and holds it in memory behind an `Arc`.

### Routes

| Method | Path              | Family | Description                                                                                   |
| ------ | ----------------- | ------ | --------------------------------------------------------------------------------------------- |
| `GET`  | `/`               | `web`  | Landing page: series, title, and a link to the opening page (page 1).                          |
| `GET`  | `/book/{number}`  | `web`  | Reader: a breadcrumb of the last ten pages read, the page's choices, then its text. Unknown page → HTTP 404; unreachable database → HTTP 503. A page with no choices shows "Fin de l'aventure" and a restart link. |
| `GET`  | `/data/{number}`  | `api`  | The raw page object as JSON (text, choices, `fight` when present).                              |
| `GET`  | `/game/new`       | `game` | The offer to roll a hero up, with a warning when a living one would be left behind. Writes nothing. |
| `POST` | `/game/new`       | `game` | Rolls up Prêtre Jean and starts a play-through. `mode` in the form picks the difficulty; an unknown one rolls by the book. |
| `GET`  | `/game/resume`    | `game` | Redirects to wherever the adventure stands: the open fight, the death page, the last page read, page 1, or `/game/new` without a hero. |
| `GET`  | `/game`           | `game` | The full sheet — Vie, Force, gold, bag, waiting changes, open fight — or the offer to roll a hero up. Read-only. |
| `GET`  | `/heroes`         | `game` | The heroes who did not come back, most recent first, each with the day he fell.                  |
| `GET`  | `/combat/{number}`| `game` | The fight a page holds, armed on first arrival. Unknown page, or one whose fight fields nobody → HTTP 404. |
| `POST` | `/combat/{number}/assault` | `game` | Plays one assault and comes back to the fight.                                          |
| `POST` | `/combat/{number}/resolve` | `game` | Closes a decided fight and follows the book to what comes next.                          |
| `POST` | `/book/{number}/choice/{index}` | `game` | Follows one choice, paying what it costs, then redirects to its destination.       |
| `POST` | `/game/pending/{index}/apply` · `/dismiss` · `/game/pending/dismiss` | `game` | The reader's word on a change only he can judge.            |
| `GET`  | `/game/death`     | `game` | The end of the adventure, and the offer of another hero.                                        |
| `POST` | `/flag-page`      | `inspection` | Files a report on a page: `path`, `title`, `comment` in the form. JSON when the request accepts JSON first (the page's script), otherwise a flash message and a redirect back to the page. A blank comment → HTTP 400. |
| `GET`  | `/flagged-pages`  | `inspection` | The pages flagged for inspection, most recently commented first; `?status=open` or `resolved` narrows it. |
| `GET`  | `/flagged-pages/{id}` | `inspection` | Every remark on one flagged page, and the button to resolve or reopen it. Unknown id → HTTP 404. |
| `POST` | `/flagged-pages/{id}/resolve` · `/reopen` | `inspection` | Changes the state of the file and comes back to it.                       |

A handler holds no logic: it calls one `core::db` function and renders. It
catches nothing — a database that cannot answer leaves as an `AppError`, and
`application/errors.rs` is the one place that turns it into an answer.

A page number is **parsed, not extracted**: the handlers take the segment as
text and `routes::whole_number` answers 404 for anything that is not digits, so
`/book/abc` is a page that does not exist rather than a bad request.

`/data/{number}` also appends the requested page to the read history before
looking it up. An unknown page returns a UTF-8 JSON body
`{"success": false, "message": "le numéro N n'a pas été trouvé"}` — note that
this error still carries HTTP 200.

---

## The database

SQLite, through SeaORM — one file per environment, and it holds **three things:
the reading history, the play-throughs and the flagged pages.**

The book does not go in it, deliberately. It is 668 static pages that change only
when the import pipeline is re-run, it fits in memory several times over, and
`core/story.rs` reads it off disk at startup. Storing it would buy nothing.

`src/core/` is everything below the web layer, and nothing there knows what a
request is. The layering is strict and the imports only ever go one way:

```
config      environment variables, and which database file
story       the book, read off disk into memory
models      the shape of what is stored — no module here runs a query
db          the engine and the queries, on top of config and models
```

**`core::db` is the only module that talks to SQLite**, and no row ever leaves
it: callers get plain values back, which is what keeps the ORM out of the web
layer. A new query belongs here, never in a handler.

The engine is opened on the first call rather than at startup, so building the
application never needs a database file. A `Db` is what a handler holds, and the
four shapes it takes are the four situations there are:

| `Db`          | Is                                                                    |
| ------------- | --------------------------------------------------------------------- |
| `Environment` | The file `APP_ENV` picks, opened once into a process-wide connection   |
| `Bound`       | A connection handed over ready-made — what the tests give the routes   |
| `Unavailable` | Nothing is configured, so no request can pretend otherwise             |
| `Failing`     | A connection that refuses, which is how a dead database is tested      |

A new file is given the tables `db/schema.rs` spells out, and marked with
`SCHEMA_VERSION` in `PRAGMA user_version`. A file carrying another version is
refused with the 503 page until it is migrated (see "Migrations" below).

### Which database

`DATABASE_DIR` names the directory the files live in — a relative path is taken
from the repository root, and the directory is made if it is missing.
`APP_ENV` (`dev` or `prod`, default `dev`) picks the file inside it,
`haute_tension_dev.sqlite3` or `haute_tension_prod.sqlite3`, so a dev run never
opens prod data. A `DATABASE_DIR` that names a file rather than a directory is
refused.

### Indexed columns and the JSON blob

Every table follows one pattern, and a new table should too. A field is a
**real column** when a query filters, sorts or joins on it, or when the database
has a rule to hold about it: the id, the book every query is scoped to, a
timestamp something is ordered by, the hero's Force, Vie and gold, which may
never go below zero. Everything else is bundled into **one `data` column
holding a JSON object**. A new field on a fight is therefore a key in that
object and nothing else. Promoting a key to a column is a schema change: it
takes the model, and a migration for the files that already exist.

A row's children live in a table of their own but travel inside the parent's
dict: a game's bag is rows of `items`, and reads and writes as the game's
`items` list.

`core/models/hybrid_document.rs` is the trait every model implements, and its
four methods are the whole contract:

| Method | Does |
| ------ | ---- |
| `to_dict()` | The row as one flat dict: its columns first, then its children, then the blob's keys |
| `from_dict(values)` | The reverse: keys naming a column become that column, keys naming a relationship become child rows, the rest is packed into `data` |
| `update_from_dict(values)` | The same onto a loaded row, which is how `core::db` writes a changed state back. A child dict with an `id` updates that row, one without is a new row, and a row no dict names is deleted |
| `to_json()` | `to_dict()` as text, with instants written as ISO 8601 |

A dict is a `RowDict` — an `IndexMap` of `Field`, which is JSON plus the two
things JSON has no shape for: an `Instant` and a `Date`. That is what lets one
instant come back as a `DateTime` from a *column* and as ISO 8601 text from the
*blob*, which is the contract every file already written holds, and the one the
tests assert. The `row!` macro writes one literally.

`core::db` never sets a column by hand: it reads a row as a dict, changes the
dict, and writes it back through `db/store.rs`. An instant kept in a column goes
through `core/models/utc_datetime.rs` — aware in, aware out, stored as
fixed-width UTC text so that an `ORDER BY` on it is an order in time. An instant
put in the blob comes back as ISO 8601 text, and is handed out as it is.

`db/store.rs` is also where `created_at` and `updated_at` are stamped, for the
tables whose rows change. On insert both are the same instant; a row that is
written moves `updated_at` unless the dict names one itself, and **a row whose
columns and blob are unchanged is not written at all**, so its `updated_at` does
not move. Only the row that changed moves: a bag line changing count does not
move its game. The column defaults write the same fixed-width text for a row
inserted by hand. `page_views` has neither column: a visit is never changed, and
`viewed_at` already says when it was written.

### The four tables

Every id is an integer the database counts up and never reuses
(`AUTOINCREMENT`). A game's id travels in the session cookie, whose signature is
what keeps a reader from writing another hero's number into it.

| Table | Model | Columns | In the blob |
| ----- | ----- | ------- | ----------- |
| `games` | `Game` | `id`, `book`, `force`, `vie_max`, `vie_actuelle`, `gold`, `died_at`, `created_at`, `updated_at` | the mode and the dice, the pending changes, the open fight, where and of what he died |
| `items` | `Item` | `id`, `game_id` (→ `games.id`), `created_at`, `updated_at` | `element`, `label`, `count`: one row per line of the bag |
| `page_views` | `PageView` | `id`, `book`, `game_id` (→ `games.id`), `viewed_at` | `page` |
| `page_inspections` | `PageInspection` | `id`, `book`, `path`, `status`, `created_at`, `updated_at` | `page_title`, `comments` |

`force`, `vie_max`, `vie_actuelle` and `gold` are `SMALLINT` with a `CHECK`
holding each at zero or above. SQLite enforces no column width, so the checks
are what the database actually guarantees.

The indexes are the orders the rows are served in: `(book, viewed_at)` and
`(game_id, viewed_at)` on the history, `(book, died_at)` on the memorial,
`game_id` on the bag, `(book, updated_at)` and `status` on the flagged pages,
plus the unique `(book, path)` that keeps two reports on one page from opening
two files. Both foreign keys are enforced — SQLite only checks them when told
to, so the engine turns them on for every connection.

The history is one row per page asked for, scoped by a `book` field written
`"<series>/<book>"`, so a second book never shows up in the first one's history.

- `record_page_view(book, page, game_id)` — notes that a page was asked for, and
  says whether it recorded anything. Asking again for the page one is already on
  is a reload rather than a move, and is dropped; coming back to a page after
  going elsewhere is a loop in the story, and is kept.
- `last_pages(book, limit)` / `get_oldest_page(book)` — the bounded reading
  history, oldest first. Bounding happens **on read**, so recording a visit stays
  a plain insert.

### Migrations

`migrations/` holds one SQL script per schema version, numbered from `001`. Each
moves a file from the version before to its own. `SCHEMA_VERSION` in
`core/db/schema.rs` is the number of the last one, and `tests/test_migrations.rs`
checks that they agree and that a migrated file ends with exactly the schema a
new file gets — comparing the SQL `sqlite_master` holds, character for character.
That comparison is why the `CREATE TABLE` statements are spelled out in
`schema.rs` rather than generated: a builder emitting its own spacing would fail
it without anything being wrong.

`001_integer_ids_items_and_stats.sql` moves a file the previous schema wrote —
version 0: uuid ids, the bag and the stats inside the blob — to version 1. Stop
the server, back the file up, and apply the script with `-bail`, so the first
error stops it before anything is committed:

```bash
cp data/haute_tension_dev.sqlite3 data/haute_tension_dev.sqlite3.before-001
sqlite3 -bail data/haute_tension_dev.sqlite3 < migrations/001_integer_ids_items_and_stats.sql
```

It is one transaction. It refuses anything but a version-0 file, and before it
commits it checks that every game, bag line, visit and flagged page came across
and that every reference resolves; a failed check leaves the file exactly as it
was. Games and flagged pages are renumbered in the order they were created, and
keep their old uuid in the blob as `legacy_id`, so an old log line can still be
traced to them. Visits keep their ids. Bag lines are stamped with the time of
the migration, since the old schema never recorded when a thing was picked up.

A reader whose session cookie still holds an old uuid is shown as having no
hero: the cookie names an id that no longer exists. The game itself is intact,
and on the memorial if he fell.

### When the database is not there

**A request that needs it fails.** It is not served half-built: a reader handed a
page quietly missing its breadcrumb is worse off than one told the database is
down.

`application/errors.rs` is the one place that does it, and every handler reaches
it by returning rather than by remembering to catch. It answers **503** — nothing
is wrong with the request or with the application, and the same request will work
once the database is back — as a page for a reader, and as JSON under
`/data/{number}` or for a caller that puts JSON first in `Accept`, because that
answer is parsed rather than read. The reason goes to the log, named, on the
`Request failed` line the trace writes.

`DatabaseFailure::Driver` is the driver failing — a file that cannot be opened or
written, a constraint refused; `DatabaseFailure::Unavailable` is there being
nothing configured to fail. Both reach the same answer, and the refusal carries
its kind on the response so the trace can name it.

What needs no database still works without one: the landing page reads nothing,
and an unknown page is a 404 without a query, because the book is in memory.

---

## The templates

`templates/` is Askama: one struct in `application/templates.rs` per file, each
checked against its template when the crate is compiled, so a field a template
reads and nobody fills is a build error rather than a blank page.

`base.html` carries the layout and the whole stylesheet (serif body, parchment
palette, drop cap on the first paragraph, mobile breakpoint at 36rem). Every
struct hands it a `Layout` — the page's title, the address it was reached at, and
anything waiting to be said to the reader — which is what draws the menu, the
flash messages and the flag dialog. `index.html` is the landing card; `page.html`
renders the breadcrumb, the page number, the choices block, then the story text.

There is no reverse routing: a template writes the address it links to. The
endpoint names still exist, in `routes::endpoint_of`, because that is what the log
line `endpoint='web.read_page'` is read off.

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

A mode is two `Throw`s — a base plus `count`D`faces` — so a new difficulty is an
entry in `character::MODES`, not a branch, and the sheet prints whatever formula
made the hero.

Both throws happen **once**, in `db::start_game()`, and are written; every later
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

`core/inventory.rs` reads the `gains` and `losses` the import pipeline attached
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
difficulty they chose, what they were worth and what they were still carrying.
**Being abandoned is not dying**: a hero left behind in good health is simply
left behind, and does not join them.

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

`core/combat.rs` is the engine, and it is pure: no database, no HTTP, no clock.
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
`combat::FIGHTS_WITH_SPECIAL_RULES`, the combat page warns the reader on them,
and `TODO.md` says what each one needs.

### State

A play-through lives in the `games` table and its bag in `items`; the session
cookie carries the game's id and nothing else. A game and its bag are read,
changed and written back together — the book is static and the reading history
is a log.

The cookie is signed, not encrypted: a base64url JSON payload and an
HMAC-SHA256 of it under `SESSION_SECRET_KEY`, `HttpOnly`, `Path=/`,
`SameSite=Lax`. It carries the game id and the flash messages, and its signature
is what keeps a reader from writing another hero's number into it. A cookie
naming no game is a reader with no hero, which every route already handles.

---

## The log

One file, `logs/general.log`, not versioned and made on first write. It is the
server's own trace: what is read when something has gone wrong and the page
itself has nothing to say about it. The directory is made again at every open,
so a `logs/` taken away under a running server — a `git clean -fdx`, a hand —
costs the lines written in the meantime and nothing more.

**DEBUG, and on.** `LOG_LEVEL` raises it; a trace one must first go and turn on
is a trace one does not have on the day it is needed. It rotates at 512 KB and
keeps five archives, so yesterday's run is still there today.

Two ways in, and one rule for both — **name every variable and write out its
content**:

```rust
note("Assault played", vars![("game", 12), ("number", 3), ("hero_vie", 21)]);  // a step, DEBUG
event("Hero rolled up", vars![("force", 15), ("vie", 24)]);                    // it happened, INFO
failure("The database refused", vars![("trouble", &error), ("page", "22")]);   // ERROR
```

```
2026-09-09 22:06:23  Hero rolled up — game='12', force=14, vie=27, force_dice=[5, 3]
2026-09-09 22:06:23  Combat armed — game='12', page='22', enemies=["collecteur d'impots"]
```

`application/logs/request_trace.rs` is a layer wrapped around the whole router
rather than one family of routes, so nothing is outside it: every request leaves
a `Request` line and an `Answer` line, and a third when it fails. A JSON answer
is written in full; a page is described (`<12138 bytes of text/html>`); a refusal
is read whatever its shape, because its body carries the sentence explaining it;
a body whose length is not known in advance is described rather than read, since
reading it would consume it.

### What it never writes

A field whose name says `secret`, `token`, `password`, `authorization`,
`cookie`, `session` or `key` is replaced by its length — at the top level, and
**inside any body being logged**, because a body is where a secret travels.

`game_id` is the exception this application adds, written to its first eight
characters. Game ids are small integers the database counts up, so in practice
they are written whole: guessing one gains nothing, because the session cookie
that carries it is signed. The cut still shortens anything longer, such as an
old uuid left in a cookie from before migration 001.

`LOG_VALUE_LIMIT` cuts a value beyond 2000 characters and says by how much; 0
there writes every answer whole.

---

## Book data

Runtime books live in `books/<series>/<book>/`:

- **`pages.json`** — the file the application loads, named by
  `core::story::PAGES_FILE`. A JSON list of 668 page objects for *La Forteresse
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

The shapes are read through `application/models/story_page.rs`: `StoryPage`
(with optional `fight`), `StoryChoice` (`goto`, `gains`, `losses`) and
`ElementChange` (`element`, `amount`).

---

## Getting started

### 1. Toolchain

Rust 1.85 or later, from [rustup](https://rustup.rs). Nothing else: SQLite is
linked into the binary.

```bash
cargo build
```

### 2. Database

Copy `.env.example` to `.env` (git-ignored) and say where the files go:

```bash
APP_ENV=dev
DATABASE_DIR=data
```

`DATABASE_DIR` is a directory, made on first use if it is missing; `APP_ENV`
picks the file inside it (`data/haute_tension_dev.sqlite3` here, git-ignored),
and the tables are created the first time the file is opened. There is nothing
to install or bring up. A file written by an older schema is refused until it is
migrated, as "Migrations" above describes.

Only the reading history, the play-throughs and the flagged pages need it. The
book is read off disk, so the landing page works whether or not a database is
configured.

### 3. Run the server

```bash
make serve            # or: cargo run --bin serve
```

Then open <http://localhost:5001/>.

### 4. Manual checks

```bash
curl http://localhost:5001/data/1
curl http://localhost:5001/data/99999      # unknown page, HTTP 200 + JSON error
```

---

## Tests

All tests live in the repository-root `tests/` directory, one binary per file,
and run under `cargo test`. There is nothing to bring up: every test runs on an
in-memory SQLite of its own, which is the same engine as the file the
application opens.

```bash
make test        # the whole suite, well under ten seconds
cargo test       # the same
```

| Target | Does |
| ------ | ---- |
| `make test` | Runs the whole suite |
| `make coverage` | The suite plus an HTML report in `target/llvm-cov/html/` |
| `make lint` | `cargo clippy --all-targets` |
| `make serve` | Runs the development server on port 5001 |
| `make import` | Runs the import pipeline over `work/raw_data` |

`ARGS` passes arguments through to cargo (`make test ARGS="history"`).

`tests/common/mod.rs` is the harness, and a handful of helpers are the whole
interface:

- `fake_db()` — an in-memory database of this test's own, tables created through
  the real `create_the_schema`, handed to the routes as a `Db::Bound`. Seed a
  table with `db.table("games").seed(vec![...])`, flat dicts in the `to_dict()`
  shape — columns and blob keys mixed, a game's bag riding along as its `items`
  list, an `id` updating that row and none inserting a new one, a row left out
  deleted; `docs()` gives the same shape back, and the `row!` macro writes one.
- `books_path()` / `books_path_with_fights()` — a small two-page book written to
  a temporary directory, so no test reads the packaged one.
- `client()` / `fighting_client()` / `hero()` — test clients over that book, the
  last with a hero already rolled up; `no_database_client()` and
  `dead_database_client()` are the two ways it can fail. They keep cookies
  between requests, so a session survives a redirect.
- `fixed(&[…])` — dice chosen for the rule being checked; `seeded(n)` when a run
  only has to be reproducible.
- `env_lock()` / `log_lock()` — the environment and the log capture belong to the
  process, and cargo runs a file's tests in parallel threads, so a test touching
  either takes the lock.

654 tests cover the engine, its guards and the schema version
(`test_core_connection.rs`), the hybrid models, the bag as rows, the stat
columns and the timestamps (`test_core_models.rs`), migration 001 applied to a
version-0 file (`test_migrations.rs`), the reading history (`test_core_db.rs`),
how a run picks its database file (`test_core_config.rs`), loading a book and
every malformed input it rejects (`test_story.rs`), the two dice
(`test_core_dice.rs`), rolling up a hero in either mode
(`test_core_character.rs`), what a gain or a loss does — every one the book
carries (`test_core_inventory.rs`), the sheet and the bag through the browser
(`test_inventory_routes.rs`), the memorial (`test_fallen_heroes.rs`,
`test_memorial_page.rs`), every combat rule on its own with fixed dice
(`test_core_combat.rs`), play-through persistence (`test_core_games.rs`), the
data route (`test_routes.rs`), browser navigation and the breadcrumb
(`test_web_routes.rs`), the character and combat routes (`test_game_routes.rs`),
the menu (`test_menu.rs`), the app factory and entry point (`test_app.rs`), what
the log writes and what it must never write (`test_core_logs.rs`), the request
trace (`test_request_trace.rs`), what a dead database costs every route
(`test_errors.rs`), and the import pipeline (`test_parse_pages.rs`).

Nothing in the suite is left to chance: `core::dice::Rng` is the one seam chance
comes through, `create_app` threads one through the whole application, and a rule
is asserted against dice chosen for it rather than against a lucky seed.

### Coverage

`make coverage` runs the suite once under instrumentation with
[cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov), writes the HTML
report to `target/llvm-cov/html/` (not versioned) and prints the per-file table
with the uncovered lines named. Installed once:

```bash
brew install cargo-llvm-cov            # or: cargo install cargo-llvm-cov --locked
rustup component add llvm-tools-preview
```

The gate is `MIN`, defaulting to 95, and it is on **lines**: `--branch` needs a
nightly toolchain and measures nothing on stable, so the table prints regions —
the finer count — beside the lines, and the gate reads the lines.

**The suite does not clear the gate yet.** All 654 tests pass under
instrumentation and the crate sits at **89.98% of lines, 88.45% of regions**, so
`make coverage` fails on `MIN` until the gaps are closed or it is lowered. Two
fifths of the shortfall is in three files the application never serves:
`pipeline/parse_pages.rs` (66.87%), `bin/parse_pages.rs` (0%) and `bin/serve.rs`
(25%). Without them the crate is at **93.38% of lines, 91.88% of regions**.

---

## Design notes

The decisions that are not obvious from the code, and what the tests hold them
to.

**Chance is a trait.** `core::dice::Rng` is what every rule that needs a die
takes; `SeededRng` makes a run reproducible and the harness's `FixedDice` hands a
rule the faces chosen for it. Dice are consumed in a fixed order, so a test names
the faces rather than a seed — no rule reaches for a generator of its own.

**The database a handler holds is a value.** `Db` is an enum rather than a global:
`Environment` opens the file `APP_ENV` picks, once, into a process-wide
connection; `Bound` is a connection handed over ready-made, which is how a test
gives the routes an in-memory database; `Unavailable` and `Failing` are the two
shapes of "the database cannot answer". Every `db::*` function starts by asking
the handle for its connection, which is the one place a failure can come from.

**A dict needs a value type richer than JSON.** `to_dict()` gives an instant back
as a `DateTime` when it is a column and as ISO 8601 text when it is in the blob,
because that is what a stored blob holds. So `Field` is JSON plus `Instant` and
`Date`, `RowDict` is an `IndexMap` of them, and `serde_json` is built with
`preserve_order` so a blob's keys survive a round trip in the order they were
written.

**Timestamps are stamped in the writer.** `created_at` on insert, `updated_at` on
every write that changes something, and no write at all for a row whose columns
and blob are unchanged — all three in `db/store.rs`, never in a caller.
`updated_at` counts as a column for that comparison, which is what lets a test
set one explicitly.

**The escaper is the crate's own.** The pages write `&#39;` for an apostrophe and
`&#34;` for a quote where Askama's escaper writes `&#x27;` and `&quot;`. The book
is French and full of apostrophes, so the difference is on nearly every page and
in nearly every assertion; `src/escape.rs` is wired in through `askama.toml`.

**A refusal travels on the response.** The middleware is what knows the method,
the path and the endpoint, so a handler that gives up returns an `AppError`, the
refusal carries its kind on the response, and `request_trace` is what writes the
`The database could not answer, refusing the request` line. The answer itself is
503, JSON under `/data/{number}` or for a caller that puts JSON first, the page
otherwise.

**The pipeline reads the document rather than rewriting it.** `scraper`/`ego-tree`
is a read-only DOM, so the tree is walked once and the link markers are written
into the text as it is collected — the text comes out the same and the document
is left alone. It sits in the library rather than in the binary, so it can be
tested: `src/bin/parse_pages.rs` is a thin command line over it.

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

**2. Parse.** `src/pipeline/parse_pages.rs` treats every sub-directory of
`work/raw_data/` as one book and writes `work/parsed_data/<book>.json`. It never
reads the YAML index nor the network: the page number comes from the page's
`h2`, the text from the `div.ob-text` blocks (one entry per paragraph, `<br>`
or `<pre>` line), and each link is resolved first against the `og:url` of the
local pages, then by the number in its URL, then by matching text slugs.

```bash
make import                                  # the defaults below
cargo run --bin parse_pages -- -i work/raw_data -o work/parsed_data
```

`-n` inlines `<link text> <number>` and `-v` logs per-file details. Output:
`{ "<page>": { "text": [...], "numbers": [...], "file_path": "..." } }`, plus a
per-book report (failed files, pages without numbers, remapped links).

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
`books/<series>/<book>/pages.json` and restart the server.

---

## Conventions

`AGENTS.md` is the authority; the essentials:

- Four-space indentation, `snake_case` modules and functions, `PascalCase` types,
  uppercase constants. `cargo clippy --all-targets` stays clean.
- Google-style doc comments on every public item — `# Arguments`, `# Returns`,
  `# Errors` where they say something the signature does not; comments only for
  genuinely non-obvious logic.
- One public type per snake_case file, models under `core/models/`, a module
  split as soon as it starts doing two things.
- The database layer stays in `core::db` and the web layer stays out of it: a new
  query goes there, never in a handler, and no row ever leaves it. A new table
  follows the hybrid pattern: real columns only for what is queried or
  constrained, one JSON `data` blob for the rest. A schema change comes with the
  next script in `migrations/` and a raised `SCHEMA_VERSION`.
- Code, identifiers and comments in English even when the discussion is in
  French; user-facing French text stays UTF-8.
- No `unwrap()` or `expect()` on something a caller can get wrong: a failure that
  is expected is a `Result`, and one that cannot happen says why it cannot.
- Commit subjects are short sentences (`Page parsing reworked.`); no `wip`
  subjects in review-ready work.

Never commit `.env`, caches or logs.

---

## License

MIT — see `LICENSE`.
