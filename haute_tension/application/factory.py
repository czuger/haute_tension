from flask import Flask

from haute_tension.application.routes import create_api_blueprint
from haute_tension.application.web_routes import create_web_blueprint
from haute_tension.core.config import ROOT
from haute_tension.core.db import load_story

TEMPLATE_PATH = ROOT / "haute_tension" / "templates"
BOOK = "pretre_jean/forteresse_alamuth"
BOOK_SERIES = "Prêtre Jean"
BOOK_TITLE = "La Forteresse d'Alamuth"


def create_app(book: str = BOOK) -> Flask:
    """Create and configure the Flask application.

    The book is read from the database once, at startup: it only changes when
    `scripts/import_book.py` is run, and holding it in memory keeps every page
    view from going back to Mongo for data that has not moved.

    Args:
        book: The book to serve, as `"<series>/<book>"`.

    Returns:
        The configured Flask application.

    Raises:
        EnvironmentError: If the database is not configured.
    """
    app = Flask(__name__, template_folder=str(TEMPLATE_PATH))
    story_data = load_story(book)
    app.register_blueprint(
        create_web_blueprint(story_data, BOOK_SERIES, BOOK_TITLE)
    )
    app.register_blueprint(create_api_blueprint(story_data, book))
    return app
