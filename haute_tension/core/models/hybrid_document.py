"""What every table shares: a few real columns, and a JSON blob for the rest.

The schema is hybrid on purpose. A field is a column of its own only when a
query filters, sorts or joins on it — the id, the book, a timestamp things are
ordered by. Everything else is bundled into one `data` column holding a JSON
object, so that a new field on a hero or a fight is a key in that object and
not a migration.

The mixin is the whole contract:

- `to_dict()` is the row as one flat dict, the columns and the blob merged.
- `from_dict()` is the reverse: the keys that name a column become that column,
  the others are packed into `data`.
- `update_from_dict()` does the same onto an existing row, which is how
  `core.db` writes a loaded and changed state back.
- `to_json()` is `to_dict()` as text, for anything that answers JSON.

The blob is written with `json_default`, so an instant put into it comes back
as ISO 8601 text rather than as a `datetime`; only a real column keeps the type.
"""

import json
from datetime import date, datetime
from typing import Self

from sqlalchemy import Text
from sqlalchemy.orm import Mapped, mapped_column

DATA_COLUMN = "data"


def json_default(value: object) -> str:
    """Write what `json` does not know how to: an instant, as ISO 8601 text.

    Args:
        value: Whatever `json.dumps` could not serialize.

    Returns:
        The text to write in its place.

    Raises:
        TypeError: For anything that is not a date or a datetime.
    """
    if isinstance(value, (datetime, date)):
        return value.isoformat()
    raise TypeError(f"{type(value).__name__} cannot be written as JSON")


class HybridDocument:
    """A row made of indexed columns plus one JSON object for everything else."""

    data: Mapped[str] = mapped_column(Text, nullable=False, default="{}")

    @classmethod
    def indexed_columns(cls) -> tuple[str, ...]:
        """The names of the real columns, `data` excepted."""
        return tuple(
            column.key for column in cls.__table__.columns if column.key != DATA_COLUMN
        )

    @classmethod
    def split(
        cls, values: dict[str, object]
    ) -> tuple[dict[str, object], dict[str, object]]:
        """Sort a flat dict into what goes in a column and what goes in the blob.

        Args:
            values: The row as one flat dict.

        Returns:
            The column values, then the rest.
        """
        columns = set(cls.indexed_columns())
        indexed = {key: value for key, value in values.items() if key in columns}
        rest = {key: value for key, value in values.items() if key not in columns}
        return indexed, rest

    @classmethod
    def from_dict(cls, values: dict[str, object]) -> Self:
        """Build a row from one flat dict.

        Args:
            values: The columns and the blob's keys, mixed.

        Returns:
            The row, not yet added to a session.
        """
        indexed, rest = cls.split(values)
        return cls(**indexed, data=_pack(rest))

    def update_from_dict(self, values: dict[str, object]) -> None:
        """Replace the whole row with one flat dict, blob included.

        Args:
            values: The columns and the blob's keys, mixed. A column left out
                keeps its value; the blob is replaced entirely.
        """
        indexed, rest = self.split(values)
        for key, value in indexed.items():
            setattr(self, key, value)
        self.data = _pack(rest)

    def to_dict(self) -> dict[str, object]:
        """The row as one flat dict: the columns first, then the blob's keys."""
        values: dict[str, object] = {
            key: getattr(self, key) for key in self.indexed_columns()
        }
        values.update(json.loads(self.data or "{}"))
        return values

    def to_json(self) -> str:
        """The row as JSON text, instants written as ISO 8601."""
        return json.dumps(self.to_dict(), default=json_default, ensure_ascii=False)


def _pack(values: dict[str, object]) -> str:
    """The blob as the column stores it."""
    return json.dumps(values, default=json_default, ensure_ascii=False)
