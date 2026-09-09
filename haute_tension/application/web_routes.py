from flask import Blueprint, abort, render_template

from haute_tension.core.logs.general_log import note
from haute_tension.application.models.story_page import StoryData
from haute_tension.core.db import HistoryUnavailable, last_pages, record_page_view

OPENING_PAGE = 1


def create_web_blueprint(
    story_data: StoryData,
    book: str,
    book_series: str,
    book_title: str,
) -> Blueprint:
    """Create the browser routes for the book landing page and reader.

    Args:
        story_data: Story pages indexed by page number.
        book: The book being read, as `"<series>/<book>"`.
        book_series: Display name of the book series.
        book_title: Display title of the current book.

    Returns:
        A configured Flask blueprint.
    """
    blueprint = Blueprint("web", __name__)

    @blueprint.get("/")
    def landing_page() -> str:
        """Show the current book and a link to begin reading."""
        return render_template(
            "index.html",
            book_series=book_series,
            book_title=book_title,
            opening_page=OPENING_PAGE,
        )

    @blueprint.get("/book/<int:number>")
    def read_page(number: int) -> str:
        """Show one story page, its destinations, and the trail that led here."""
        page = story_data.get(str(number))
        if page is None:
            abort(404)
        return render_template(
            "page.html",
            book_series=book_series,
            book_title=book_title,
            page=page,
            trail=_reading_trail(book, str(number)),
        )

    return blueprint


def _reading_trail(book: str, current_page: str) -> list[str]:
    """Record this visit and return the pages most recently read, oldest first.

    The trail is the one thing on the page that needs the database, and the book
    itself does not: a reader whose server is down — or who has never configured
    one, which is the wider half of `HistoryUnavailable` — should still be able
    to read, so an unavailable history costs the breadcrumb and nothing else.
    The `try` holds two database calls and nothing else, so this is not a blanket
    `except` around the route: a template or story failure still surfaces.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        current_page: The page number being shown, which ends the trail.

    Returns:
        Up to `MAX_PAGE_HISTORY` page numbers, oldest first, or nothing at all
        when the database cannot be reached.
    """
    try:
        record_page_view(book, current_page)
        return last_pages(book)
    except HistoryUnavailable as trouble:
        note(
            "Reading history unavailable, serving the page without a trail",
            page=current_page,
            reason=repr(trouble),
        )
        return []
