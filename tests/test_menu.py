"""The site menu: Nouveau, Partie en cours, Feuille, Les tombés."""

from datetime import datetime, timezone

import pytest

from haute_tension.application.french_date import french_date
from haute_tension.application.game_routes import SESSION_KEY


def text_of(response):
    """The response body as text."""
    return response.get_data(as_text=True)


def bury(fake_db, page="22", cause="un troll"):
    """Lay the one stored hero to rest, in place."""
    game = fake_db["games"].docs[0]
    game["vie_actuelle"] = 0
    game["died_at"] = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)
    game["died_on_page"] = page
    game["died_of"] = cause
    fake_db["games"].docs = [game]


class TestTheMenu:
    """Four entries, on every page, hero or not."""

    @pytest.mark.parametrize("path", ["/", "/book/1", "/game", "/heroes", "/game/new"])
    def test_every_page_carries_the_four_entries(self, hero, path):
        body = text_of(hero.get(path))

        assert 'href="/game/new">Nouveau<' in body
        assert 'href="/game/resume">Partie en cours<' in body
        assert 'href="/game">Feuille<' in body
        assert 'href="/heroes">Les tombés<' in body

    def test_a_reader_without_a_hero_has_the_same_menu(self, client):
        body = text_of(client.get("/"))

        assert 'href="/game/new">Nouveau<' in body
        assert 'href="/game/resume">Partie en cours<' in body


class TestNouveau:
    """Offering a hero, and saying what it costs."""

    def test_it_offers_the_dice(self, fighting_client):
        response = fighting_client.get("/game/new")

        assert response.status_code == 200
        assert "Lancer les dés" in text_of(response)
        assert 'value="normal"' in text_of(response)
        assert 'value="easy"' in text_of(response)

    def test_it_rolls_nothing_by_itself(self, fighting_client, fake_db):
        fighting_client.get("/game/new")

        assert fake_db["games"].docs == []

    def test_without_a_hero_there_is_nothing_to_warn_about(self, fighting_client):
        assert "déjà en route" not in text_of(fighting_client.get("/game/new"))

    def test_a_living_hero_is_shown_before_being_abandoned(self, hero, fake_db):
        stored = fake_db["games"].docs[0]
        body = text_of(hero.get("/game/new"))

        assert "Un héros est déjà en route" in body
        assert f"Force {stored['force']}" in body
        assert f"Vie {stored['vie_actuelle']}/{stored['vie_max']}" in body
        assert 'href="/game/resume"' in body

    def test_a_hero_in_a_fight_is_said_to_be(self, hero):
        hero.get("/combat/1")

        assert "aux prises au paragraphe 1" in text_of(hero.get("/game/new"))

    def test_the_offer_leaves_the_living_hero_untouched(self, hero, fake_db):
        before = fake_db["games"].docs
        hero.get("/game/new")

        assert fake_db["games"].docs == before

    def test_a_dead_hero_is_told_of_instead(self, hero, fake_db):
        bury(fake_db)
        body = text_of(hero.get("/game/new"))

        assert "Votre héros est mort au paragraphe 22" in body
        assert "déjà en route" not in body

    def test_rolling_from_it_replaces_the_hero(self, hero, fake_db):
        with hero.session_transaction() as session:
            first = session[SESSION_KEY]
        hero.post("/game/new", data={"mode": "easy"})

        with hero.session_transaction() as session:
            assert session[SESSION_KEY] != first
        assert len(fake_db["games"].docs) == 2

    def test_the_abandoned_hero_is_kept_but_not_among_the_fallen(self, hero, fake_db):
        hero.post("/game/new")

        assert len(fake_db["games"].docs) == 2
        assert "Personne n'est encore tombé" in text_of(hero.get("/heroes"))

    def test_the_sheet_links_to_it_rather_than_rolling(self, hero):
        body = text_of(hero.get("/game"))

        assert 'href="/game/new">Abandonner et créer un nouveau héros' in body
        assert 'action="/game/new"' not in body


class TestPartieEnCours:
    """Back to wherever the adventure stands."""

    def test_without_a_hero_it_offers_one(self, fighting_client):
        response = fighting_client.get("/game/resume")

        assert response.status_code == 302
        assert response.headers["Location"].endswith("/game/new")

    def test_a_hero_who_has_read_nothing_starts_at_the_opening(self, hero):
        response = hero.get("/game/resume")

        assert response.headers["Location"].endswith("/book/1")

    def test_it_goes_back_to_the_last_page_read(self, hero):
        hero.get("/book/2")
        hero.get("/book/4")

        assert hero.get("/game/resume").headers["Location"].endswith("/book/4")

    def test_it_goes_back_into_an_open_fight(self, hero):
        hero.get("/book/2")
        hero.get("/combat/2")

        assert hero.get("/game/resume").headers["Location"].endswith("/combat/2")

    def test_a_dead_hero_is_sent_to_the_death_page(self, hero, fake_db):
        hero.get("/book/2")
        bury(fake_db)

        assert hero.get("/game/resume").headers["Location"].endswith("/game/death")

    def test_it_records_no_visit_of_its_own(self, hero, fake_db):
        hero.get("/book/2")
        hero.get("/game/resume")

        assert [view["page"] for view in fake_db["page_views"].docs] == ["2"]

    def test_the_page_it_lands_on_is_the_hero_s(self, hero, fake_db):
        """Another hero's last page is not this one's."""
        hero.get("/book/2")
        hero.post("/game/new")

        assert hero.get("/game/resume").headers["Location"].endswith("/book/1")

    def test_a_stale_session_offers_a_hero(self, fighting_client):
        with fighting_client.session_transaction() as session:
            session[SESSION_KEY] = "gone"

        response = fighting_client.get("/game/resume")

        assert response.headers["Location"].endswith("/game/new")


class TestFeuille:
    """The sheet reads everything and changes nothing."""

    def test_the_changes_waiting_on_the_reader_are_listed(self, hero):
        hero.post("/book/1/choice/1")
        body = text_of(hero.get("/game"))

        assert "À vous de voir" in body
        assert "1 effet en attente" in body
        assert "point de Force" in body
        assert "pendant tout le temps où vous les porterez" in body
        assert 'href="/book/1">paragraphe 1<' in body

    def test_without_any_the_section_is_absent(self, hero):
        assert "À vous de voir" not in text_of(hero.get("/game"))

    def test_the_sheet_posts_nothing(self, hero):
        assert "<form" not in text_of(hero.get("/game"))

    def test_the_sheet_never_changes_the_hero(self, hero, fake_db):
        hero.post("/book/1/choice/1")
        before = fake_db["games"].docs

        hero.get("/game")

        assert fake_db["games"].docs == before


class TestLesTombes:
    """The memorial, dated."""

    def test_the_day_he_fell_is_shown(self, hero, fake_db):
        bury(fake_db)

        assert "le 10 septembre 2026" in text_of(hero.get("/heroes"))

    def test_without_a_fallen_hero_it_says_so(self, hero):
        assert "Personne n'est encore tombé" in text_of(hero.get("/heroes"))


class TestFrenchDate:
    """The filter behind the memorial's dates."""

    def test_an_instant_reads_as_a_french_date(self):
        assert french_date("2026-09-10T12:00:00+00:00") == "10 septembre 2026"

    def test_the_first_of_a_month(self):
        assert french_date("2025-01-01T00:00:00+00:00") == "1 janvier 2025"

    def test_nothing_reads_as_nothing(self):
        assert french_date(None) == ""
