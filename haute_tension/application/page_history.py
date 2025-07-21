import json
from pathlib import Path
from typing import cast

MAX_PAGE_HISTORY = 10


def get_oldest_page(history_path: Path) -> str | None:
    """Return the oldest retained page number.

    Args:
        history_path: Path to the page-history JSON file.

    Returns:
        The oldest page, `None` for an empty history, or `"1"` when no history
        file exists.
    """
    if not history_path.exists():
        return "1"

    last_pages = _load_page_history(history_path)
    return last_pages[0] if last_pages else None


def update_last_pages(current_page: str, history_path: Path) -> list[str]:
    """Append a page number and retain only the most recent entries.

    Args:
        current_page: Page number to append.
        history_path: Path to the page-history JSON file.

    Returns:
        The updated bounded page history.
    """
    last_pages = _load_page_history(history_path)
    last_pages.append(current_page)
    last_pages = last_pages[-MAX_PAGE_HISTORY:]
    history_path.write_text(json.dumps(last_pages), encoding="utf-8")
    return last_pages


def _load_page_history(history_path: Path) -> list[str]:
    """Load page history, returning an empty list when it does not exist.

    Args:
        history_path: Path to the page-history JSON file.

    Returns:
        Previously retained page numbers.

    Raises:
        ValueError: If the history file does not contain a list of strings.
        json.JSONDecodeError: If the history file does not contain valid JSON.
    """
    try:
        serialized_history = history_path.read_text(encoding="utf-8")
    except FileNotFoundError:
        return []

    history: object = json.loads(serialized_history)
    if not isinstance(history, list) or not all(
        isinstance(page, str) for page in history
    ):
        raise ValueError("Page history must be a list of strings.")
    return cast(list[str], history)
