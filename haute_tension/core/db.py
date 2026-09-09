"""The MongoDB database: the connection, and every read and write.

The only module here that talks to the database — `core.config` is pure and
`core.models` only describes the collection. Callers get plain values back:
nothing outside this module and `core/models/` ever holds a document object,
which is what keeps the Flask routes free of the ORM.

**Only the reading history is stored.** The book is static — 668 pages that
change only when the import pipeline is re-run — so `core.story` reads it off
disk into memory at startup, and it is deliberately not in here: putting it in a
database would buy nothing and would make a page unservable without a reachable
server.

The connection is opened on the first call rather than at import time, so
importing `core.db` never needs a reachable server — and a test can stand a fake
one in by replacing `connect_db()`.

The history is **scoped to a book**, named `"<series>/<book>"`: the functions
below take a `book`, and it is part of every query, so a second book never shows
up in the first one's history.
"""

import os
from datetime import datetime, timezone

from mongoengine import connect, disconnect
from mongoengine import get_db as mongoengine_db
from mongoengine.errors import MongoEngineException
from pymongo.errors import PyMongoError

from haute_tension.core.config import current_db_name
from haute_tension.core.models import PageView

# What every caller catches around a database call. Two families rather than
# one: pymongo raises when the server cannot be reached or refuses a command,
# mongoengine when a document does not fit its model. Both mean "the database
# did not do what was asked", and every caller reports them the same way.
DatabaseError = (PyMongoError, MongoEngineException)

# What a caller catches when it can do without the database entirely. Wider than
# DatabaseError by one case, and it is the case that matters most: a reader who
# has never configured a server at all gets EnvironmentError out of connect_db(),
# not a driver error, and would otherwise be unable to read a book that is sitting
# on disk. Anything that only *reads better* with a history catches this; anything
# that exists to write one catches DatabaseError and reports the failure.
HistoryUnavailable = DatabaseError + (EnvironmentError,)

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


def record_page_view(book: str, page: str) -> bool:
    """Note that a page was asked for, unless it is already the last one read.

    Asking for the page one is already on is a reload, not a move, and a trail
    reading `22 › 22 › 22` says less than one reading `22`. Only *consecutive*
    repeats are dropped: coming back to a page after going elsewhere is a loop in
    the story, which is worth seeing.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        page: The page number asked for, whether or not the book has it.

    Returns:
        Whether a visit was actually recorded.
    """
    connect_db()
    if _latest_page(book) == page:
        return False

    PageView(
        book=book, page=page, viewed_at=datetime.now(timezone.utc)
    ).save(force_insert=True)
    return True


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


def _latest_page(book: str) -> str | None:
    """Return the page most recently read, or `None` when nothing has been.

    Args:
        book: The book being read, as `"<series>/<book>"`.

    Returns:
        The last page number recorded for that book.
    """
    latest = PageView.objects(book=book).order_by("-viewed_at", "-id").first()
    return latest.page if latest else None


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
