"""The MongoDB database: the connection, and every read and write.

The only module here that talks to the database — `core.config` is pure and
`core.models` only describes the collections. Callers get plain dicts back
(`StoryData`, `StoryPage`): nothing outside this module and `core/models/` ever
holds a document object, which is what keeps the Flask routes and the import
script free of the ORM.

The connection is opened on the first call rather than at import time, so
importing `core.db` never needs a reachable server — and a test can stand a fake
one in by replacing `connect_db()`.

Everything is **scoped to a book**, named `"<series>/<book>"`: the functions
below take a `book`, and it is part of every query, so a second imported book
never shows up in the first one's pages or history.
"""

import os
from datetime import datetime, timezone
from typing import cast

from mongoengine import connect, disconnect
from mongoengine import get_db as mongoengine_db
from mongoengine.errors import MongoEngineException
from pymongo.errors import PyMongoError

from haute_tension.application.models.story_page import (
    StoryData,
    StoryPage as StoryPageDict,
)
from haute_tension.core.config import current_db_name
from haute_tension.core.models import PageView, StoryPage

# What every caller catches around a database call. Two families rather than
# one: pymongo raises when the server cannot be reached or refuses a command,
# mongoengine when a document does not fit its model. Both mean "the database
# did not do what was asked", and every caller reports them the same way.
DatabaseError = (PyMongoError, MongoEngineException)

# Long enough for a local mongod, short enough that a dead server shows up as an
# error rather than as a hung request.
SERVER_SELECTION_TIMEOUT_MS = 3000

# How many pages of reading history are kept. What MAX_PAGE_HISTORY used to bound
# in last_pages.json, now applied when the history is read.
MAX_PAGE_HISTORY = 10

# The database the current connection was opened on, so a run that switches
# APP_ENV mid-flight (the tests do) reconnects instead of reading the wrong one.
_connected_to: str | None = None


def connect_db() -> None:
    """Register the connection the models use, once per database name.

    Every function below calls this first, which is also the single seam the
    tests replace: with it stubbed out, nothing here ever reaches for a real
    server.

    Raises:
        EnvironmentError: If MONGO_URI is unset, or names a database other than
            the one APP_ENV asks for.
    """
    global _connected_to
    db_name = current_db_name()
    if _connected_to == db_name:
        return

    mongo_uri = os.environ.get("MONGO_URI")
    if not mongo_uri:
        raise EnvironmentError(
            "Missing MONGO_URI environment variable. "
            "Copy .env.example to .env and fill it in."
        )

    if _connected_to is not None:
        disconnect()
    connect(
        db=db_name,
        host=mongo_uri,
        serverSelectionTimeoutMS=SERVER_SELECTION_TIMEOUT_MS,
        # Explicit only to keep pymongo from warning about its legacy default;
        # no UUID is ever stored here.
        uuidRepresentation="standard",
    )

    # A database named in MONGO_URI ("…:27017/somewhere") wins over `db` above,
    # and would quietly take a dev run onto another database. APP_ENV is what
    # picks it.
    connected = mongoengine_db().name
    if connected != db_name:
        disconnect()
        raise EnvironmentError(
            f"MONGO_URI points at the database '{connected}', but APP_ENV asks "
            f"for '{db_name}'. Leave the database out of MONGO_URI."
        )

    _connected_to = db_name


def reset_connection() -> None:
    """Close the open connection, so the next call reconnects.

    Only the tests need this: a process normally works on one database for its
    whole life. The connection is dropped and not merely forgotten, because
    mongoengine registers it under a fixed alias and refuses to open a second
    one under the same name.
    """
    global _connected_to
    if _connected_to is not None:
        disconnect()
    _connected_to = None


def load_story(book: str) -> StoryData:
    """Load and index every page of one book.

    Args:
        book: The book to read, as `"<series>/<book>"`.

    Returns:
        Story pages indexed by their page number, empty when the book has never
        been imported.
    """
    connect_db()
    return {
        page.page: cast(StoryPageDict, page.to_dict())
        for page in StoryPage.objects(book=book)
    }


def save_story(book: str, pages: list[dict[str, object]]) -> int:
    """Replace one book's pages with the ones given, and say how many were written.

    The book is rewritten wholesale rather than merged: an import is the whole
    book as the pipeline last produced it, and a page dropped between two runs
    should not survive in the database.

    Args:
        book: The book to write, as `"<series>/<book>"`.
        pages: The pages, each keyed as `merged_pages.json` stores them.

    Returns:
        The number of pages written.

    Raises:
        ValueError: If a page has no string page number, or two share one.
    """
    documents = [_page_document(book, page) for page in pages]
    _refuse_duplicate_pages(documents)

    connect_db()
    StoryPage.objects(book=book).delete()
    if documents:
        StoryPage.objects.insert(documents, load_bulk=False)
    return len(documents)


def record_page_view(book: str, page: str) -> None:
    """Note that a page was asked for.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        page: The page number asked for, whether or not the book has it.
    """
    connect_db()
    PageView(
        book=book, page=page, viewed_at=datetime.now(timezone.utc)
    ).save(force_insert=True)


def last_pages(book: str, limit: int = MAX_PAGE_HISTORY) -> list[str]:
    """Return the pages most recently asked for, oldest first.

    Bounding happens here rather than on write, so recording a visit stays a
    plain insert. The result reads like the old `last_pages.json` did.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        limit: How many pages to keep at most.

    Returns:
        Up to `limit` page numbers, oldest first.
    """
    connect_db()
    views = PageView.objects(book=book).order_by("-viewed_at", "-id").limit(limit)
    return [view.page for view in views][::-1]


def get_oldest_page(book: str) -> str | None:
    """Return the oldest retained page number.

    Args:
        book: The book being read, as `"<series>/<book>"`.

    Returns:
        The oldest page still in the bounded history, or `None` when nothing has
        been read yet.
    """
    history = last_pages(book)
    return history[0] if history else None


def _page_document(book: str, page: dict[str, object]) -> StoryPage:
    """Build one page document, refusing anything that is not a page.

    Args:
        book: The book the page belongs to.
        page: One page as `merged_pages.json` stores it.

    Returns:
        The unsaved page document.

    Raises:
        ValueError: If the page is not an object with a string page number.
    """
    if not isinstance(page, dict):
        raise ValueError("Each merged book page must be an object.")
    if not isinstance(page.get("page"), str):
        raise ValueError("Each merged book page must have a string page number.")

    document = StoryPage.from_dict(page)
    document.book = book
    return document


def _refuse_duplicate_pages(documents: list[StoryPage]) -> None:
    """Refuse an import that would silently drop a page.

    Two pages sharing a number is a bug in the import pipeline, not something to
    resolve by keeping the last one: they share a primary key, so the insert
    would write one row where the book has two.

    Args:
        documents: The pages about to be written.

    Raises:
        ValueError: If two pages share a page number.
    """
    seen: set[str] = set()
    for document in documents:
        if document.page in seen:
            raise ValueError(f"Duplicate page number: {document.page}")
        seen.add(document.page)
