"""Tests for the app factory and the development-server entry point."""

from unittest.mock import patch

import pytest

from haute_tension.application.factory import BOOK, create_app


class TestCreateApp:
    """Wiring the app onto a book read off disk."""

    def test_both_blueprints_are_registered(self, books_path):
        assert sorted(create_app(BOOK, books_path).blueprints) == ["api", "web"]

    def test_the_book_is_served_without_a_database(self, books_path):
        """Browsing needs no mongod: only the history is stored."""
        client = create_app(BOOK, books_path).test_client()

        assert client.get("/book/1").status_code == 200

    def test_the_book_is_read_once_at_startup(self, books_path):
        from haute_tension.core.story import PAGES_FILE

        app = create_app(BOOK, books_path)
        (books_path / BOOK / PAGES_FILE).write_text("[]", encoding="utf-8")

        assert app.test_client().get("/book/1").status_code == 200

    def test_a_missing_book_is_reported(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            create_app(BOOK, tmp_path)

    def test_the_packaged_book_is_the_default(self):
        assert create_app().test_client().get("/book/1").status_code == 200


class TestMain:
    """The development server."""

    def test_it_serves_on_localhost_only(self):
        from haute_tension import app as app_module

        with patch.object(app_module.app, "run") as run:
            app_module.main()

        run.assert_called_once_with(host="127.0.0.1", port=5001, debug=True)
