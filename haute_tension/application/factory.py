import random
from pathlib import Path

from flask import Flask

from haute_tension.application.game_routes import create_game_blueprint
from haute_tension.application.logs.request_trace import wire_the_request_trace
from haute_tension.application.routes import create_api_blueprint
from haute_tension.application.web_routes import create_web_blueprint
from haute_tension.core.config import ROOT, session_secret
from haute_tension.core.logs.general_log import event
from haute_tension.core.story import load_story

BOOKS_PATH = ROOT / "haute_tension" / "books"
TEMPLATE_PATH = ROOT / "haute_tension" / "templates"
BOOK = "pretre_jean/forteresse_alamuth"
BOOK_SERIES = "Prêtre Jean"
BOOK_TITLE = "La Forteresse d'Alamuth"


def create_app(
    book: str = BOOK,
    books_path: Path = BOOKS_PATH,
    rng: random.Random | None = None,
) -> Flask:
    """Create and configure the Flask application.

    The book is read off disk once, at startup: it is static, small enough to sit
    in memory, and changes only when the import pipeline is re-run. The database
    holds the reading history and the play-throughs.

    Args:
        book: The book to serve, as `"<series>/<book>"`.
        books_path: Directory the books live under.
        rng: Source of chance for character creation and combat. Seed one to
            make a run reproducible; the default leaves it to `random`.

    Returns:
        The configured Flask application.

    Raises:
        FileNotFoundError: If the book has no `pages.json`.
        ValueError: If the book data is malformed or repeats a page.
    """
    app = Flask(__name__, template_folder=str(TEMPLATE_PATH))
    app.secret_key = session_secret()
    wire_the_request_trace(app)
    story_data = load_story(books_path / book)
    app.register_blueprint(
        create_web_blueprint(story_data, book, BOOK_SERIES, BOOK_TITLE)
    )
    app.register_blueprint(create_api_blueprint(story_data, book))
    app.register_blueprint(
        create_game_blueprint(story_data, book, BOOK_SERIES, BOOK_TITLE, rng)
    )
    event(
        "Application built",
        book=book,
        pages=len(story_data),
        fights=sum(1 for page in story_data.values() if page.get("fight")),
    )
    return app
