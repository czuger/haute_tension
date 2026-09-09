"""Tests for opening the connection every read and write goes through.

The one part of `core.db` the other tests stub out, so it is exercised here with
mongoengine's own `connect` / `disconnect` replaced: nothing below reaches for a
server either.
"""

from types import SimpleNamespace

import pytest

from haute_tension.core import db as core_db


@pytest.fixture
def connections(monkeypatch):
    """Record what `connect_db()` asks mongoengine to do, without doing it."""
    calls = SimpleNamespace(connected=[], disconnected=0, opened_on="haute_tension_dev")

    def fake_connect(**kwargs):
        calls.connected.append(kwargs)

    def fake_disconnect():
        calls.disconnected += 1

    monkeypatch.setattr(core_db, "connect", fake_connect)
    monkeypatch.setattr(core_db, "disconnect", fake_disconnect)
    monkeypatch.setattr(
        core_db, "mongoengine_db", lambda: SimpleNamespace(name=calls.opened_on)
    )
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")
    return calls


class TestConnectDb:
    """Opening, reusing and refusing a connection."""

    def test_the_env_database_is_opened(self, connections):
        core_db.connect_db()

        assert connections.connected[0]["db"] == "haute_tension_dev"
        assert connections.connected[0]["host"] == "mongodb://localhost:27017"

    def test_the_timeout_bounds_an_unreachable_server(self, connections):
        core_db.connect_db()

        assert connections.connected[0]["serverSelectionTimeoutMS"] == (
            core_db.SERVER_SELECTION_TIMEOUT_MS
        )

    def test_a_second_call_reuses_the_connection(self, connections):
        core_db.connect_db()
        core_db.connect_db()

        assert len(connections.connected) == 1

    def test_switching_env_reconnects(self, connections, monkeypatch):
        core_db.connect_db()

        monkeypatch.setenv("APP_ENV", "prod")
        connections.opened_on = "haute_tension_prod"
        core_db.connect_db()

        assert [call["db"] for call in connections.connected] == [
            "haute_tension_dev",
            "haute_tension_prod",
        ]
        assert connections.disconnected == 1

    def test_a_missing_uri_is_refused(self, monkeypatch):
        monkeypatch.delenv("MONGO_URI", raising=False)

        with pytest.raises(EnvironmentError, match="Missing MONGO_URI"):
            core_db.connect_db()

    def test_a_uri_naming_another_database_is_refused(self, connections):
        connections.opened_on = "somewhere_else"

        with pytest.raises(EnvironmentError, match="points at the database"):
            core_db.connect_db()

        assert connections.disconnected == 1

    def test_a_refused_uri_leaves_no_connection_behind(self, connections):
        connections.opened_on = "somewhere_else"
        with pytest.raises(EnvironmentError):
            core_db.connect_db()

        connections.opened_on = "haute_tension_dev"
        core_db.connect_db()

        assert len(connections.connected) == 2


class TestResetConnection:
    """Closing the open connection, which only the tests need."""

    def test_the_next_call_reconnects(self, connections):
        core_db.connect_db()
        core_db.reset_connection()
        core_db.connect_db()

        assert len(connections.connected) == 2

    def test_the_open_connection_is_dropped_not_just_forgotten(self, connections):
        """mongoengine refuses a second connection under the same alias."""
        core_db.connect_db()
        core_db.reset_connection()

        assert connections.disconnected == 1

    def test_resetting_without_a_connection_disconnects_nothing(self, connections):
        core_db.reset_connection()

        assert connections.disconnected == 0


class TestDatabaseError:
    """What a caller catches around a database call."""

    def test_both_driver_families_are_caught(self):
        from mongoengine.errors import MongoEngineException
        from pymongo.errors import PyMongoError

        assert core_db.DatabaseError == (PyMongoError, MongoEngineException)

    def test_an_optional_history_also_tolerates_no_database_at_all(self):
        """Never configuring a server raises EnvironmentError, not a driver one."""
        assert EnvironmentError in core_db.HistoryUnavailable
        assert set(core_db.DatabaseError) < set(core_db.HistoryUnavailable)
