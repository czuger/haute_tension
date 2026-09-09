"""One mongoengine document per collection — the shape of what is stored.

The models describe the collections and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands plain values back, so only this package and `core.db` ever see
a document object.

    page_views -> PageView, one row per page asked for

One collection, because there is only one thing worth storing: the book is
static and read into memory at startup, so nothing about it belongs here.
"""

from .page_view import PageView

__all__ = ["PageView"]
