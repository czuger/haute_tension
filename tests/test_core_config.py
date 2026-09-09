"""Tests for how a run picks its database."""

import pytest

from haute_tension.core.config import current_db_name, current_env, load_env


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
        monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")

        assert load_env(["MONGO_URI"]) == {"MONGO_URI": "mongodb://localhost:27017"}

    def test_nothing_required_is_nothing_to_check(self):
        assert load_env([]) == {}

    def test_a_missing_variable_is_refused(self):
        with pytest.raises(EnvironmentError, match="MONGO_URI"):
            load_env(["MONGO_URI"])

    def test_an_empty_variable_counts_as_missing(self, monkeypatch):
        monkeypatch.setenv("MONGO_URI", "")

        with pytest.raises(EnvironmentError, match="MONGO_URI"):
            load_env(["MONGO_URI"])

    def test_every_missing_variable_is_named(self):
        with pytest.raises(EnvironmentError) as raised:
            load_env(["MONGO_URI", "APP_SECRET"])

        assert "MONGO_URI" in str(raised.value)
        assert "APP_SECRET" in str(raised.value)
