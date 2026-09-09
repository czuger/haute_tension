"""Load a parsed book into the database.

    python scripts/import_book.py [<series>/<book>]

The equivalent of the fetch scripts: `merged_pages.json` is the pipeline's
output, this is what puts it where the app reads it. The book is rewritten
wholesale, so re-running it is how a re-parsed book reaches the app.
"""

import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from haute_tension.core.config import ROOT, current_db_name, load_env
from haute_tension.core.db import save_story

REQUIRED_ENV_VARS = ["MONGO_URI"]
BOOKS_PATH = ROOT / "haute_tension" / "books"
DEFAULT_BOOK = "pretre_jean/forteresse_alamuth"


def read_pages(book: str) -> list[dict[str, object]]:
    """Read one book's parsed pages off disk.

    Args:
        book: The book to read, as `"<series>/<book>"`.

    Returns:
        The pages, as `merged_pages.json` stores them.

    Raises:
        SystemExit: If the book has no `merged_pages.json`, or it is not a list.
        json.JSONDecodeError: If the file does not contain valid JSON.
    """
    pages_path = BOOKS_PATH / book / "merged_pages.json"
    if not pages_path.exists():
        raise SystemExit(f"{pages_path} not found.")

    pages = json.loads(pages_path.read_text(encoding="utf-8"))
    if not isinstance(pages, list):
        raise SystemExit(f"{pages_path} must hold a list of pages.")
    return pages


def main(argv: list[str] | None = None) -> None:
    """Import one book, and say what was written.

    Args:
        argv: Command-line arguments; the book name at most. Defaults to
            `sys.argv`.
    """
    argv = sys.argv[1:] if argv is None else argv
    load_env(REQUIRED_ENV_VARS)

    book = (argv[0] if argv else DEFAULT_BOOK).strip("/")
    written = save_story(book, read_pages(book))
    print(f"Imported {written} pages of '{book}' into {current_db_name()}")


if __name__ == "__main__":
    main()
