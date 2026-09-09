"""The reading history: which page was asked for, and when.

What `last_pages.json` used to hold, with one row per visit instead of one file
per reader. The history is bounded when it is read rather than when it is
written (see `core.db.last_pages`), so a visit is only ever an insert.

The id is Mongo's own ObjectId: a visit has no id of its own, nothing ever looks
one up by id, and `DictDocument` keeps an ObjectId out of the dicts it hands
over.
"""

from mongoengine import DateTimeField, Document, StringField

from .base import DictDocument


class PageView(Document, DictDocument):
    """One page, asked for once."""

    meta = {
        "collection": "page_views",
        # Read newest first, always for one book: the compound index is the
        # order the history is served in.
        "indexes": [("book", "-viewed_at")],
        "strict": False,
    }

    book = StringField(required=True)
    page = StringField(required=True)
    viewed_at = DateTimeField(required=True)

    def __str__(self) -> str:
        return f"{self.book} p.{self.page} @ {self.viewed_at}"
