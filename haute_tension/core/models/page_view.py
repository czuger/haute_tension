"""The reading history: which page was asked for, and when.

What `last_pages.json` used to hold, with one row per visit instead of one file
per reader. The history is bounded when it is read rather than when it is
written (see `core.db.last_pages`), so a visit is only ever an insert.

The id is an integer the database counts up: a visit has no id of its own and
nothing ever looks one up by id. `book`, `game_id` and `viewed_at` are real
columns because every read filters on one of the first two and sorts on the
third; the page number itself is only ever read back, so it sits in the blob:

    page    the page number asked for, whether or not the book has it

A visit is written once and never changed, so it carries no `created_at` or
`updated_at`: `viewed_at` is when it was written.
"""

from datetime import datetime

from sqlalchemy import ForeignKey, Index, Integer, String
from sqlalchemy.orm import Mapped, mapped_column

from haute_tension.core.models.base import Base
from haute_tension.core.models.hybrid_document import HybridDocument
from haute_tension.core.models.utc_datetime import UtcDateTime


class PageView(HybridDocument, Base):
    """One page, asked for once."""

    __tablename__ = "page_views"
    __table_args__ = (
        # Read newest first, always for one book or one game: the compound
        # indexes are the orders the history is served in.
        Index("ix_page_views_book_viewed_at", "book", "viewed_at"),
        Index("ix_page_views_game_id_viewed_at", "game_id", "viewed_at"),
        {"sqlite_autoincrement": True},
    )

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    book: Mapped[str] = mapped_column(String, nullable=False)

    # The play-through that asked for it, when there was one. A reader with no
    # hero still leaves a trail; only "where was I?" needs to know whose it is,
    # since two heroes of the same book each have their own last page.
    game_id: Mapped[int | None] = mapped_column(ForeignKey("games.id"))

    viewed_at: Mapped[datetime] = mapped_column(UtcDateTime, nullable=False)

    def __str__(self) -> str:
        return f"{self.book} p.{(self.data or {}).get('page')} @ {self.viewed_at}"
