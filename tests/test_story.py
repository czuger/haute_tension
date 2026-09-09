import json
import tempfile
import unittest
from pathlib import Path

from haute_tension.core.story import PAGES_FILE, load_story


class LoadStoryTestCase(unittest.TestCase):
    """Tests for reading and indexing a book's pages."""

    def setUp(self) -> None:
        """Create an isolated book directory for each test."""
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.book_path = Path(self.temporary_directory.name)
        self.pages_path = self.book_path / PAGES_FILE

    def tearDown(self) -> None:
        """Remove the isolated test directory."""
        self.temporary_directory.cleanup()

    def _write_pages(self, pages: object) -> None:
        """Persist merged-pages content for the book under test.

        Args:
            pages: Value serialized as the book's `pages.json` content.
        """
        self.pages_path.write_text(json.dumps(pages), encoding="utf-8")

    def test_pages_are_indexed_by_page_number(self) -> None:
        """Return every page keyed by its own page number."""
        self._write_pages(
            [
                {"page": "1", "text": ["Un."]},
                {"page": "2", "text": ["Deux."]},
            ]
        )

        story_data = load_story(self.book_path)

        self.assertEqual(sorted(story_data), ["1", "2"])
        self.assertEqual(story_data["2"]["text"], ["Deux."])

    def test_missing_book_file_is_reported(self) -> None:
        """Raise when the merged-pages file does not exist."""
        with self.assertRaises(FileNotFoundError):
            load_story(self.book_path)

    def test_invalid_json_is_reported(self) -> None:
        """Raise when the merged-pages file is not valid JSON."""
        self.pages_path.write_text("{", encoding="utf-8")

        with self.assertRaises(json.JSONDecodeError):
            load_story(self.book_path)

    def test_non_list_book_is_rejected(self) -> None:
        """Raise when the merged-pages document is not a list."""
        self._write_pages({"page": "1"})

        with self.assertRaises(ValueError):
            load_story(self.book_path)

    def test_non_object_page_is_rejected(self) -> None:
        """Raise when an entry in the merged-pages list is not an object."""
        self._write_pages(["1"])

        with self.assertRaises(ValueError):
            load_story(self.book_path)

    def test_page_without_string_number_is_rejected(self) -> None:
        """Raise when a page number is missing or not a string."""
        self._write_pages([{"page": 1}])

        with self.assertRaises(ValueError):
            load_story(self.book_path)

    def test_duplicate_page_number_is_rejected(self) -> None:
        """Raise when two pages share the same page number."""
        self._write_pages([{"page": "1"}, {"page": "1"}])

        with self.assertRaises(ValueError):
            load_story(self.book_path)

    def test_empty_book_loads_no_pages(self) -> None:
        """Return an empty index for a book without any page."""
        self._write_pages([])

        self.assertEqual(load_story(self.book_path), {})


if __name__ == "__main__":
    unittest.main()
