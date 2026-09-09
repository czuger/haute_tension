"""Tests for the script that loads a parsed book into the database."""

import json

import pytest

import import_book
from haute_tension.core.db import load_story

BOOK = "pretre_jean/forteresse_alamuth"


@pytest.fixture
def books_dir(tmp_path, monkeypatch):
    """Stand a book tree in, so no test reads the packaged one."""
    monkeypatch.setattr(import_book, "BOOKS_PATH", tmp_path)
    monkeypatch.setenv("MONGO_URI", "mongodb://localhost:27017")

    def write(book, pages):
        path = tmp_path / book
        path.mkdir(parents=True, exist_ok=True)
        (path / "merged_pages.json").write_text(
            json.dumps(pages) if not isinstance(pages, str) else pages,
            encoding="utf-8",
        )

    return write


class TestReadPages:
    """Getting the parsed pages off disk."""

    def test_pages_are_read(self, books_dir):
        books_dir(BOOK, [{"page": "1"}])

        assert import_book.read_pages(BOOK) == [{"page": "1"}]

    def test_a_missing_book_is_reported(self, books_dir):
        with pytest.raises(SystemExit, match="not found"):
            import_book.read_pages(BOOK)

    def test_a_book_that_is_not_a_list_is_reported(self, books_dir):
        books_dir(BOOK, {"page": "1"})

        with pytest.raises(SystemExit, match="must hold a list of pages"):
            import_book.read_pages(BOOK)

    def test_invalid_json_is_reported(self, books_dir):
        books_dir(BOOK, "{")

        with pytest.raises(json.JSONDecodeError):
            import_book.read_pages(BOOK)


class TestMain:
    """Running the import end to end."""

    def test_the_named_book_is_imported(self, fake_db, books_dir, capsys):
        books_dir(BOOK, [{"page": "1", "text": ["Un."]}, {"page": "2"}])

        import_book.main([BOOK])

        assert sorted(load_story(BOOK)) == ["1", "2"]
        assert "Imported 2 pages" in capsys.readouterr().out

    def test_the_database_is_named_in_the_report(self, fake_db, books_dir, capsys):
        books_dir(BOOK, [{"page": "1"}])

        import_book.main([BOOK])

        assert "haute_tension_dev" in capsys.readouterr().out

    def test_the_default_book_is_imported_without_arguments(
        self, fake_db, books_dir, capsys
    ):
        books_dir(import_book.DEFAULT_BOOK, [{"page": "1"}])

        import_book.main([])

        assert sorted(load_story(import_book.DEFAULT_BOOK)) == ["1"]

    def test_a_trailing_slash_is_tolerated(self, fake_db, books_dir):
        books_dir(BOOK, [{"page": "1"}])

        import_book.main([f"/{BOOK}/"])

        assert sorted(load_story(BOOK)) == ["1"]

    def test_running_again_replaces_the_book(self, fake_db, books_dir):
        books_dir(BOOK, [{"page": "1"}, {"page": "2"}])
        import_book.main([BOOK])

        books_dir(BOOK, [{"page": "1"}])
        import_book.main([BOOK])

        assert sorted(load_story(BOOK)) == ["1"]

    def test_a_missing_uri_is_refused(self, fake_db, books_dir, monkeypatch):
        monkeypatch.delenv("MONGO_URI", raising=False)

        with pytest.raises(EnvironmentError, match="MONGO_URI"):
            import_book.main([BOOK])
