import json
import tempfile
import unittest
from pathlib import Path

from haute_tension.application.page_history import (
    get_oldest_page,
    update_last_pages,
)


class PageHistoryTestCase(unittest.TestCase):
    """Tests for persistent recent-page history."""

    def setUp(self) -> None:
        """Create an isolated history path for each test."""
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.history_path = Path(self.temporary_directory.name) / "last_pages.json"

    def tearDown(self) -> None:
        """Remove the isolated test directory."""
        self.temporary_directory.cleanup()

    def test_missing_history_starts_at_first_page(self) -> None:
        """Return the opening page when no history has been persisted."""
        self.assertEqual(get_oldest_page(self.history_path), "1")

    def test_empty_history_has_no_oldest_page(self) -> None:
        """Return no page when the persisted history is empty."""
        self.history_path.write_text("[]", encoding="utf-8")

        self.assertIsNone(get_oldest_page(self.history_path))

    def test_update_retains_only_ten_most_recent_pages(self) -> None:
        """Discard old page numbers once the history limit is exceeded."""
        for page_number in range(1, 12):
            update_last_pages(str(page_number), self.history_path)

        history = json.loads(self.history_path.read_text(encoding="utf-8"))
        self.assertEqual(history, [str(page) for page in range(2, 12)])

    def test_oldest_page_is_the_first_retained_entry(self) -> None:
        """Return the first page kept by the bounded history."""
        self.history_path.write_text('["4", "7"]', encoding="utf-8")

        self.assertEqual(get_oldest_page(self.history_path), "4")

    def test_update_returns_the_bounded_history(self) -> None:
        """Return the history that was just persisted."""
        history = update_last_pages("1", self.history_path)

        self.assertEqual(history, ["1"])
        self.assertEqual(
            json.loads(self.history_path.read_text(encoding="utf-8")),
            ["1"],
        )

    def test_update_appends_to_an_existing_history(self) -> None:
        """Keep previously persisted pages when appending a new one."""
        self.history_path.write_text('["1"]', encoding="utf-8")

        self.assertEqual(update_last_pages("2", self.history_path), ["1", "2"])

    def test_non_list_history_is_rejected(self) -> None:
        """Raise when the persisted history is not a list."""
        self.history_path.write_text('{"page": "1"}', encoding="utf-8")

        with self.assertRaises(ValueError):
            get_oldest_page(self.history_path)

    def test_history_of_non_strings_is_rejected(self) -> None:
        """Raise when the persisted history holds non-string page numbers."""
        self.history_path.write_text("[1]", encoding="utf-8")

        with self.assertRaises(ValueError):
            get_oldest_page(self.history_path)

    def test_invalid_json_history_is_rejected(self) -> None:
        """Raise when the persisted history is not valid JSON."""
        self.history_path.write_text("[", encoding="utf-8")

        with self.assertRaises(json.JSONDecodeError):
            get_oldest_page(self.history_path)


if __name__ == "__main__":
    unittest.main()
