"""Fixtures every test file shares: the database, and an app on top of it.

The suite runs the same way against two backends, and every test is written not
to care which:

- **`python -m pytest`** — the models are bound to an in-memory mongomock server.
  No mongod, nothing to bring up, and the whole suite runs in a fraction of a
  second. This is the pass one wants while writing a test.
- **`make test`** — `MONGO_URI_TEST` names a real MongoDB (the Makefile brings
  one up in a container), and the models are bound to that instead. Slower, and
  the one that counts before pushing: mongomock is a reimplementation, so a
  driver behaviour it does not share is a bug this suite would otherwise never
  see.

Either way `core.db.connect_db()` — the one place that would reach for a server
of its own — is stubbed out, so nothing here follows `.env`.
"""

import copy
import json
import os

import mongoengine
import mongomock
import pytest

from haute_tension.core import db as core_db
from haute_tension.core.story import PAGES_FILE

# A real MongoDB to run against, instead of mongomock. The Makefile sets it; an
# empty environment means the in-memory server.
MONGO_URI_TEST = os.environ.get("MONGO_URI_TEST", "").strip()

# The database the models are bound to for the whole session, and it is emptied
# before every test — so it must never be one the application uses. Deliberately
# not `haute_tension_dev`: the test server is meant to be a container of its own,
# but nothing stops a MONGO_URI from pointing the application at it, and a name
# of its own is what then keeps `make test` from dropping a reading history.
#
# Unrelated to what `current_db_name()` returns, which is what `connect_db()` —
# stubbed here — would have connected to.
TEST_DB_NAME = "haute_tension_test"

BOOK = "pretre_jean/forteresse_alamuth"


class FakeCollection:
    """One collection, seen as the list of documents tests set up.

    `docs` is the whole interface: assigning replaces the collection's contents,
    reading gives them back without Mongo's `_id`, which nothing in this
    application reads. Documents are copied in, so a test that hands over a
    shared dict does not get it back stamped with an `_id`.
    """

    def __init__(self, collection) -> None:
        self._collection = collection

    @property
    def docs(self) -> list[dict]:
        return [
            {key: value for key, value in doc.items() if key != "_id"}
            for doc in self._collection.find()
        ]

    @docs.setter
    def docs(self, docs: list[dict]) -> None:
        self._collection.delete_many({})
        if docs:
            self._collection.insert_many(copy.deepcopy(docs))


class FakeDB:
    """The database, indexed by collection name like a pymongo one."""

    def __init__(self, database) -> None:
        self._database = database

    def __getitem__(self, name: str) -> FakeCollection:
        return FakeCollection(self._database[name])

    def list_collection_names(self) -> list[str]:
        return self._database.list_collection_names()


@pytest.fixture
def fake_db(monkeypatch):
    """Bind the models to a database of their own, emptied for every test.

    mongomock unless `MONGO_URI_TEST` names a real server. `connect_db()` is the
    single seam: stubbing it out is what keeps every read and write in `core.db`
    from opening a connection of its own.
    """
    mongoengine.disconnect()
    if MONGO_URI_TEST:
        mongoengine.connect(
            db=TEST_DB_NAME,
            host=MONGO_URI_TEST,
            serverSelectionTimeoutMS=core_db.SERVER_SELECTION_TIMEOUT_MS,
            uuidRepresentation="standard",
        )
    else:
        mongoengine.connect(
            db=TEST_DB_NAME,
            mongo_client_class=mongomock.MongoClient,
            uuidRepresentation="standard",
        )
    monkeypatch.setattr(core_db, "connect_db", lambda: None)

    database = mongoengine.get_db()
    # Dropped rather than emptied: an index left behind by another test's model
    # would outlive the documents it was built for.
    for name in database.list_collection_names():
        database.drop_collection(name)

    yield FakeDB(database)

    mongoengine.disconnect()
    core_db.reset_connection()


@pytest.fixture(autouse=True)
def clean_env(monkeypatch):
    """.env is loaded at import time; keep tests independent of the real one."""
    for name in ("APP_ENV", "MONGO_URI"):
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
def client(fake_db, books_path):
    """A test client for an app serving that book, with a database behind it."""
    from haute_tension.application.factory import create_app

    app = create_app(BOOK, books_path)
    app.config.update(TESTING=True)
    return app.test_client()
