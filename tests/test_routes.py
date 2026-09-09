import json
import tempfile
import unittest
from pathlib import Path

from flask.testing import FlaskClient

from haute_tension.application.factory import create_app


class RoutesTestCase(unittest.TestCase):
    """Tests for the story data route."""

    def setUp(self) -> None:
        """Create an app backed by isolated story and history files."""
        self.temporary_directory = tempfile.TemporaryDirectory()
        temporary_path = Path(self.temporary_directory.name)
        self.book_path = temporary_path / "books" / "series" / "book"
        self.history_path = temporary_path / "last_pages.json"
        self.book_path.mkdir(parents=True)
        (self.book_path / "merged_pages.json").write_text(
            json.dumps(
                [
                    {
                        "page": "1",
                        "language": "fr",
                        "text": ["Première ligne.", "Deuxième ligne."],
                        "choices": [
                            {"goto": "3", "gains": [], "losses": []},
                            {"goto": "12", "gains": [], "losses": []},
                        ],
                        "file_path": "raw_data/1.html",
                    }
                ]
            ),
            encoding="utf-8",
        )
        app = create_app(self.book_path, self.history_path)
        app.config.update(TESTING=True)
        self.client: FlaskClient = app.test_client()

    def tearDown(self) -> None:
        """Remove the isolated test directory."""
        self.temporary_directory.cleanup()

    def test_data_route_returns_page_choices(self) -> None:
        """Return story data with its linked choices."""
        response = self.client.get("/data/1")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(
            [choice["goto"] for choice in response.get_json()["choices"]],
            ["3", "12"],
        )

    def test_data_route_records_the_requested_page(self) -> None:
        """Append every requested page number to the persisted history."""
        self.client.get("/data/1")
        self.client.get("/data/404")

        history = json.loads(self.history_path.read_text(encoding="utf-8"))
        self.assertEqual(history, ["1", "404"])

    def test_data_route_reports_an_unknown_page(self) -> None:
        """Return a UTF-8 JSON error when a page is unknown."""
        response = self.client.get("/data/404")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(response.mimetype, "application/json")
        self.assertEqual(
            response.get_json(),
            {
                "success": False,
                "message": "le numéro 404 n'a pas été trouvé",
            },
        )

    def test_unknown_page_message_keeps_accented_characters(self) -> None:
        """Serialize the error message without escaping non-ASCII text."""
        response = self.client.get("/data/404")

        self.assertIn("numéro", response.get_data(as_text=True))


if __name__ == "__main__":
    unittest.main()
