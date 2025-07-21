from flask import Blueprint, abort, render_template

from haute_tension.application.models.story_page import StoryData

OPENING_PAGE = 1


def create_web_blueprint(
    story_data: StoryData,
    book_series: str,
    book_title: str,
) -> Blueprint:
    """Create the browser routes for the book landing page and reader.

    Args:
        story_data: Story pages indexed by page number.
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
        """Show one story page and links to its possible destinations."""
        page = story_data.get(str(number))
        if page is None:
            abort(404)
        return render_template(
            "page.html",
            book_series=book_series,
            book_title=book_title,
            page=page,
        )

    return blueprint
