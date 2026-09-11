"""A page a reader flagged as having a problem, and what was said about it.

One row per page and per book, whatever the number of reports: flagging a page
that is already flagged appends to its comments rather than opening a second
file on it. `core.db.flag_page` is the find-or-create; the unique constraint on
`(book, path)` is what keeps two reports racing each other from making two.

The id is a `uuid4().hex` like a game's, because it travels in a URL.

Real columns: the id, the `(book, path)` the file is found by, the `status` the
list filters on and the `updated_at` it is sorted by. In the blob:

    page_title      what the page's tab said at the time, for the list
    comments        [{text, created_at}], oldest first, dated as ISO 8601 text
    created_at      ISO 8601 text
"""

from datetime import datetime

from sqlalchemy import Index, String, UniqueConstraint
from sqlalchemy.orm import Mapped, mapped_column

from haute_tension.core.models.base import Base
from haute_tension.core.models.hybrid_document import HybridDocument
from haute_tension.core.models.utc_datetime import UtcDateTime

OPEN = "open"
RESOLVED = "resolved"
STATUSES = (OPEN, RESOLVED)


class PageInspection(HybridDocument, Base):
    """One page, flagged for inspection, with everything said about it."""

    __tablename__ = "page_inspections"
    __table_args__ = (
        UniqueConstraint("book", "path", name="uq_page_inspections_book_path"),
        # The list: one book's files, most recently commented first.
        Index("ix_page_inspections_book_updated_at", "book", "updated_at"),
    )

    id: Mapped[str] = mapped_column(String(32), primary_key=True)
    book: Mapped[str] = mapped_column(String, nullable=False)

    # The path of the flagged page, as the browser had it: "/book/22".
    path: Mapped[str] = mapped_column(String, nullable=False)

    status: Mapped[str] = mapped_column(
        String(16), nullable=False, default=OPEN, index=True
    )
    updated_at: Mapped[datetime] = mapped_column(UtcDateTime, nullable=False)

    def __str__(self) -> str:
        comments = self.to_dict().get("comments") or []
        return f"{self.book} {self.path} ({self.status}, {len(comments)} comments)"
