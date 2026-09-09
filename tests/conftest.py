"""Fixtures every test file shares: the fake database, and an app on top of it.

No test here ever needs a running mongod: the models are pointed at an in-memory
mongomock server, and `core.db.connect_db()` — the one place that would reach for
a server — is stubbed out.
"""

import copy

import mongoengine
import mongomock
import pytest

from haute_tension.core import db as core_db
from haute_tension.core.models import PageView, StoryPage

# The database the models are bound to for the whole session. Its name matters
# only in what the import script prints; `current_db_name()` gives the same one,
# since clean_env drops APP_ENV.
TEST_DB_NAME = "haute_tension_dev"

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
    """Bind the models to an in-memory server, emptied for every test.

    `connect_db()` is the single seam: stubbing it out is what keeps every read
    and write in `core.db` from reaching for a real server.
    """
    mongoengine.disconnect()
    mongoengine.connect(
        db=TEST_DB_NAME,
        mongo_client_class=mongomock.MongoClient,
        uuidRepresentation="standard",
    )
    monkeypatch.setattr(core_db, "connect_db", lambda: None)

    database = mongoengine.get_db()
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
