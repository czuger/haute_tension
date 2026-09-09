"""Tests for the story data route."""

import json

BOOK = "pretre_jean/forteresse_alamuth"


class TestDataRoute:
    """Serving one page as JSON, and noting that it was asked for."""

    def test_a_page_is_returned_with_its_choices(self, client):
        response = client.get("/data/1")

        assert response.status_code == 200
        assert [choice["goto"] for choice in response.get_json()["choices"]] == ["2"]

    def test_the_page_text_is_returned(self, client):
        assert response_json(client, "/data/1")["text"] == [
            "Le début de l'aventure.",
            "Deuxième ligne.",
        ]

    def test_a_requested_page_is_recorded(self, client, fake_db):
        client.get("/data/1")

        views = fake_db["page_views"].docs
        assert len(views) == 1
        assert views[0]["book"] == BOOK
        assert views[0]["page"] == "1"

    def test_an_unknown_page_is_recorded_too(self, client, fake_db):
        client.get("/data/404")

        assert [view["page"] for view in fake_db["page_views"].docs] == ["404"]

    def test_an_unknown_page_is_reported(self, client):
        response = client.get("/data/404")

        assert response.status_code == 200
        assert response.mimetype == "application/json"
        assert response.get_json() == {
            "success": False,
            "message": "le numéro 404 n'a pas été trouvé",
        }

    def test_the_error_keeps_accented_characters(self, client):
        assert "numéro" in client.get("/data/404").get_data(as_text=True)


def response_json(client, path):
    """The parsed body of a successful request."""
    response = client.get(path)
    assert response.status_code == 200
    return json.loads(response.get_data(as_text=True))
