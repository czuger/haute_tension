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
import os

import mongoengine
import mongomock
import pytest

from haute_tension.core import db as core_db
from haute_tension.core.models import PageView, StoryPage

# A real MongoDB to run against, instead of mongomock. The Makefile sets it; an
# empty environment means the in-memory server.
MONGO_URI_TEST = os.environ.get("MONGO_URI_TEST", "").strip()

# The database the models are bound to for the whole session, and it is emptied
# before every test — so it must never be one the application uses. Deliberately
# not `haute_tension_dev`: the test server is meant to be a container of its own,
# but nothing stops a MONGO_URI from pointing the application at it, and a name
# of its own is what keeps `make test` from dropping an imported book.
#
# Unrelated to what `current_db_name()` returns, which is what the import script
# prints and which `connect_db()` — stubbed here — would have connected to.
TEST_DB_NAME = "haute_tension_test"

# The collections whose `_id` is the book's own id (a page number) rather than an
# ObjectId — everywhere the app says `page`, Mongo says `_id`, and `docs` below
# shows them the way the app writes and reads them.
MODELS = (StoryPage, PageView)
SOURCE_ID_KEYS = {
    model.collection_name(): next(
        (key for key in model.stored_fields() if key == model._meta.get("id_field")),
        None,
    )
    for model in MODELS
}

BOOK = "pretre_jean/forteresse_alamuth"


class FakeCollection:
    """One mongomock collection, seen as the list of documents tests set up.

    `docs` is the whole interface: assigning replaces the collection's contents,
    reading gives them back with Mongo's `_id` shown as the app's own key (or
    dropped, for the collections that have no id of their own). Documents are
    copied in, so a test that hands over a shared dict does not get it back
    stamped with an `_id`.
    """

    def __init__(self, collection, source_id_key: str | None) -> None:
        self._collection = collection
        self._source_id_key = source_id_key

    @property
    def docs(self) -> list[dict]:
        return [self._as_app_dict(doc) for doc in self._collection.find()]

    @docs.setter
    def docs(self, docs: list[dict]) -> None:
        self._collection.delete_many({})
        if docs:
            self._collection.insert_many([self._as_document(doc) for doc in docs])

    def _as_app_dict(self, doc: dict) -> dict:
        if not self._source_id_key:
            return {key: value for key, value in doc.items() if key != "_id"}
        return {
            (self._source_id_key if key == "_id" else key): value
            for key, value in doc.items()
        }

    def _as_document(self, doc: dict) -> dict:
        doc = copy.deepcopy(doc)
        if not self._source_id_key:
            return doc
        return {
            ("_id" if key == self._source_id_key else key): value
            for key, value in doc.items()
        }


class FakeDB:
    """The mongomock database, indexed by collection name like a pymongo one."""

    def __init__(self, database) -> None:
        self._database = database

    def __getitem__(self, name: str) -> FakeCollection:
        return FakeCollection(self._database[name], SOURCE_ID_KEYS.get(name))

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
def book_pages(fake_db):
    """A small navigable book, already imported."""
    fake_db["story_pages"].docs = [
        {
            "page": "1",
            "book": BOOK,
            "language": "fr",
            "text": ["Le début de l'aventure.", "Deuxième ligne."],
            "file_path": "raw_data/1.html",
            "choices": [{"goto": "2", "gains": [], "losses": []}],
        },
        {
            "page": "2",
            "book": BOOK,
            "language": "fr",
            "text": ["La fin de l'aventure."],
            "file_path": "raw_data/2.html",
            "choices": [],
        },
    ]
    return fake_db


@pytest.fixture
def client(book_pages):
    """A test client for an app serving the imported book."""
    from haute_tension.application.factory import create_app

    app = create_app(BOOK)
    app.config.update(TESTING=True)
    return app.test_client()
