import json
from pathlib import Path
from typing import cast

from haute_tension.application.models.story_page import StoryData, StoryPage


def load_story(book_path: Path) -> StoryData:
    """Load and index all numbered pages from a book directory.

    Args:
        book_path: Directory containing the book's `merged_pages.json` file.

    Returns:
        Story pages indexed by their page number.

    Raises:
        FileNotFoundError: If the merged-pages file does not exist.
        json.JSONDecodeError: If the merged-pages file contains invalid JSON.
        ValueError: If the merged-pages data is malformed or repeats a page.
    """
    pages_path = book_path / "merged_pages.json"
    serialized_pages = pages_path.read_text(encoding="utf-8")
    pages: object = json.loads(serialized_pages)
    if not isinstance(pages, list):
        raise ValueError("Merged book pages must be a list.")

    story_data: StoryData = {}
    for page in pages:
        if not isinstance(page, dict):
            raise ValueError("Each merged book page must be an object.")
        page_number = page.get("page")
        if not isinstance(page_number, str):
            raise ValueError("Each merged book page must have a string page number.")
        if page_number in story_data:
            raise ValueError(f"Duplicate page number: {page_number}")
        story_data[page_number] = cast(StoryPage, page)
    return story_data
