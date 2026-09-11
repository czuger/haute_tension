"""Tests for how a run picks its database."""

import pytest

from haute_tension.core.config import (
    ROOT,
    current_db_name,
    current_env,
    database_path,
    load_env,
)


class TestCurrentEnv:
    """Which environment a run works in."""

    def test_dev_is_the_default(self):
        assert current_env() == "dev"

    @pytest.mark.parametrize("name", ["dev", "prod"])
    def test_a_known_env_is_accepted(self, monkeypatch, name):
        monkeypatch.setenv("APP_ENV", name)

        assert current_env() == name

    def test_case_and_padding_are_ignored(self, monkeypatch):
        monkeypatch.setenv("APP_ENV", "  PROD ")

        assert current_env() == "prod"

    def test_an_unknown_env_is_refused(self, monkeypatch):
        monkeypatch.setenv("APP_ENV", "staging")

        with pytest.raises(EnvironmentError, match="APP_ENV must be one of"):
            current_env()


class TestCurrentDbName:
    """The database name a run reads and writes."""

    def test_the_env_suffixes_the_database_name(self, monkeypatch):
        monkeypatch.setenv("APP_ENV", "prod")

        assert current_db_name() == "haute_tension_prod"

    def test_dev_and_prod_are_separate_databases(self, monkeypatch):
        monkeypatch.setenv("APP_ENV", "dev")
        dev = current_db_name()
        monkeypatch.setenv("APP_ENV", "prod")

        assert dev != current_db_name()


class TestLoadEnv:
    """Refusing to run without the variables a script needs."""

    def test_set_variables_are_returned(self, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "data")

        assert load_env(["DATABASE_DIR"]) == {"DATABASE_DIR": "data"}

    def test_nothing_required_is_nothing_to_check(self):
        assert load_env([]) == {}

    def test_a_missing_variable_is_refused(self):
        with pytest.raises(EnvironmentError, match="DATABASE_DIR"):
            load_env(["DATABASE_DIR"])

    def test_an_empty_variable_counts_as_missing(self, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "")

        with pytest.raises(EnvironmentError, match="DATABASE_DIR"):
            load_env(["DATABASE_DIR"])

    def test_every_missing_variable_is_named(self):
        with pytest.raises(EnvironmentError) as raised:
            load_env(["DATABASE_DIR", "APP_SECRET"])

        assert "DATABASE_DIR" in str(raised.value)
        assert "APP_SECRET" in str(raised.value)


class TestDatabasePath:
    """Which SQLite file a run opens."""

    def test_the_file_is_named_after_the_env(self, monkeypatch, tmp_path):
        monkeypatch.setenv("DATABASE_DIR", str(tmp_path))
        monkeypatch.setenv("APP_ENV", "prod")

        assert database_path() == tmp_path / "haute_tension_prod.sqlite3"

    def test_a_relative_directory_is_taken_from_the_repository_root(self, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "data")

        assert database_path() == ROOT / "data" / "haute_tension_dev.sqlite3"

    def test_a_home_directory_is_expanded(self, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "~/haute_tension_data")

        assert "~" not in str(database_path())
        assert database_path().is_absolute()

    def test_dev_and_prod_are_separate_files(self, monkeypatch, tmp_path):
        monkeypatch.setenv("DATABASE_DIR", str(tmp_path))
        monkeypatch.setenv("APP_ENV", "dev")
        dev = database_path()
        monkeypatch.setenv("APP_ENV", "prod")

        assert dev != database_path()
        assert dev.parent == database_path().parent

    def test_nothing_configured_is_no_path(self):
        assert database_path() is None

    def test_a_blank_directory_counts_as_unset(self, monkeypatch):
        monkeypatch.setenv("DATABASE_DIR", "   ")

        assert database_path() is None
