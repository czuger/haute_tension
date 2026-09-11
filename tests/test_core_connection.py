"""Tests for opening the engine every read and write goes through.

The one part of `core.db` the other tests stub out, so it is exercised here for
real: on SQLite files in a temporary directory, which is what a run opens.
"""

import sqlite3

import pytest
from sqlalchemy import inspect

from haute_tension.core import db as core_db


@pytest.fixture
def database_dir(monkeypatch, tmp_path):
    """A directory of the test's own for the files `connect_db()` opens."""
    monkeypatch.setenv("DATABASE_DIR", str(tmp_path))
    return tmp_path


def opened_file() -> str:
    """The file the current engine is on."""
    return core_db._engine.url.database


class TestConnectDb:
    """Opening, reusing and refusing a connection."""

    def test_the_env_database_is_opened(self, database_dir):
        core_db.connect_db()

        assert opened_file() == str(database_dir / "haute_tension_dev.sqlite3")

    def test_the_tables_are_created(self, database_dir):
        core_db.connect_db()

        assert sorted(inspect(core_db._engine).get_table_names()) == [
            "games",
            "items",
            "page_inspections",
            "page_views",
        ]

    def test_a_missing_directory_is_made(self, database_dir, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", str(database_dir / "deeper" / "still"))

        core_db.connect_db()

        assert (database_dir / "deeper" / "still").is_dir()

    def test_a_second_call_reuses_the_engine(self, database_dir):
        core_db.connect_db()
        engine = core_db._engine
        core_db.connect_db()

        assert core_db._engine is engine

    def test_switching_env_reopens(self, database_dir, monkeypatch):
        core_db.connect_db()
        dev = core_db._engine

        monkeypatch.setenv("APP_ENV", "prod")
        core_db.connect_db()

        assert core_db._engine is not dev
        assert opened_file() == str(database_dir / "haute_tension_prod.sqlite3")

    def test_a_missing_directory_variable_is_refused(self):
        with pytest.raises(core_db.DatabaseUnavailable, match="Missing DATABASE_DIR"):
            core_db.connect_db()

    def test_a_directory_that_is_a_file_is_refused(self, database_dir, monkeypatch):
        not_a_directory = database_dir / "a_file"
        not_a_directory.write_text("", encoding="utf-8")
        monkeypatch.setenv("DATABASE_DIR", str(not_a_directory))

        with pytest.raises(core_db.DatabaseUnavailable, match="not a directory"):
            core_db.connect_db()

    def test_a_refusal_leaves_no_engine_behind(self, database_dir, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "")
        with pytest.raises(core_db.DatabaseUnavailable):
            core_db.connect_db()

        assert core_db._engine is None

        monkeypatch.setenv("DATABASE_DIR", str(database_dir))
        core_db.connect_db()

        assert core_db._engine is not None

    def test_what_is_written_is_read_back_from_the_file(self, database_dir):
        """The whole point: a second engine on the same file sees the row."""
        core_db.record_page_view("livre", "22")
        core_db.reset_connection()

        assert core_db.last_pages("livre") == ["22"]

    def test_foreign_keys_are_enforced(self, database_dir):
        """SQLite only checks them when told to, on every connection."""
        from sqlalchemy import text
        from sqlalchemy.exc import IntegrityError

        core_db.connect_db()

        with core_db._engine.connect() as connection, pytest.raises(IntegrityError):
            connection.execute(
                text(
                    "INSERT INTO page_views (book, game_id, viewed_at, data) "
                    "VALUES ('livre', 999, '2026', '{}')"
                )
            )


FILE = "haute_tension_dev.sqlite3"


def version_of(path) -> int:
    """The schema version a file carries."""
    connection = sqlite3.connect(path)
    version = connection.execute("PRAGMA user_version").fetchone()[0]
    connection.close()
    return version


def tables_of(path) -> list[str]:
    """The tables a file holds."""
    connection = sqlite3.connect(path)
    names = [row[0] for row in connection.execute(
        "SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name"
    )]
    connection.close()
    return names


def a_file_at(path, version: int, with_a_table: bool = True) -> None:
    """Write a file carrying a schema version, and a table unless told not to."""
    connection = sqlite3.connect(path)
    if with_a_table:
        connection.execute("CREATE TABLE games (id VARCHAR(32) PRIMARY KEY)")
    connection.execute(f"PRAGMA user_version = {version}")
    connection.commit()
    connection.close()


class TestSchemaVersion:
    """Marking a new file, and refusing one another schema wrote."""

    def test_a_new_file_is_marked_with_the_current_version(self, database_dir):
        core_db.connect_db()

        assert version_of(database_dir / FILE) == core_db.SCHEMA_VERSION

    def test_a_file_an_older_schema_wrote_is_refused(self, database_dir):
        a_file_at(database_dir / FILE, 0)

        with pytest.raises(
            core_db.DatabaseUnavailable,
            match="schema version 0, and this code reads version 1.*migrations/",
        ):
            core_db.connect_db()

        assert core_db._engine is None

    def test_a_refused_file_is_left_as_it_was(self, database_dir):
        a_file_at(database_dir / FILE, 0)

        with pytest.raises(core_db.DatabaseUnavailable):
            core_db.connect_db()

        assert version_of(database_dir / FILE) == 0
        assert tables_of(database_dir / FILE) == ["games"]

    def test_a_file_a_newer_schema_wrote_is_refused(self, database_dir):
        a_file_at(database_dir / FILE, core_db.SCHEMA_VERSION + 1)

        with pytest.raises(
            core_db.DatabaseUnavailable,
            match=f"schema version {core_db.SCHEMA_VERSION + 1},",
        ):
            core_db.connect_db()

    def test_a_file_without_tables_is_new_whatever_it_says(self, database_dir):
        a_file_at(database_dir / FILE, 7, with_a_table=False)

        core_db.connect_db()

        assert version_of(database_dir / FILE) == core_db.SCHEMA_VERSION

    def test_a_file_at_the_current_version_is_opened_again(self, database_dir):
        core_db.record_page_view("livre", "1")
        core_db.reset_connection()

        assert core_db.last_pages("livre") == ["1"]


class TestResetConnection:
    """Disposing of the open engine, which only the tests need."""

    def test_the_next_call_reopens(self, database_dir):
        core_db.connect_db()
        first = core_db._engine
        core_db.reset_connection()
        core_db.connect_db()

        assert core_db._engine is not first

    def test_the_engine_is_dropped_not_just_forgotten(self, database_dir):
        core_db.connect_db()
        core_db.reset_connection()

        assert core_db._engine is None
        assert core_db._connected_to is None

    def test_resetting_without_an_engine_is_harmless(self):
        core_db.reset_connection()

        assert core_db._engine is None


class TestDatabaseError:
    """What a caller catches around a database call."""

    def test_the_driver_family_is_caught(self):
        from sqlalchemy.exc import SQLAlchemyError

        assert core_db.DatabaseError == (SQLAlchemyError,)

    def test_no_database_at_all_is_a_failure_of_its_own(self):
        """Never configuring a database is not a driver error, and is caught too."""
        assert core_db.DatabaseUnavailable in core_db.DatabaseFailure
        assert set(core_db.DatabaseError) < set(core_db.DatabaseFailure)

    def test_it_is_narrower_than_every_os_error(self):
        """Registering a handler for OSError would swallow far more than this."""
        assert issubclass(core_db.DatabaseUnavailable, OSError)
        assert core_db.DatabaseUnavailable is not OSError
