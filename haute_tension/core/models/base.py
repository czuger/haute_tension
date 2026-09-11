"""The declarative base every table is mapped on.

One `MetaData` for the whole application, so `core.db.connect_db()` can create
every table with a single `create_all()`. Nothing else lives here: the hybrid
pattern the models share is `HybridDocument`, in its own module.
"""

from sqlalchemy.orm import DeclarativeBase


class Base(DeclarativeBase):
    """The registry the models are mapped on."""
