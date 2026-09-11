"""What every table shares: a few real columns, and a JSON blob for the rest.

The schema is hybrid on purpose. A field is a column of its own when a query
filters, sorts or joins on it, or when the database has a rule to keep about it
— the id, the book, a timestamp, the hero's characteristics that may never go
below zero. Everything else is bundled into one `data` column holding a JSON
object, so that a new field on a fight is a key in that object.

A row's children — the lines of a game's bag — are rows of a table of their
own, but they travel inside the parent's dict, as a list of their dicts under
the relationship's name. The mixin is the whole contract:

- `to_dict()` is the row as one flat dict: its columns, its children, its blob.
- `from_dict()` is the reverse: keys naming a column become that column, keys
  naming a relationship become child rows, the others are packed into `data`.
- `update_from_dict()` does the same onto an existing row, which is how
  `core.db` writes a loaded and changed state back. A child dict carrying an
  `id` updates that child, one without is a new child, and a child no dict
  names is dropped — which the relationship's `delete-orphan` cascade turns
  into a delete.
- `to_json()` is `to_dict()` as text, for anything that answers JSON.

The blob is normalised as it is set, so an instant put into it is ISO 8601 text
at once and not only after a round trip; only a real column keeps the type.
"""

import copy
import json
from datetime import date, datetime
from typing import Self

from sqlalchemy import JSON, inspect
from sqlalchemy.orm import Mapped, mapped_column

DATA_COLUMN = "data"

# One row as a flat dict, the shape `to_dict()` gives and `from_dict()` takes.
RowDict = dict[str, object]


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
    """A row made of indexed columns, child rows, and one JSON object for the rest."""

    data: Mapped[RowDict] = mapped_column(JSON, nullable=False, default=dict)

    @classmethod
    def indexed_columns(cls) -> tuple[str, ...]:
        """The names of the real columns, `data` excepted."""
        return tuple(
            column.key for column in cls.__table__.columns if column.key != DATA_COLUMN
        )

    @classmethod
    def child_tables(cls) -> dict[str, type["HybridDocument"]]:
        """The one-to-many relationships whose rows travel inside this row's dict.

        Returns:
            Each relationship's name, and the model its rows are.
        """
        return {
            relationship.key: relationship.mapper.class_
            for relationship in inspect(cls).relationships
            if relationship.uselist
        }

    @classmethod
    def split(cls, values: RowDict) -> tuple[RowDict, dict[str, list[RowDict]], RowDict]:
        """Sort a flat dict into columns, children and the blob.

        Args:
            values: The row as one flat dict.

        Returns:
            The column values, the child dicts by relationship, then the rest.
        """
        columns = set(cls.indexed_columns())
        children = cls.child_tables()
        indexed = {key: value for key, value in values.items() if key in columns}
        nested = {key: value for key, value in values.items() if key in children}
        rest = {
            key: value
            for key, value in values.items()
            if key not in columns and key not in children
        }
        return indexed, nested, rest

    @classmethod
    def from_dict(cls, values: RowDict) -> Self:
        """Build a row, and its children, from one flat dict.

        Args:
            values: The columns, the child lists and the blob's keys, mixed.

        Returns:
            The row, not yet added to a session.
        """
        indexed, nested, rest = cls.split(values)
        children = cls.child_tables()
        rows = {
            key: [children[key].from_dict(entry) for entry in entries]
            for key, entries in nested.items()
        }
        return cls(**indexed, **rows, data=_normalised(rest))

    def update_from_dict(self, values: RowDict) -> None:
        """Replace the whole row with one flat dict, children and blob included.

        Args:
            values: The columns, the child lists and the blob's keys, mixed. A
                column or a relationship left out keeps what it has; the blob is
                replaced entirely.
        """
        indexed, nested, rest = self.split(values)
        for key, value in indexed.items():
            setattr(self, key, value)
        for key, entries in nested.items():
            setattr(self, key, self._synced_children(key, entries))
        self.data = _normalised(rest)

    def _synced_children(
        self, key: str, entries: list[RowDict]
    ) -> list["HybridDocument"]:
        """The child rows a list of child dicts describes, reusing those it names.

        Args:
            key: The relationship.
            entries: One dict per child, in order.

        Returns:
            The rows: the existing child an entry's `id` names, updated, or a new
            one for an entry without. A child no entry names is left out.
        """
        existing = {
            child.id: child for child in getattr(self, key) if child.id is not None
        }
        model = self.child_tables()[key]
        rows = []
        for entry in entries:
            child = existing.get(entry.get("id"))
            if child is None:
                child = model.from_dict(entry)
            else:
                child.update_from_dict(entry)
            rows.append(child)
        return rows

    def to_dict(self) -> RowDict:
        """The row as one flat dict: its columns, its children, then the blob's keys."""
        values: RowDict = {key: getattr(self, key) for key in self.indexed_columns()}
        for key in self.child_tables():
            values[key] = [child.to_dict() for child in getattr(self, key)]
        values.update(copy.deepcopy(self.data or {}))
        return values

    def to_json(self) -> str:
        """The row as JSON text, children included, instants written as ISO 8601."""
        return json.dumps(self.to_dict(), default=json_default, ensure_ascii=False)


def _normalised(values: RowDict) -> RowDict:
    """The blob as it reads back: plain JSON values, instants as text, a copy."""
    return json.loads(json.dumps(values, default=json_default))
