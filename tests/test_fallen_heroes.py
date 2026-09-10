"""Heroes who did not come back: how they are remembered, and where."""

import random
from datetime import datetime, timezone

from haute_tension.core.db import (
    apply_pending,
    begin_combat,
    fallen_heroes,
    find_game,
    follow_choice,
    play_assault,
    start_game,
)
from tests.test_core_dice import FixedDice

BOOK = "pretre_jean/forteresse_alamuth"
OTHER_BOOK = "pretre_jean/autre_livre"

DEADLY_FIGHT = {
    "fight_type": "single",
    "enemies": [{"name": "collecteur d'impots", "force": 60, "vie": 10}],
    "outcome": {"on_victory": "621", "on_defeat": "death"},
}
FATAL_CHOICE = {
    "goto": "2",
    "gains": [],
    "losses": [{"element": "life point", "label_fr": "point de Vie", "amount": 99}],
}


def weak_hero(fake_db):
    """A hero rolled at the bottom of the range, easy to kill."""
    return start_game(BOOK, rng=FixedDice(1, 1, 1, 1, 1, 1, 1, 1))


class TestFallingInCombat:
    """The stone is cut when Vie reaches zero."""

    def test_a_hero_who_loses_is_laid_to_rest(self, fake_db):
        game = weak_hero(fake_db)
        begin_combat(game["id"], "22", DEADLY_FIGHT)
        while find_game(game["id"])["vie_actuelle"] > 0:
            play_assault(game["id"], random.Random(3))

        assert find_game(game["id"])["is_dead"]

    def test_he_is_remembered_where_he_fell(self, fake_db):
        game = weak_hero(fake_db)
        begin_combat(game["id"], "22", DEADLY_FIGHT)
        while find_game(game["id"])["vie_actuelle"] > 0:
            play_assault(game["id"], random.Random(3))

        assert find_game(game["id"])["died_on_page"] == "22"

    def test_what_killed_him_is_named(self, fake_db):
        game = weak_hero(fake_db)
        begin_combat(game["id"], "22", DEADLY_FIGHT)
        while find_game(game["id"])["vie_actuelle"] > 0:
            play_assault(game["id"], random.Random(3))

        assert find_game(game["id"])["died_of"] == "collecteur d'impots"

    def test_a_living_hero_is_not_among_them(self, fake_db):
        start_game(BOOK, rng=random.Random(1))

        assert fallen_heroes(BOOK) == []


class TestFallingOnTheRoad:
    """A hero can die of a paragraph as well as of an adversary."""

    def test_a_fatal_loss_lays_him_to_rest(self, fake_db):
        game = weak_hero(fake_db)

        after = follow_choice(game["id"], "63", FATAL_CHOICE)

        assert after["vie_actuelle"] == 0
        assert after["is_dead"]
        assert after["died_on_page"] == "63"

    def test_the_cause_says_it_was_the_road(self, fake_db):
        game = weak_hero(fake_db)
        follow_choice(game["id"], "63", FATAL_CHOICE)

        assert find_game(game["id"])["died_of"] == "les épreuves du chemin"

    def test_a_change_the_reader_applies_can_kill_too(self, fake_db):
        game = weak_hero(fake_db)
        fatal = {
            "goto": "2",
            "gains": [],
            "losses": [
                {
                    "element": "life point",
                    "label_fr": "point de Vie",
                    "amount": 99,
                    "condition": "Si vous n'avez pas mangé",
                }
            ],
        }
        follow_choice(game["id"], "63", fatal)
        assert not find_game(game["id"])["is_dead"]

        apply_pending(game["id"], 0)

        assert find_game(game["id"])["is_dead"]

    def test_the_first_death_is_the_one_that_counts(self, fake_db):
        """Nothing afterwards should move the date on the stone."""
        game = weak_hero(fake_db)
        follow_choice(game["id"], "63", FATAL_CHOICE)
        first = find_game(game["id"])

        follow_choice(game["id"], "999", FATAL_CHOICE)

        assert find_game(game["id"])["died_on_page"] == first["died_on_page"] == "63"


class TestTheMemorial:
    """What the list holds."""

    def test_the_fallen_are_listed(self, fake_db):
        game = weak_hero(fake_db)
        follow_choice(game["id"], "63", FATAL_CHOICE)

        [epitaph] = fallen_heroes(BOOK)

        assert epitaph["id"] == game["id"]
        assert epitaph["died_on_page"] == "63"
        assert epitaph["force"] == game["force"]
        assert epitaph["vie_max"] == game["vie_max"]

    def test_what_he_carried_is_remembered(self, fake_db):
        game = weak_hero(fake_db)
        follow_choice(game["id"], "63", FATAL_CHOICE)

        [epitaph] = fallen_heroes(BOOK)

        assert {item["element"] for item in epitaph["bag"]} == {
            "sword",
            "bag",
            "ration",
        }

    def test_his_difficulty_is_remembered(self, fake_db):
        from haute_tension.core.character import EASY

        game = start_game(BOOK, mode=EASY, rng=random.Random(1))
        follow_choice(game["id"], "63", FATAL_CHOICE)

        assert fallen_heroes(BOOK)[0]["mode_label"] == "Facile"

    def test_the_most_recent_comes_first(self, fake_db):
        """The times are set here: two deaths a millisecond apart have no order."""
        first = weak_hero(fake_db)
        follow_choice(first["id"], "10", FATAL_CHOICE)
        second = weak_hero(fake_db)
        follow_choice(second["id"], "20", FATAL_CHOICE)

        buried = {
            "10": datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc),
            "20": datetime(2026, 9, 10, 13, 0, tzinfo=timezone.utc),
        }
        fake_db["games"].docs = [
            {**doc, "died_at": buried[doc["died_on_page"]]}
            for doc in fake_db["games"].docs
        ]

        assert [e["died_on_page"] for e in fallen_heroes(BOOK)] == ["20", "10"]

    def test_only_this_book_is_remembered(self, fake_db):
        here = weak_hero(fake_db)
        follow_choice(here["id"], "10", FATAL_CHOICE)
        elsewhere = start_game(OTHER_BOOK, rng=random.Random(1))
        follow_choice(elsewhere["id"], "10", FATAL_CHOICE)

        assert [e["id"] for e in fallen_heroes(BOOK)] == [here["id"]]

    def test_the_list_is_bounded(self, fake_db):
        for number in range(4):
            game = weak_hero(fake_db)
            follow_choice(game["id"], str(number), FATAL_CHOICE)

        assert len(fallen_heroes(BOOK, limit=2)) == 2

    def test_an_abandoned_hero_in_good_health_is_not_among_them(self, fake_db):
        """Being left behind is not dying."""
        start_game(BOOK, rng=random.Random(1))
        start_game(BOOK, rng=random.Random(2))

        assert fallen_heroes(BOOK) == []


class TestTheMemorialPage:
    """Reading it in the browser."""

    def test_it_says_when_nobody_has_fallen(self, hero):
        body = hero.get("/heroes").get_data(as_text=True)

        assert "Personne n'est encore tombé" in body

    def test_a_fallen_hero_is_shown(self, hero, fake_db):
        game = fake_db["games"].docs[0]
        game["vie_actuelle"] = 0
        game["died_at"] = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)
        game["died_on_page"] = "22"
        game["died_of"] = "collecteur d'impots"
        fake_db["games"].docs = [game]

        body = hero.get("/heroes").get_data(as_text=True)

        assert "Paragraphe 22" in body
        assert "collecteur d&#39;impots" in body

    def test_what_he_carried_is_shown(self, hero, fake_db):
        game = fake_db["games"].docs[0]
        game["vie_actuelle"] = 0
        game["died_at"] = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)
        fake_db["games"].docs = [game]

        body = hero.get("/heroes").get_data(as_text=True)

        assert "épée" in body
        assert "ration" in body

    def test_it_is_reachable_from_every_page(self, hero):
        for path in ("/", "/book/1", "/game", "/heroes"):
            assert 'href="/heroes"' in hero.get(path).get_data(as_text=True)

    def test_a_reader_without_a_hero_may_still_read_it(self, fighting_client):
        response = fighting_client.get("/heroes")

        assert response.status_code == 200
        assert "Créer un héros" in response.get_data(as_text=True)

    def test_the_dead_hero_sheet_says_so(self, hero, fake_db):
        game = fake_db["games"].docs[0]
        game["vie_actuelle"] = 0
        game["died_at"] = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)
        game["died_on_page"] = "22"
        game["died_of"] = "un troll"
        fake_db["games"].docs = [game]

        body = hero.get("/game").get_data(as_text=True)

        assert "Ce héros est mort" in body
        assert "au paragraphe 22" in body
        assert "Repartir de zéro" in body
        assert "Abandonner et créer" not in body
