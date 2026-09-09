# Haute Tension

[![Build Status](https://travis-ci.org/czuger/haute_tension.svg?branch=master)](https://travis-ci.org/czuger/haute_tension)
[![Code Climate](https://codeclimate.com/github/czuger/haute_tension/badges/gpa.svg)](https://codeclimate.com/github/czuger/haute_tension)
[![Test Coverage](https://codeclimate.com/github/czuger/haute_tension/badges/coverage.svg)](https://codeclimate.com/github/czuger/haute_tension/coverage)

Une mise en ligne du livre dont vous êtes le héros : *La Forteresse d'Alamuth*
(saga du Prêtre Jean).

The project has two halves:

1. **A Flask reader** (`haute_tension/`) that serves the book as a browsable
   website, plus a small JSON API.
2. **An offline import pipeline** (`work/`) that scrapes the original pages from
   the web, parses them into JSON, and enriches them (per-choice gains/losses,
   structured combat encounters) into the runtime book data.

---

## Repository layout

```
haute_tension/                 Flask application package
  app.py                       Development-server entry point (port 5001)
  application/
    factory.py                 create_app(): wires blueprints, paths, book title
    web_routes.py              Browser routes: "/" and "/book/<number>"
    routes.py                  API route: "/data/<number>"
    story.py                   load_story(): reads and indexes merged_pages.json
    page_history.py            Bounded on-disk history of recently read pages
    models/story_page.py       TypedDicts: StoryPage, StoryChoice, ElementChange
  templates/                   Jinja templates (base / index / page), CSS inline
  books/<series>/<book>/       Runtime book data (see "Book data" below)

work/                          Offline import & conversion pipeline
  download_raw_data.rb         Ruby crawler: fetches source HTML into raw_data/
  urls.txt                     Books to crawl (one per line, "#" disables)
  parse_pages.py               HTML -> JSON: page text + outgoing page numbers
  conversions_prompt_actions.txt   LLM prompt: extract choice gains/losses
  conversion_prompt_combat.txt     LLM prompt: detect and structure fights
  new_fable_prompt_for_processing_book.txt  Combined single-pass version
  raw_data/                    Downloaded HTML + per-book YAML index
  parsed_data/                 Generated intermediate JSON/YAML

tests/                         Test suite (pytest runner, Flask test client)
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

`application/factory.py` holds all the wiring and the default paths, resolved
from the repository root:

| Constant        | Value                                                   |
| --------------- | ------------------------------------------------------- |
| `BOOK_PATH`     | `haute_tension/books/pretre_jean/forteresse_alamuth`     |
| `HISTORY_PATH`  | `haute_tension/last_pages.json`                          |
| `TEMPLATE_PATH` | `haute_tension/templates`                                |
| `BOOK_SERIES`   | `Prêtre Jean`                                            |
| `BOOK_TITLE`    | `La Forteresse d'Alamuth`                                |

`create_app(book_path, history_path)` accepts overrides for both paths, which is
how the tests run against isolated fixture books. It loads the story once at
startup and registers two blueprints.

### Routes

| Method | Path              | Blueprint | Description                                                                                   |
| ------ | ----------------- | --------- | --------------------------------------------------------------------------------------------- |
| `GET`  | `/`               | `web`     | Landing page: series, title, and a link to the opening page (page 1).                          |
| `GET`  | `/book/<number>`  | `web`     | Reader: the page's choices first, then its text. Unknown page → HTTP 404. A page with no choices shows "Fin de l'aventure" and a restart link. |
| `GET`  | `/data/<number>`  | `api`     | The raw page object as JSON (text, choices, `fight` when present).                              |

`/data/<number>` also appends the requested page to the read history before
looking it up. An unknown page returns a UTF-8 JSON body
`{"success": false, "message": "le numéro N n'a pas été trouvé"}` — note that
this error still carries HTTP 200.

### Story loading

`application/story.py::load_story()` reads `merged_pages.json` from the book
directory, validates that it is a list of page objects each carrying a string
`page` number, and returns a `dict[str, StoryPage]` indexed by that number. It
raises `ValueError` on malformed data or duplicate page numbers, and lets
`FileNotFoundError` / `json.JSONDecodeError` propagate.

### Page history

`application/page_history.py` persists a list of recently requested page numbers
to `last_pages.json` (git-ignored):

- `update_last_pages(page, path)` appends and truncates to the last
  `MAX_PAGE_HISTORY = 10` entries.
- `get_oldest_page(path)` returns the oldest retained page, `"1"` when no file
  exists yet, and `None` when the file holds an empty list.

### Templates

`base.html` carries the layout and the whole stylesheet (serif body, parchment
palette, drop cap on the first paragraph, mobile breakpoint at 36rem).
`index.html` is the landing card; `page.html` renders the page number, the
choices block, then the story text.

---

## Book data

Runtime books live in `haute_tension/books/<series>/<book>/`:

- **`merged_pages.json`** — the file the application actually loads. A JSON list
  of 668 page objects for *La Forteresse d'Alamuth*.
- **`translated_elements.json`** — a flat map from English element keys to their
  French display names (`"healing potion": "potion de guérison"`, …), used to
  localize the gains/losses vocabulary produced by the import pipeline.
- **`pages.json`** — a later regeneration of the same book (68 fights detected
  instead of 50, 262 pages differing). It is *not* loaded by the app; treat it as
  a candidate for the next `merged_pages.json`.

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

Dependencies come from `pyproject.toml`: Flask, PyYAML, beautifulsoup4/bs4. Do
not add a `requirements.txt`. Add `[test]` to also install pytest and
pytest-cov:

```bash
python -m pip install -e ".[test]"
```

### 3. Run the server

```bash
cd haute_tension && PYTHONPATH=.. python app.py
```

`PYTHONPATH=..` puts the repository root on the import path so `haute_tension.*`
resolves.

Then open <http://localhost:5001/>.

### 4. Manual checks

```bash
curl http://localhost:5001/data/1
curl http://localhost:5001/data/99999      # unknown page, HTTP 200 + JSON error
```

---

## Tests

All tests live in the repository-root `tests/` directory. They are written with
the standard library's `unittest` and run under pytest, which is configured in
`pyproject.toml` to measure branch coverage of `haute_tension` and to fail below
95%:

```bash
python -m pytest
```

27 tests cover the data route and its history side effect (`test_routes.py`),
browser navigation and 404s (`test_web_routes.py`), the bounded page history and
its validation errors (`test_page_history.py`), book loading and every malformed
input it rejects (`test_story.py`), and the development-server entry point
(`test_app.py`). Each route test builds a throwaway book in a
`TemporaryDirectory` and passes it to `create_app()`. Current coverage: 100%.

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
`haute_tension/books/<series>/<book>/merged_pages.json` and restart the server.

---

## Conventions

`AGENTS.md` is the authority; the essentials:

- Four-space indentation, PEP 8 naming, fully typed functions including
  `-> None`; avoid `Any`.
- Google-style docstrings on every function and method; comments only for
  genuinely non-obvious logic.
- Absolute imports from the repository root
  (`from haute_tension.application.story import load_story`), `pathlib.Path` for
  paths, one public class per snake_case file, models under `models/`.
- Code, identifiers and comments in English even when the discussion is in
  French; user-facing French text stays UTF-8.
- No global `try`/`except` around entry points — let tracebacks surface.
- Commit subjects are short imperative sentences (`Page parsing reworked.`); no
  `wip` subjects in review-ready work.

Never commit `last_pages.json`, caches or logs.

---

## License

MIT — see `LICENSE`.
