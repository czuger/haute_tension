"""Fixtures every test file shares: the database, and an app on top of it.

Every test runs against an in-memory SQLite database of its own: `fake_db`
opens one, creates the tables, binds `core.db` to it and stubs out
`core.db.connect_db()` — the one place that would open a file of its own — so
nothing here follows `.env`. A SQLite file and an in-memory SQLite are the same
engine, so there is no second backend to run the suite against.
"""

import json
import os
import random

import pytest
from sqlalchemy import select
from sqlalchemy.orm import Session
from sqlalchemy.pool import StaticPool

from haute_tension.core import db as core_db
from haute_tension.core.models.base import Base
from haute_tension.core.models.game import Game
from haute_tension.core.models.item import Item
from haute_tension.core.models.page_inspection import PageInspection
from haute_tension.core.models.page_view import PageView
from haute_tension.core.story import PAGES_FILE

BOOK = "pretre_jean/forteresse_alamuth"

# The tables, by the name the application gives them, so a test can say
# `fake_db["games"]` the way it would name a collection.
TABLES = {
    Game.__tablename__: Game,
    Item.__tablename__: Item,
    PageView.__tablename__: PageView,
    PageInspection.__tablename__: PageInspection,
}


class FakeTable:
    """One table, seen as the list of flat dicts tests set up.

    `docs` is the whole interface: reading gives every row as `to_dict()`, the
    columns and the blob merged; assigning replaces the table's contents. A row
    keyed by an id the new list also carries is updated in place rather than
    deleted and re-inserted, so the reading history that points at a game
    survives that game being changed under it. A game's dict carries its
    `items`, so seeding a game seeds its bag: an item with an id is that row,
    one without is a new row, and one left out is deleted.
    """

    def __init__(self, engine, model) -> None:
        self._engine = engine
        self._model = model

    @property
    def docs(self) -> list[dict]:
        with Session(self._engine) as session:
            return [row.to_dict() for row in session.scalars(select(self._model))]

    @docs.setter
    def docs(self, docs: list[dict]) -> None:
        kept = {doc["id"] for doc in docs if "id" in doc}
        with Session(self._engine) as session:
            for row in session.scalars(select(self._model)):
                if row.id not in kept:
                    session.delete(row)
            session.flush()
            for doc in docs:
                row = self._model.from_dict(doc)
                if "id" in doc:
                    session.merge(row)
                else:
                    session.add(row)
            session.commit()


class FakeDB:
    """The database, indexed by table name."""

    def __init__(self, engine) -> None:
        self._engine = engine

    def __getitem__(self, name: str) -> FakeTable:
        return FakeTable(self._engine, TABLES[name])


@pytest.fixture
def fake_db(monkeypatch):
    """Bind `core.db` to an in-memory database of its own, empty.

    `connect_db()` is the single seam: stubbing it out is what keeps every read
    and write in `core.db` from opening a file of its own. One connection is
    shared for the whole test, because an in-memory SQLite lives and dies with
    its connection.
    """
    engine = core_db._open_engine(
        "sqlite://",
        poolclass=StaticPool,
        connect_args={"check_same_thread": False},
    )
    Base.metadata.create_all(engine)
    monkeypatch.setattr(core_db, "_engine", engine)
    monkeypatch.setattr(core_db, "connect_db", lambda: None)

    yield FakeDB(engine)

    engine.dispose()


@pytest.fixture(autouse=True)
def clean_env(monkeypatch):
    """.env is loaded at import time; keep tests independent of the real one."""
    for name in ("APP_ENV", "DATABASE_DIR"):
        monkeypatch.delenv(name, raising=False)
    core_db.reset_connection()


@pytest.fixture
def books_path(tmp_path):
    """A small navigable book on disk, so no test reads the packaged one."""
    book_path = tmp_path / BOOK
    book_path.mkdir(parents=True)
    (book_path / PAGES_FILE).write_text(
        json.dumps(
            [
                {
                    "page": "1",
                    "language": "fr",
                    "text": ["Le début de l'aventure.", "Deuxième ligne."],
                    "file_path": "raw_data/1.html",
                    "choices": [{"goto": "2", "gains": [], "losses": []}],
                },
                {
                    "page": "2",
                    "language": "fr",
                    "text": ["La fin de l'aventure."],
                    "file_path": "raw_data/2.html",
                    "choices": [],
                },
            ]
        ),
        encoding="utf-8",
    )
    return tmp_path


@pytest.fixture
def books_path_with_fights(books_path):
    """The same book, with a fight on page 1 and a melee on page 2."""
    path = books_path / BOOK / PAGES_FILE
    pages = json.loads(path.read_text(encoding="utf-8"))
    pages[0]["fight"] = {
        "fight_type": "single",
        "enemies": [{"name": "collecteur", "force": 6, "vie": 10}],
        "outcome": {"on_victory": "2", "on_defeat": "death", "on_flee": None},
    }
    pages[0]["choices"] = [
        {
            "goto": "2",
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
            "losses": [{"element": "gold coin", "label_fr": "pièce d'or", "amount": 3}],
        },
        {
            "goto": "4",
            "gains": [
                {
                    "element": "strength point",
                    "label_fr": "point de Force",
                    "amount": 1,
                    "condition": "pendant tout le temps où vous les porterez",
                }
            ],
            "losses": [],
        },
    ]
    pages[1]["fight"] = {
        "fight_type": "simultaneous",
        "enemies": [{"name": "lepreux", "force": 6, "vie": 6, "count": 2}],
        "outcome": {"on_victory": "1", "on_defeat": "death", "on_flee": None},
    }
    pages.append(
        {
            "page": "3",
            "language": "fr",
            "text": ["Une page dont le combat est mal analysé."],
            "file_path": "raw_data/3.html",
            "choices": [],
            # Truthy, so the page looks like it holds a fight, but nothing in it
            # can be fielded — which the parser does produce.
            "fight": {"fight_type": "single", "enemies": ["orc"], "outcome": {}},
        }
    )
    pages.append(
        {
            "page": "4",
            "language": "fr",
            "text": ["Une page paisible."],
            "file_path": "raw_data/4.html",
            "choices": [],
        }
    )
    path.write_text(json.dumps(pages), encoding="utf-8")
    return books_path


@pytest.fixture
def client(fake_db, books_path):
    """A test client for an app serving that book, with a database behind it."""
    return _client(books_path)


@pytest.fixture
def fighting_client(fake_db, books_path_with_fights):
    """A client whose book has fights, with the dice fixed for the whole app."""
    return _client(books_path_with_fights, random.Random(1))


@pytest.fixture
def hero(fighting_client):
    """A client that already has a hero rolled up."""
    fighting_client.post("/game/new")
    return fighting_client


def _client(books_path, rng=None):
    """Build a test client for a book on disk."""
    from haute_tension.application.factory import create_app

    app = create_app(BOOK, books_path, rng)
    app.config.update(TESTING=True)
    app.secret_key = "test-secret"
    return app.test_client()
