"""What a reader is shown when the database cannot answer."""

import pytest
from sqlalchemy.exc import OperationalError

from haute_tension.core import db as core_db
from haute_tension.core.db import DatabaseUnavailable


@pytest.fixture
def dead_database(monkeypatch):
    """Every read and write fails the way a file that cannot be opened looks."""

    def boom():
        raise OperationalError("PRAGMA foreign_keys=ON", {}, Exception("unable to open database file"))

    monkeypatch.setattr(core_db, "connect_db", boom)


@pytest.fixture
def no_database(monkeypatch):
    """Nothing configured to reach in the first place."""

    def boom():
        raise DatabaseUnavailable("Missing DATABASE_DIR environment variable.")

    monkeypatch.setattr(core_db, "connect_db", boom)


class TestThePagesThatNeedIt:
    """A page that cannot be built is refused, not served half-built."""

    def test_the_reader_is_told(self, client, dead_database):
        response = client.get("/book/1")

        assert response.status_code == 503
        assert "ne répond pas" in response.get_data(as_text=True)

    def test_the_story_text_is_not_served(self, client, dead_database):
        """Half a page, quietly missing its trail, is what this replaces."""
        body = client.get("/book/1").get_data(as_text=True)

        assert "Le début de l&#39;aventure." not in body
        assert 'class="trail"' not in body

    def test_no_database_configured_reads_the_same(self, client, no_database):
        response = client.get("/book/1")

        assert response.status_code == 503
        assert "ne répond pas" in response.get_data(as_text=True)

    def test_the_page_points_at_the_configuration(self, client, no_database):
        assert "DATABASE_DIR" in client.get("/book/1").get_data(as_text=True)

    def test_rolling_up_a_hero_is_refused(self, fighting_client, no_database):
        assert fighting_client.post("/game/new").status_code == 503

    def test_an_assault_is_refused(self, hero, dead_database):
        assert hero.post("/combat/1/assault").status_code == 503


class TestTheApiAnswersJson:
    """`/data/<number>` is parsed, not read."""

    def test_the_refusal_is_json(self, client, dead_database):
        response = client.get("/data/1")

        assert response.status_code == 503
        assert response.mimetype == "application/json"

    def test_it_says_what_is_wrong(self, client, dead_database):
        assert client.get("/data/1").get_json() == {
            "success": False,
            "message": "la base de données est injoignable",
        }

    def test_it_is_not_an_html_page(self, client, dead_database):
        assert "<!doctype html>" not in client.get("/data/1").get_data(as_text=True)


class TestThePagesThatDoNot:
    """What needs no database still works when there is none."""

    def test_the_landing_page_is_served(self, client, dead_database):
        import html

        response = client.get("/")

        assert response.status_code == 200
        assert "La Forteresse d'Alamuth" in html.unescape(
            response.get_data(as_text=True)
        )

    def test_an_unknown_page_is_still_a_404(self, client, dead_database):
        """The book is in memory, so a missing page is known without a query."""
        assert client.get("/book/999").status_code == 404


class TestTheLog:
    """A refusal leaves the reason behind it."""

    def test_the_failure_is_written_with_its_traceback(
        self, client, dead_database, caplog
    ):
        import logging

        caplog.set_level(logging.DEBUG, logger="haute_tension.general")

        client.get("/book/1")

        assert "The database could not answer" in caplog.text
        assert "OperationalError" in caplog.text

    def test_the_line_names_the_request(self, client, dead_database, caplog):
        import logging

        caplog.set_level(logging.DEBUG, logger="haute_tension.general")

        client.get("/book/1")

        assert "path='/book/1'" in caplog.text
        assert "endpoint='web.read_page'" in caplog.text
