"""One line of a hero's bag: a kind of thing, and how many of it he carries.

Its own table rather than a list inside the game's row. **One row per line of
the bag**, as the sheet shows it — "4 rations" is one row with a count of four —
and never one row for the whole bag.

Real columns: the id, the game the line belongs to, and when it was first
carried and last changed. The rest is the blob:

    element     the book's name for the thing: "sword", "ration", "key"
    label       what the reader is shown, in French: "épée"
    count       how many he carries; at zero the line leaves the bag, and the
                row is deleted

The rows are written through their game: `Game.items` owns them, in the order
they were first carried.
"""

from sqlalchemy import ForeignKey, Integer
from sqlalchemy.orm import Mapped, mapped_column

from haute_tension.core.models.base import Base
from haute_tension.core.models.hybrid_document import HybridDocument
from haute_tension.core.models.timestamped import Timestamped


class Item(Timestamped, HybridDocument, Base):
    """One line of a hero's bag."""

    __tablename__ = "items"
    __table_args__ = {"sqlite_autoincrement": True}

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    game_id: Mapped[int] = mapped_column(
        ForeignKey("games.id"), nullable=False, index=True
    )

    def __str__(self) -> str:
        line = self.data or {}
        return f"{line.get('label')} ×{line.get('count')}"
