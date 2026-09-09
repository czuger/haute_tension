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
import random

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


# The collections whose `_id` is an id of the application's own rather than an
# ObjectId. Everywhere the app says `id`, Mongo says `_id`, and `docs` below
# shows them the way the app writes and reads them — which is what lets a test
# seed a game by the id it will then look up.
SOURCE_ID_COLLECTIONS = {"games"}


class FakeCollection:
    """One collection, seen as the list of documents tests set up.

    `docs` is the whole interface: assigning replaces the collection's contents,
    reading gives them back with Mongo's `_id` shown as the app's own `id`, or
    dropped for the collections that have no id of their own. Documents are
    copied in, so a test that hands over a shared dict does not get it back
    stamped with an `_id`.
    """

    def __init__(self, collection, keyed_by_source_id: bool) -> None:
        self._collection = collection
        self._keyed_by_source_id = keyed_by_source_id

    @property
    def docs(self) -> list[dict]:
        return [self._as_app_dict(doc) for doc in self._collection.find()]

    @docs.setter
    def docs(self, docs: list[dict]) -> None:
        self._collection.delete_many({})
        if docs:
            self._collection.insert_many(
                [self._as_document(doc) for doc in docs]
            )

    def _as_app_dict(self, doc: dict) -> dict:
        if not self._keyed_by_source_id:
            return {key: value for key, value in doc.items() if key != "_id"}
        return {("id" if key == "_id" else key): value for key, value in doc.items()}

    def _as_document(self, doc: dict) -> dict:
        doc = copy.deepcopy(doc)
        if not self._keyed_by_source_id:
            return doc
        return {("_id" if key == "id" else key): value for key, value in doc.items()}


class FakeDB:
    """The database, indexed by collection name like a pymongo one."""

    def __init__(self, database) -> None:
        self._database = database

    def __getitem__(self, name: str) -> FakeCollection:
        return FakeCollection(
            self._database[name], name in SOURCE_ID_COLLECTIONS
        )

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
def books_path_with_fights(books_path):
    """The same book, with a fight on page 1 and a melee on page 2."""
    path = books_path / BOOK / PAGES_FILE
    pages = json.loads(path.read_text(encoding="utf-8"))
    pages[0]["fight"] = {
        "fight_type": "single",
        "enemies": [{"name": "collecteur", "force": 6, "vie": 10}],
        "outcome": {"on_victory": "2", "on_defeat": "death", "on_flee": None},
    }
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
