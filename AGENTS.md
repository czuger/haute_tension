# Repository Guidelines

## Project Structure & Module Organization

The application is a Flask service in `haute_tension/`. `app.py` is the development-server entry point, `application/` contains the app factory and the routes, and `templates/` contains the browser UI.

`haute_tension/core/` is everything below the web layer, and nothing in it knows about Flask. It is strictly layered and the imports only go one way: `config.py` (environment variables and which database), `story.py` (the book, read off disk), `models/` (one mongoengine document per collection, describing shapes and running no query), then `db.py` (the connection and every read and write). `db.py` is the only module that talks to MongoDB, and no document object ever reaches a route or a template. Keep it that way: a new query belongs in `db.py`, never in a route.

**Only the reading history is stored.** The book is static, fits in memory, and is read off disk at startup by `core/story.py`; putting it in the database would buy nothing and would cost the ability to serve a page without a reachable server. Do not move it there.

That ability is load-bearing, so keep it: anything on a reader page that needs Mongo must degrade rather than fail. Catch `db.HistoryUnavailable` (the driver errors plus the `EnvironmentError` raised when no server is configured at all) around the database calls themselves, never around a whole route.

The connection is opened lazily by `db.connect_db()`, which is the single seam the tests replace. `APP_ENV` (`dev` / `prod`) suffixes the database name, so a dev run never touches prod data.

Runtime books live under `haute_tension/books/<series>/<book>/`; `pages.json` is the file the application loads, named by `core.story.PAGES_FILE`.

Story import and conversion files live in `work/`: source YAML/HTML is under `work/raw_data/`, and generated output belongs in `work/parsed_data/`.

## Build, Test, and Development Commands

- `pyenv virtualenv <python-version> haute_tension && pyenv local haute_tension` creates and selects the named virtualenv used by the repository's `.python-version` file.
- `python -m pip install -e ".[test]"` installs the project, its dependencies and the test extras declared in `pyproject.toml`.
- `make help` lists the repository's targets; `make test` is how the suite is run before pushing.
- Copy `.env.example` to `.env` and point `MONGO_URI` at a reachable mongod; `APP_ENV` picks the database. Leave the database name out of `MONGO_URI` — `db.connect_db()` refuses a URI that names one. Only the reading history needs the server; browsing works without one.
- `cd haute_tension && PYTHONPATH=.. python app.py` starts the development server on port 5001.

Declare Python dependencies and package metadata in `pyproject.toml`; do not recreate `requirements.txt` files.

## Coding Style & Naming Conventions

- Use four-space indentation and PEP 8 naming for Python: `snake_case` functions and modules, `PascalCase` classes, and uppercase constants.
- Fully type every function and method, including explicit `-> None` return types. Prefer precise types such as `Optional[...]` and `list[str]`; avoid `Any`, and justify unavoidable uses with a short comment.
- Keep functions to a soft limit of 50 lines. Split larger functions into well-named, single-responsibility helpers, and prefer composition of small functions over nested logic.
- Keep route handlers small. Split modules by topic when they take on too many responsibilities, introduce subpackages as a topic grows, and keep `__init__.py` files thin.
- Define one public class per snake_case-named file. Plain-container dataclasses and small, tightly coupled supporting types such as result objects, enums, or configuration classes may share the main class's file.
- Put each model in its own file under a `models/` directory. Import models from their defining modules rather than re-exporting them from `models/__init__.py`, especially when importing one may load an optional dependency.
- Use absolute imports from the repository root, such as `from haute_tension.application.story import load_story`; do not use relative imports or bare package-module imports such as `import app`.
- Use `pathlib.Path` for new file paths.
- Give every command-line parameter a unique one-letter short option alongside its long form.
- Do not add a global `try`/`except` around Python entry points; let unhandled exceptions fail with their full traceback.
- Even when prompts or discussions are in French, keep code, identifiers, and code comments in English. Preserve UTF-8 French user-facing text.
- Favor readability, explicit behavior, and small testable units over cleverness. No formatter or linter is configured, so make focused, style-consistent edits.

## Docstrings

Add concise Google-style docstrings to every function and method. Include `Args`, `Returns`, `Yields`, and `Raises` sections when applicable, but do not restate information already obvious from the signature. Add more detail only when it provides useful context.

## Comments

Reserve comments for genuinely complex, non-obvious, or easily misread logic. Keep them brief and focused on intent or reasoning; do not narrate straightforward code or caption the obvious.

## Testing Guidelines

Run the suite with `python -m pytest`. All tests must live in the repository-root `tests/` directory; do not create package-local or alternate test directories. Name test files `test_*.py` and prefer Flask's test client. Branch coverage of `haute_tension` and `scripts` is measured by `pytest-cov` and configured in `pyproject.toml` to fail below 95%, so keep new code covered.

The suite runs against two backends and no test may care which. `make test` brings up a real MongoDB in a container (port 27019, database `haute_tension_test`) and runs the suite against it; `make test-fast` and a bare `python -m pytest` use an in-memory mongomock server and need nothing brought up. `tests/conftest.py` binds the models to whichever applies and stubs `core.db.connect_db()` out either way; take the `fake_db`, `book_pages` or `client` fixture rather than standing up your own. Run `make test` before pushing: mongomock is a reimplementation, and a driver behaviour it does not share is a bug only the real server shows.

Never point `TEST_DB_NAME` at a database the application uses — the suite empties it before every test. Seed a collection by assigning to `fake_db["<collection>"].docs`, which is keyed the way the app reads it. Document manual checks for `/data/<number>` in the pull request.

## Commit & Pull Request Guidelines

History uses short sentence-style subjects such as `Page parsing reworked.` Use a specific imperative subject and avoid `wip` commits in review-ready work. Pull requests should describe user-visible behavior, list commands or manual checks performed, identify generated-data changes, link related issues, and include screenshots for template/UI updates.

## Security & Generated Files

Never commit real credentials or `.env`. Do not commit caches, logs, or OS/editor artifacts.
