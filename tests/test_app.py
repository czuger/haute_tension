"""Tests for the app factory and the development-server entry point."""

from unittest.mock import patch

import pytest

from haute_tension.application.factory import BOOK, create_app


class TestCreateApp:
    """Wiring the app onto a book read off disk."""

    def test_every_blueprint_is_registered(self, books_path):
        assert sorted(create_app(BOOK, books_path).blueprints) == [
            "api",
            "game",
            "inspection",
            "web",
        ]

    def test_the_session_cookie_is_signed(self, books_path):
        assert create_app(BOOK, books_path).secret_key

    def test_the_app_is_built_without_a_database(self, books_path):
        """The book is on disk, so building the app reaches for no server."""
        assert create_app(BOOK, books_path).blueprints

    def test_a_page_needing_the_database_refuses_without_one(self, books_path):
        """Nothing is served half-built: the reader is told instead."""
        client = create_app(BOOK, books_path).test_client()

        assert client.get("/book/1").status_code == 503

    def test_the_landing_page_needs_no_database(self, books_path):
        """It shows the book and a link, and reads nothing."""
        client = create_app(BOOK, books_path).test_client()

        assert client.get("/").status_code == 200

    def test_the_book_is_read_once_at_startup(self, fake_db, books_path):
        from haute_tension.core.story import PAGES_FILE

        app = create_app(BOOK, books_path)
        (books_path / BOOK / PAGES_FILE).write_text("[]", encoding="utf-8")

        assert app.test_client().get("/book/1").status_code == 200

    def test_a_missing_book_is_reported(self, tmp_path):
        with pytest.raises(FileNotFoundError):
            create_app(BOOK, tmp_path)

    def test_the_packaged_book_is_the_default(self, fake_db):
        assert create_app().test_client().get("/book/1").status_code == 200


class TestMain:
    """The development server."""

    def test_it_serves_on_localhost_only(self):
        from haute_tension import app as app_module

        with patch.object(app_module.app, "run") as run:
            app_module.main()

        run.assert_called_once_with(host="127.0.0.1", port=5001, debug=True)
