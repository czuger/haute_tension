"""The book itself, read off disk.

The book is static — 668 pages that only change when the import pipeline is
re-run — so it is read once at startup and held in memory rather than stored.
Nothing here touches the database: `core.db` keeps the one thing that does
change, which is what has been read.
"""

import json
from pathlib import Path
from typing import cast

from haute_tension.application.models.story_page import StoryData, StoryPage

# The file a book directory is read from. The pipeline writes several JSON files
# there over a book's life; this is the one the application serves.
PAGES_FILE = "pages.json"


def load_story(book_path: Path) -> StoryData:
    """Load and index all numbered pages from a book directory.

    Args:
        book_path: Directory containing the book's `pages.json` file.

    Returns:
        Story pages indexed by their page number.

    Raises:
        FileNotFoundError: If the pages file does not exist.
        json.JSONDecodeError: If the pages file contains invalid JSON.
        ValueError: If the pages data is malformed or repeats a page.
    """
    pages_path = book_path / PAGES_FILE
    serialized_pages = pages_path.read_text(encoding="utf-8")
    pages: object = json.loads(serialized_pages)
    if not isinstance(pages, list):
        raise ValueError("Book pages must be a list.")

    story_data: StoryData = {}
    for page in pages:
        if not isinstance(page, dict):
            raise ValueError("Each book page must be an object.")
        page_number = page.get("page")
        if not isinstance(page_number, str):
            raise ValueError("Each book page must have a string page number.")
        if page_number in story_data:
            raise ValueError(f"Duplicate page number: {page_number}")
        story_data[page_number] = cast(StoryPage, page)
    return story_data
