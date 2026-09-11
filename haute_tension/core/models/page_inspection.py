"""A page a reader flagged as having a problem, and what was said about it.

One document per page and per book, whatever the number of reports: flagging a
page that is already flagged appends to its comments rather than opening a second
file on it. `core.db.flag_page` is the find-or-create; the unique index on
`(book, path)` is what keeps two reports racing each other from making two.

The id is a `uuid4().hex` like a game's, because it travels in a URL.
"""

from mongoengine import (
    DateTimeField,
    Document,
    EmbeddedDocument,
    EmbeddedDocumentListField,
    StringField,
)

OPEN = "open"
RESOLVED = "resolved"
STATUSES = (OPEN, RESOLVED)


class InspectionComment(EmbeddedDocument):
    """One remark on a flagged page. No author: the application has no users."""

    meta = {"strict": False}

    text = StringField(required=True)
    created_at = DateTimeField(required=True)


class PageInspection(Document):
    """One page, flagged for inspection, with everything said about it."""

    meta = {
        "collection": "page_inspections",
        "indexes": [{"fields": ("book", "path"), "unique": True}, "status"],
        "strict": False,
    }

    id = StringField(primary_key=True)
    book = StringField(required=True)

    # The path of the flagged page, as the browser had it: "/book/22".
    path = StringField(required=True)
    # What its tab said at the time, for the list; the path is the key.
    page_title = StringField()

    comments = EmbeddedDocumentListField(InspectionComment)
    status = StringField(required=True, default=OPEN, choices=STATUSES)

    created_at = DateTimeField(required=True)
    updated_at = DateTimeField(required=True)

    def __str__(self) -> str:
        return f"{self.book} {self.path} ({self.status}, {len(self.comments)} comments)"
