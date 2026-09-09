import json

from flask import Blueprint, Response, jsonify

from haute_tension.application.models.story_page import StoryData
from haute_tension.core.db import record_page_view


def create_api_blueprint(story_data: StoryData, book: str) -> Blueprint:
    """Create the blueprint serving story data.

    Args:
        story_data: Story sections indexed by page number.
        book: The book being served, as `"<series>/<book>"`.

    Returns:
        A configured Flask blueprint.
    """
    blueprint = Blueprint("api", __name__)

    @blueprint.get("/data/<int:number>")
    def get_numbers(number: int) -> Response:
        """Return one story page with its choices and actions."""
        number_string = str(number)
        record_page_view(book, number_string)
        if number_string not in story_data:
            return _page_not_found_response(number)

        return jsonify(story_data[number_string])

    return blueprint


def _page_not_found_response(number: int) -> Response:
    """Build the UTF-8 response for an unknown page number.

    Args:
        number: Unknown story page number.

    Returns:
        A JSON response describing the missing page.
    """
    response_data = {
        "success": False,
        "message": f"le numéro {number} n'a pas été trouvé",
    }
    return Response(
        json.dumps(response_data, ensure_ascii=False, indent=2),
        content_type="application/json; charset=utf-8",
    )
