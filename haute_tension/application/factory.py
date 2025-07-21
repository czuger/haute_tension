from pathlib import Path

from flask import Flask

from haute_tension.application.routes import create_api_blueprint
from haute_tension.application.story import load_story
from haute_tension.application.web_routes import create_web_blueprint

PROJECT_ROOT = Path(__file__).resolve().parents[2]
BOOK_PATH = (
    PROJECT_ROOT
    / "haute_tension"
    / "books"
    / "pretre_jean"
    / "forteresse_alamuth"
)
HISTORY_PATH = PROJECT_ROOT / "haute_tension" / "last_pages.json"
TEMPLATE_PATH = PROJECT_ROOT / "haute_tension" / "templates"
BOOK_SERIES = "Prêtre Jean"
BOOK_TITLE = "La Forteresse d'Alamuth"


def create_app(
    book_path: Path = BOOK_PATH,
    history_path: Path = HISTORY_PATH,
) -> Flask:
    """Create and configure the Flask application.

    Args:
        book_path: Path to the directory containing the runtime book data.
        history_path: Path used to persist recently requested audio pages.

    Returns:
        The configured Flask application.
    """
    app = Flask(__name__, template_folder=str(TEMPLATE_PATH))
    story_data = load_story(book_path)
    app.register_blueprint(
        create_web_blueprint(story_data, BOOK_SERIES, BOOK_TITLE)
    )
    app.register_blueprint(create_api_blueprint(story_data, history_path))
    return app
