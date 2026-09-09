"""Tests for rolling up a hero and fighting with him through the browser."""

import re

from haute_tension.application.game_routes import SESSION_KEY


def text_of(response):
    """The response body as text."""
    return response.get_data(as_text=True)


def combat_status(client, page="1"):
    """Whether the fight on a page is still open, won or lost."""
    body = text_of(client.get(f"/combat/{page}"))
    if "Vous l'emportez" in body:
        return "victory"
    if "Vous êtes vaincu" in body:
        return "defeat"
    return "ongoing"


def fight_to_the_end(client, page="1", limit=200):
    """Play assaults until the fight is decided, and say how it went."""
    for _ in range(limit):
        status = combat_status(client, page)
        if status != "ongoing":
            return status
        client.post(f"/combat/{page}/assault")
    raise AssertionError("the fight never ended")


class TestCreatingAHero:
    """Rolling up Prêtre Jean."""

    def test_without_a_game_the_reader_is_offered_one(self, fighting_client):
        response = fighting_client.get("/game")

        assert response.status_code == 200
        assert "Lancer les dés" in text_of(response)

    def test_rolling_up_creates_a_game(self, fighting_client, fake_db):
        fighting_client.post("/game/new")

        assert len(fake_db["games"].docs) == 1

    def test_the_game_id_goes_into_the_session(self, fighting_client):
        fighting_client.post("/game/new")

        with fighting_client.session_transaction() as session:
            assert session[SESSION_KEY]

    def test_the_sheet_shows_its_working(self, hero):
        body = text_of(hero.get("/game"))

        assert re.search(r"\d \+ \d \(2D6\)\s*\+ 6 = \d+", body)
        assert re.search(r"\d \+ \d \(2D6\)\s*\+ 18 = \d+", body)

    def test_the_sheet_shows_force_and_vie(self, hero, fake_db):
        stored = fake_db["games"].docs[0]
        body = text_of(hero.get("/game"))

        assert str(stored["force"]) in body
        assert f'{stored["vie_actuelle"]}</strong> / {stored["vie_max"]}' in body

    def test_reloading_the_sheet_never_re_rolls(self, hero, fake_db):
        """The dice are thrown once; every later view loads what was written."""
        first = fake_db["games"].docs[0]

        for _ in range(4):
            hero.get("/game")

        assert fake_db["games"].docs == [first]

    def test_a_stale_session_falls_back_to_rolling_up(self, fighting_client):
        with fighting_client.session_transaction() as session:
            session[SESSION_KEY] = "a-game-that-was-deleted"

        assert "Lancer les dés" in text_of(fighting_client.get("/game"))

    def test_a_stale_session_is_cleared(self, fighting_client):
        with fighting_client.session_transaction() as session:
            session[SESSION_KEY] = "gone"
        fighting_client.get("/game")

        with fighting_client.session_transaction() as session:
            assert SESSION_KEY not in session

    def test_both_difficulties_are_offered(self, fighting_client):
        body = text_of(fighting_client.get("/game"))

        assert "Difficulté normale" in body
        assert "Difficulté facile" in body

    def test_each_difficulty_shows_its_dice_and_range(self, fighting_client):
        body = text_of(fighting_client.get("/game"))

        assert "Force 6 + 2D6" in body
        assert "Force 12 + 2D4" in body
        assert "Vie 26 + 3D4" in body
        assert "29–38" in body

    def test_choosing_the_easy_mode_rolls_an_easy_hero(
        self, fighting_client, fake_db
    ):
        fighting_client.post("/game/new", data={"mode": "easy"})

        stored = fake_db["games"].docs[0]
        assert stored["mode"] == "easy"
        assert 14 <= stored["force"] <= 20
        assert 29 <= stored["vie_max"] <= 38
        assert len(stored["vie_dice"]) == 3

    def test_choosing_nothing_rolls_by_the_book(self, fighting_client, fake_db):
        fighting_client.post("/game/new")

        stored = fake_db["games"].docs[0]
        assert stored["mode"] == "normal"
        assert 8 <= stored["force"] <= 18

    def test_an_unknown_mode_rolls_by_the_book(self, fighting_client, fake_db):
        """A posted value is whatever was posted; it must still roll a hero."""
        response = fighting_client.post("/game/new", data={"mode": "invincible"})

        assert response.status_code == 302
        assert fake_db["games"].docs[0]["mode"] == "normal"

    def test_the_easy_sheet_prints_its_own_throw(self, fighting_client):
        fighting_client.post("/game/new", data={"mode": "easy"})
        body = text_of(fighting_client.get("/game"))

        assert "Difficulté facile" in body
        assert re.search(r"\d \+ \d \(2D4\)\s*\+ 12 = \d+", body)
        assert re.search(r"\d \+ \d \+ \d \(3D4\)\s*\+ 26 = \d+", body)

    def test_a_new_hero_replaces_the_old_one(self, hero):
        with hero.session_transaction() as session:
            first = session[SESSION_KEY]
        hero.post("/game/new")

        with hero.session_transaction() as session:
            assert session[SESSION_KEY] != first


class TestReachingACombat:
    """Getting from a page into the fight it holds."""

    def test_a_page_with_a_fight_offers_it(self, hero):
        assert "Engager le combat" in text_of(hero.get("/book/1"))

    def test_a_page_without_one_does_not(self, client):
        assert "Engager le combat" not in text_of(client.get("/book/1"))

    def test_the_fight_opens(self, hero):
        response = hero.get("/combat/1")

        assert response.status_code == 200
        assert "collecteur" in text_of(response)

    def test_both_sides_are_shown(self, hero):
        body = text_of(hero.get("/combat/1"))

        assert "Prêtre Jean" in body
        assert "Force 6" in body

    def test_a_fight_that_fields_nobody_is_not_a_combat(self, hero):
        """The parser does emit fights whose adversaries are unusable."""
        assert hero.get("/combat/3").status_code == 404

    def test_an_assault_on_an_unfieldable_fight_changes_nothing(
        self, hero, fake_db
    ):
        """Page 3 looks like it holds a fight, but nothing in it can be fielded."""
        before = fake_db["games"].docs[0]
        response = hero.post("/combat/3/assault")

        assert response.status_code == 302
        assert fake_db["games"].docs[0] == before

    def test_an_assault_on_an_unknown_page_changes_nothing(self, hero, fake_db):
        before = fake_db["games"].docs[0]
        response = hero.post("/combat/999/assault")

        assert response.status_code == 302
        assert fake_db["games"].docs[0] == before

    def test_an_unknown_page_has_no_combat(self, hero):
        assert hero.get("/combat/999").status_code == 404

    def test_without_a_hero_the_reader_is_sent_to_roll_one(self, fighting_client):
        response = fighting_client.get("/combat/1")

        assert response.status_code == 302
        assert response.headers["Location"].endswith("/game")


class TestFightingAnAssault:
    """One assault per click."""

    def test_an_assault_is_logged(self, hero):
        hero.post("/combat/1/assault")
        body = text_of(hero.get("/combat/1"))

        assert "Journal des assauts" in body
        assert "Assaut 1" in body

    def test_the_assault_shows_its_dice_and_forces(self, hero):
        hero.post("/combat/1/assault")
        body = text_of(hero.get("/combat/1"))

        assert re.search(r"Assaut 1 .*? \d \+ \d \+ Force \d+", body, re.S)

    def test_assaults_accumulate(self, hero):
        for _ in range(2):
            if combat_status(hero) == "ongoing":
                hero.post("/combat/1/assault")
        body = text_of(hero.get("/combat/1"))

        assert "Assaut 1" in body

    def test_an_assault_costs_somebody_vie(self, hero, fake_db):
        before = fake_db["games"].docs[0]
        hero.post("/combat/1/assault")
        after = fake_db["games"].docs[0]

        hero_hurt = after["vie_actuelle"] < before["vie_actuelle"]
        enemy_hurt = (
            after["combat"]["enemies"][0]["vie_actuelle"]
            < after["combat"]["enemies"][0]["vie_max"]
        )
        assert hero_hurt or enemy_hurt

    def test_reloading_the_page_does_not_play_an_assault(self, hero, fake_db):
        hero.post("/combat/1/assault")
        after_one = fake_db["games"].docs[0]

        for _ in range(3):
            hero.get("/combat/1")

        assert fake_db["games"].docs[0] == after_one

    def test_the_fight_reaches_a_verdict(self, hero):
        assert fight_to_the_end(hero) in ("victory", "defeat")

    def test_a_decided_fight_offers_to_move_on(self, hero):
        fight_to_the_end(hero)

        assert "Poursuivre l'aventure" in text_of(hero.get("/combat/1"))

    def test_a_decided_fight_takes_no_more_assaults(self, hero, fake_db):
        fight_to_the_end(hero)
        decided = fake_db["games"].docs[0]
        hero.post("/combat/1/assault")

        assert fake_db["games"].docs[0] == decided

    def test_an_assault_without_a_hero_is_harmless(self, fighting_client, fake_db):
        response = fighting_client.post("/combat/1/assault")

        assert response.status_code == 302
        assert fake_db["games"].docs == []

    def test_an_assault_on_a_page_holding_no_fight_changes_nothing(
        self, hero, fake_db
    ):
        """Page 4 exists and holds no fight, so there is nothing to arm."""
        before = fake_db["games"].docs[0]
        response = hero.post("/combat/4/assault")

        assert response.status_code == 302
        assert fake_db["games"].docs[0] == before


class TestLeavingACombat:
    """Following the book once the fight is decided."""

    def test_a_victory_follows_the_book(self, hero, fake_db):
        if fight_to_the_end(hero) != "victory":
            return
        response = hero.post("/combat/1/resolve")

        assert response.headers["Location"] == "/book/2"

    def test_a_defeat_ends_the_adventure(self, hero, fake_db):
        """on_defeat is "death" here, so the hero is sent to the death page."""
        fake_db["games"].docs = []
        hero.post("/game/new")
        game = fake_db["games"].docs[0]
        game["vie_actuelle"] = 0
        game["combat"] = {
            "page": "1",
            "fight_type": "single",
            "status": "defeat",
            "on_victory": "2",
            "on_defeat": "death",
            "enemies": [
                {"name": "collecteur", "force": 6, "vie_max": 10, "vie_actuelle": 4}
            ],
            "assaults": [],
        }
        fake_db["games"].docs = [game]

        response = hero.post("/combat/1/resolve")

        assert response.headers["Location"].endswith("/game/death")

    def test_leaving_clears_the_fight(self, hero, fake_db):
        fight_to_the_end(hero)
        hero.post("/combat/1/resolve")

        assert fake_db["games"].docs[0].get("combat") is None

    def test_an_undecided_fight_goes_back_to_the_page(self, hero):
        response = hero.post("/combat/1/resolve")

        assert response.headers["Location"] == "/book/1"

    def test_an_undecided_fight_is_not_cleared(self, hero, fake_db):
        hero.post("/combat/1/assault")
        hero.post("/combat/1/resolve")

        assert fake_db["games"].docs[0].get("combat") is not None

    def test_resolving_without_a_hero_goes_back_to_the_page(self, fighting_client):
        response = fighting_client.post("/combat/1/resolve")

        assert response.headers["Location"] == "/book/1"

    def test_the_death_page_offers_a_new_hero(self, hero):
        body = text_of(hero.get("/game/death"))

        assert "Reprendre" in body
        assert 'value="normal"' in body
        assert 'value="easy"' in body


class TestSpecialRules:
    """Pages whose text adds a rule the engine does not model."""

    def test_a_flagged_page_warns_the_reader(self, hero, fake_db):
        game = fake_db["games"].docs[0]
        game["combat"] = {
            "page": "1",
            "fight_type": "simultaneous",
            "status": "ongoing",
            "has_special_rules": True,
            "enemies": [
                {"name": "lepreux", "force": 6, "vie_max": 6, "vie_actuelle": 6}
            ],
            "assaults": [],
        }
        fake_db["games"].docs = [game]

        assert "règle particulière" in text_of(hero.get("/combat/1"))

    def test_an_ordinary_page_does_not(self, hero):
        assert "règle particulière" not in text_of(hero.get("/combat/1"))
