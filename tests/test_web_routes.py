import html
import json
import tempfile
import unittest
from pathlib import Path

from flask.testing import FlaskClient

from haute_tension.application.factory import create_app


class WebRoutesTestCase(unittest.TestCase):
    """Tests for the landing page and book navigation."""

    def setUp(self) -> None:
        """Create a small navigable book in an isolated directory."""
        self.temporary_directory = tempfile.TemporaryDirectory()
        temporary_path = Path(self.temporary_directory.name)
        book_path = temporary_path / "books" / "series" / "book"
        book_path.mkdir(parents=True)
        (book_path / "merged_pages.json").write_text(
            json.dumps(
                [
                    {
                        "page": "1",
                        "language": "fr",
                        "text": ["Le début de l'aventure."],
                        "choices": [
                            {"goto": "2", "gains": [], "losses": []}
                        ],
                        "file_path": "raw_data/1.html",
                    },
                    {
                        "page": "2",
                        "language": "fr",
                        "text": ["La fin de l'aventure."],
                        "choices": [],
                        "file_path": "raw_data/2.html",
                    },
                ]
            ),
            encoding="utf-8",
        )
        app = create_app(book_path, temporary_path / "last_pages.json")
        app.config.update(TESTING=True)
        self.client: FlaskClient = app.test_client()

    def tearDown(self) -> None:
        """Remove the isolated test directory."""
        self.temporary_directory.cleanup()

    def test_landing_page_shows_the_current_book(self) -> None:
        """Present the only book and link to its opening page."""
        response = self.client.get("/")
        response_text = html.unescape(response.get_data(as_text=True))

        self.assertEqual(response.status_code, 200)
        self.assertIn("La Forteresse d'Alamuth", response_text)
        self.assertIn('href="/book/1"', response_text)

    def test_reader_links_to_the_next_page(self) -> None:
        """Render story text and a link for each available choice."""
        response = self.client.get("/book/1")
        response_text = response.get_data(as_text=True)

        self.assertEqual(response.status_code, 200)
        self.assertIn("Le début de l&#39;aventure.", response_text)
        self.assertIn('href="/book/2"', response_text)
        self.assertLess(
            response_text.index('href="/book/2"'),
            response_text.index("Le début de l&#39;aventure."),
        )

    def test_terminal_page_offers_to_restart(self) -> None:
        """Show a restart action when a page has no choices."""
        response = self.client.get("/book/2")
        response_text = response.get_data(as_text=True)

        self.assertEqual(response.status_code, 200)
        self.assertIn("Fin de l'aventure", response_text)
        self.assertIn('href="/book/1"', response_text)

    def test_unknown_page_returns_not_found(self) -> None:
        """Return HTTP 404 when a story page does not exist."""
        response = self.client.get("/book/999")

        self.assertEqual(response.status_code, 404)


if __name__ == "__main__":
    unittest.main()
