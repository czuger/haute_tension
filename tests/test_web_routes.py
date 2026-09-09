"""Tests for the landing page and book navigation."""

import html


class TestLandingPage:
    """The one book on offer."""

    def test_the_current_book_is_shown(self, client):
        response = client.get("/")

        assert response.status_code == 200
        assert "La Forteresse d'Alamuth" in html.unescape(
            response.get_data(as_text=True)
        )

    def test_it_links_to_the_opening_page(self, client):
        assert 'href="/book/1"' in client.get("/").get_data(as_text=True)


class TestReader:
    """Reading a page and moving on from it."""

    def test_choices_come_before_the_story_text(self, client):
        body = client.get("/book/1").get_data(as_text=True)

        assert body.index('href="/book/2"') < body.index("Le début de l&#39;aventure.")

    def test_the_story_text_is_rendered(self, client):
        body = client.get("/book/1").get_data(as_text=True)

        assert "Le début de l&#39;aventure." in body
        assert "Deuxième ligne." in body

    def test_a_terminal_page_offers_to_restart(self, client):
        body = client.get("/book/2").get_data(as_text=True)

        assert "Fin de l'aventure" in body
        assert 'href="/book/1"' in body

    def test_an_unknown_page_returns_not_found(self, client):
        assert client.get("/book/999").status_code == 404

    def test_reading_a_page_is_not_recorded(self, client, fake_db):
        client.get("/book/1")

        assert fake_db["page_views"].docs == []
