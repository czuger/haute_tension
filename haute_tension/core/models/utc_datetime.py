"""A timestamp column that gives back exactly the instant it was given.

SQLite has no datetime type: SQLAlchemy's `DateTime` stores a naive string and
hands a naive `datetime` back, which would drop the timezone every instant in
this application carries. This column stores the ISO 8601 text of the instant in
UTC instead — `2026-09-10T12:00:00.000000+00:00` — which is both what it reads
back as an aware `datetime` and a text that sorts in chronological order, so an
`ORDER BY` on it is an order in time.
"""

from datetime import datetime, timezone

from sqlalchemy import String
from sqlalchemy.engine import Dialect
from sqlalchemy.types import TypeDecorator

# Wide enough for the fixed-width text `to_text` writes.
ISO_WIDTH = 32


class UtcDateTime(TypeDecorator[datetime]):
    """An aware `datetime`, stored as fixed-width ISO 8601 text in UTC."""

    impl = String(ISO_WIDTH)
    cache_ok = True

    def process_bind_param(
        self, value: datetime | None, dialect: Dialect
    ) -> str | None:
        """Write the instant as UTC text; a naive one is taken to be UTC."""
        return None if value is None else to_text(value)

    def process_result_value(
        self, value: str | None, dialect: Dialect
    ) -> datetime | None:
        """Read the text back as an aware `datetime`."""
        return None if value is None else datetime.fromisoformat(value)


def to_text(instant: datetime) -> str:
    """The fixed-width UTC text of an instant, the same for every row.

    Always with microseconds, so two instants compare as text the way they
    compare in time: `12:00:00` and `12:00:00.5` would otherwise differ at a
    character that is not a digit.

    Args:
        instant: The instant; a naive one is taken to be UTC.

    Returns:
        `YYYY-MM-DDTHH:MM:SS.ffffff+00:00`.
    """
    if instant.tzinfo is None:
        instant = instant.replace(tzinfo=timezone.utc)
    return instant.astimezone(timezone.utc).isoformat(timespec="microseconds")
