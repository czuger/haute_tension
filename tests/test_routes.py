import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from flask.testing import FlaskClient

from haute_tension.application.factory import create_app


class RoutesTestCase(unittest.TestCase):
    """Tests for the story data and audio routes."""

    def setUp(self) -> None:
        """Create an app backed by isolated story and history files."""
        self.temporary_directory = tempfile.TemporaryDirectory()
        temporary_path = Path(self.temporary_directory.name)
        self.book_path = temporary_path / "books" / "series" / "book"
        self.history_path = temporary_path / "last_pages.json"
        self.audio_path = temporary_path / "speech.mp3"
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
        self.audio_path.write_bytes(b"audio-data")
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

    def test_audio_route_generates_and_returns_audio(self) -> None:
        """Generate narration and return its cached MP3 file."""
        with (
            patch("haute_tension.application.routes.speak_french_text") as speak,
            patch(
                "haute_tension.application.routes.get_text_hash",
                return_value="text-hash",
            ),
            patch(
                "haute_tension.application.routes.get_audio_filename",
                return_value=self.audio_path,
            ),
        ):
            response = self.client.get("/audio/1")

        try:
            self.assertEqual(response.status_code, 200)
            self.assertEqual(response.mimetype, "audio/mpeg")
            self.assertEqual(response.data, b"audio-data")
            speak.assert_called_once_with("Première ligne.\nDeuxième ligne.")
        finally:
            response.close()

    def test_audio_route_reports_an_unknown_page(self) -> None:
        """Return a UTF-8 JSON error when an audio page is unknown."""
        response = self.client.get("/audio/404")

        self.assertEqual(response.status_code, 200)
        self.assertEqual(
            response.get_json(),
            {
                "success": False,
                "message": "le numéro 404 n'a pas été trouvé",
            },
        )


if __name__ == "__main__":
    unittest.main()
