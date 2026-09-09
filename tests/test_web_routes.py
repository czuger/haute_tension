"""Tests for the landing page, book navigation and the reading trail."""

import html
import re

from haute_tension.core import db as core_db

BOOK = "pretre_jean/forteresse_alamuth"


def trail_of(client, number):
    """The page numbers the breadcrumb shows, in order, for one reader page."""
    body = client.get(f"/book/{number}").get_data(as_text=True)
    breadcrumb = re.search(r'<nav class="trail".*?</nav>', body, re.S)
    if breadcrumb is None:
        return None
    return re.findall(r">(\d+)<", breadcrumb.group(0))


class TestLandingPage:
    """The one book on offer."""

    def test_the_current_book_is_shown(self, client):
        response = client.get("/")

        assert response.status_code == 200
        assert "La Forteresse d'Alamuth" in html.unescape(
            response.get_data(as_text=True)
        )

    def test_it_links_to_the_opening_page(self, client):
        assert 'href="/book/1"' in client.get("/").get_data(as_text=True)

    def test_it_shows_no_trail(self, client):
        client.get("/book/1")

        assert 'class="trail"' not in client.get("/").get_data(as_text=True)


class TestReader:
    """Reading a page and moving on from it."""

    def test_choices_come_before_the_story_text(self, client):
        body = client.get("/book/1").get_data(as_text=True)

        assert body.index('href="/book/2"') < body.index("Le début de l&#39;aventure.")

    def test_the_story_text_is_rendered(self, client):
        body = client.get("/book/1").get_data(as_text=True)

        assert "Le début de l&#39;aventure." in body
        assert "Deuxième ligne." in body

    def test_a_terminal_page_offers_to_restart(self, client):
        body = client.get("/book/2").get_data(as_text=True)

        assert "Fin de l'aventure" in body
        assert 'href="/book/1"' in body

    def test_an_unknown_page_returns_not_found(self, client):
        assert client.get("/book/999").status_code == 404


class TestReadingTrail:
    """The breadcrumb of the last pages read."""

    def test_reading_a_page_records_it(self, client, fake_db):
        client.get("/book/1")

        assert [view["page"] for view in fake_db["page_views"].docs] == ["1"]

    def test_the_trail_ends_on_the_page_being_read(self, client):
        assert trail_of(client, "1") == ["1"]

    def test_the_trail_follows_the_reader(self, client):
        client.get("/book/1")

        assert trail_of(client, "2") == ["1", "2"]

    def test_the_current_page_is_marked_and_not_a_link(self, client):
        client.get("/book/1")
        body = client.get("/book/2").get_data(as_text=True)
        breadcrumb = re.search(r'<nav class="trail".*?</nav>', body, re.S).group(0)

        assert 'aria-current="page">2<' in re.sub(r"\s+", "", breadcrumb)
        assert breadcrumb.count('href="/book/1"') == 1

    def test_earlier_pages_are_links_back(self, client):
        client.get("/book/1")
        body = client.get("/book/2").get_data(as_text=True)

        assert 'href="/book/1"' in body

    def test_a_reload_does_not_lengthen_the_trail(self, client):
        client.get("/book/1")

        assert trail_of(client, "1") == ["1"]
        assert trail_of(client, "1") == ["1"]

    def test_a_loop_is_shown(self, client):
        client.get("/book/1")
        client.get("/book/2")

        assert trail_of(client, "1") == ["1", "2", "1"]

    def test_the_trail_keeps_only_ten_pages(self, client, fake_db):
        """Twelve pages read, plus this visit: the ten most recent are shown."""
        fake_db["page_views"].docs = [
            {"book": BOOK, "page": str(number), "viewed_at": _stamp(number)}
            for number in range(1, 13)
        ]

        trail = trail_of(client, "1")

        assert len(trail) == 10
        assert trail == ["4", "5", "6", "7", "8", "9", "10", "11", "12", "1"]

    def test_another_book_never_shows_up(self, client, fake_db):
        fake_db["page_views"].docs = [
            {"book": "autre/livre", "page": "99", "viewed_at": _stamp(1)}
        ]

        assert trail_of(client, "1") == ["1"]

    def test_an_unreachable_database_costs_only_the_trail(self, client, monkeypatch):
        """The book is on disk, so a dead server must not stop a reader reading."""
        monkeypatch.setattr(core_db, "connect_db", _unreachable)

        response = client.get("/book/1")

        assert response.status_code == 200
        assert "Le début de l&#39;aventure." in response.get_data(as_text=True)
        assert 'class="trail"' not in response.get_data(as_text=True)


def _stamp(minute):
    """A timestamp that orders page views the way they were seeded."""
    from datetime import datetime, timedelta, timezone

    return datetime(2026, 9, 9, 12, 0, tzinfo=timezone.utc) + timedelta(minutes=minute)


def _unreachable():
    """What every read and write looks like when the server is gone."""
    from pymongo.errors import ServerSelectionTimeoutError

    raise ServerSelectionTimeoutError("no reachable servers")
