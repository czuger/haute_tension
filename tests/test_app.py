"""Tests for the app factory and the development-server entry point."""

from unittest.mock import patch

import pytest

from haute_tension.application.factory import BOOK, create_app

pytestmark = pytest.mark.usefixtures("book_pages")


class TestCreateApp:
    """Wiring the app onto a book in the database."""

    def test_both_blueprints_are_registered(self):
        assert sorted(create_app(BOOK).blueprints) == ["api", "web"]

    def test_the_book_is_read_once_at_startup(self, fake_db):
        app = create_app(BOOK)
        fake_db["story_pages"].docs = []

        assert app.test_client().get("/book/1").status_code == 200

    def test_a_book_never_imported_serves_no_pages(self, fake_db):
        fake_db["story_pages"].docs = []
        client = create_app(BOOK).test_client()

        assert client.get("/book/1").status_code == 404


class TestMain:
    """The development server."""

    def test_it_serves_on_localhost_only(self):
        from haute_tension import app as app_module

        with patch.object(app_module.app, "run") as run:
            app_module.main()

        run.assert_called_once_with(host="127.0.0.1", port=5001, debug=True)
