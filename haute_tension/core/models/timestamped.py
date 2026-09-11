"""When a row was created, and when it last changed.

A mixin for the tables whose rows are written more than once — a game, the lines
of its bag, a flagged page. Both columns fill themselves in:

- on insert, `created_at` is the current instant and `updated_at` that same
  instant, so a row nobody has changed says so by the two being equal;
- on every update SQLAlchemy writes, `updated_at` moves to the current instant,
  unless the update sets it itself. An update that changes nothing is not
  written, and moves nothing.

Only a change to the row itself moves it: an item whose count changes moves its
own `updated_at`, not its game's.

The Python defaults are what the application uses. The server defaults write
the same text for a row inserted by hand, or by a migration.

The reading history has neither column: a visit is written once and never
changed, and its `viewed_at` already says when.
"""

from datetime import datetime

from sqlalchemy.engine.default import DefaultExecutionContext
from sqlalchemy.orm import Mapped, mapped_column

from haute_tension.core.models.utc_datetime import NOW_IN_SQL, UtcDateTime, utc_now


def same_instant_as_created_at(context: DefaultExecutionContext) -> datetime:
    """The `created_at` of the row being inserted, for its `updated_at`.

    Two calls to `utc_now` would be a few microseconds apart.

    Args:
        context: The insert being executed. `created_at` is already among its
            parameters, given or defaulted, because it is declared first.

    Returns:
        That instant.
    """
    return context.get_current_parameters()["created_at"]


class Timestamped:
    """A `created_at` set on insert, and an `updated_at` moved on every update."""

    created_at: Mapped[datetime] = mapped_column(
        UtcDateTime, nullable=False, default=utc_now, server_default=NOW_IN_SQL
    )
    updated_at: Mapped[datetime] = mapped_column(
        UtcDateTime,
        nullable=False,
        default=same_instant_as_created_at,
        server_default=NOW_IN_SQL,
        onupdate=utc_now,
    )
