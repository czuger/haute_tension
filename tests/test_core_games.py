"""Tests for reading and writing a play-through."""

import random

from haute_tension.core.character import EASY, NORMAL
from haute_tension.core.combat import DEFEAT, ONGOING, VICTORY
from haute_tension.core.db import (
    apply_pending,
    begin_combat,
    dismiss_pending,
    follow_choice,
    end_combat,
    find_game,
    play_assault,
    start_game,
)
from tests.test_core_dice import FixedDice

BOOK = "pretre_jean/forteresse_alamuth"

SINGLE_FIGHT = {
    "fight_type": "single",
    "enemies": [{"name": "collecteur", "force": 6, "vie": 10}],
    "outcome": {"on_victory": "621", "on_defeat": "death", "on_flee": None},
}
MELEE_FIGHT = {
    "fight_type": "simultaneous",
    "enemies": [{"name": "lepreux", "force": 6, "vie": 6, "count": 2}],
    "outcome": {"on_victory": "10", "on_defeat": "death", "on_flee": None},
}


class TestStartGame:
    """Rolling up a hero and opening a game for him."""

    def test_the_hero_is_rolled_and_stored(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(3, 4, 5, 2, 1, 1, 1, 1))

        assert game["force"] == 13
        assert game["vie_max"] == 25
        assert game["vie_actuelle"] == 25
        assert game["force_dice"] == [3, 4]
        assert game["vie_dice"] == [5, 2]

    def test_the_mode_is_the_book_by_default(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert game["mode"] == "normal"
        assert game["force_throw"] == "2D6"
        assert game["force_base"] == 6

    def test_an_easy_hero_is_rolled_the_easy_way(self, fake_db):
        game = start_game(BOOK, mode=EASY, rng=FixedDice(3, 4, 2, 3, 1, 1, 1, 1, 1))

        assert game["mode"] == "easy"
        assert game["force"] == 12 + 7
        assert game["force_dice"] == [3, 4]
        assert game["vie_dice"] == [2, 3, 1]
        assert game["vie_max"] == 26 + 6

    def test_the_easy_sheet_says_which_dice_made_it(self, fake_db):
        game = start_game(BOOK, mode=EASY, rng=random.Random(1))

        assert game["mode_label"] == "Facile"
        assert game["force_throw"] == "2D4"
        assert game["vie_throw"] == "3D4"
        assert (game["force_base"], game["vie_base"]) == (12, 26)

    def test_the_mode_survives_a_reload(self, fake_db):
        created = start_game(BOOK, mode=EASY, rng=random.Random(1))

        assert find_game(created["id"])["mode"] == "easy"
        assert find_game(created["id"]) == created

    def test_an_easy_hero_is_stronger_than_the_book_allows(self, fake_db):
        """Its floor is above what the book's worst roll can give."""
        easy = [
            start_game(BOOK, mode=EASY, rng=random.Random(s))["force"]
            for s in range(40)
        ]
        normal = [
            start_game(BOOK, mode=NORMAL, rng=random.Random(s))["force"]
            for s in range(40)
        ]

        assert min(easy) > min(normal)

    def test_a_new_game_is_not_in_a_fight(self, fake_db):
        assert start_game(BOOK, rng=random.Random(1))["combat"] is None

    def test_the_adjustment_is_derived_from_the_force(self, fake_db):
        assert start_game(BOOK, rng=FixedDice(6, 6, 1, 1, 1, 1, 1, 1))["damage_adjustment"] == 2

    def test_each_game_gets_its_own_id(self, fake_db):
        first = start_game(BOOK, rng=random.Random(1))
        second = start_game(BOOK, rng=random.Random(1))

        assert first["id"] != second["id"]

    def test_it_is_written_to_the_database(self, fake_db):
        start_game(BOOK, rng=random.Random(1))

        assert len(fake_db["games"].docs) == 1


class TestFindGame:
    """Loading one back."""

    def test_a_game_is_loaded_as_it_was_written(self, fake_db):
        created = start_game(BOOK, rng=FixedDice(3, 4, 5, 2, 1, 1, 1, 1))

        assert find_game(created["id"]) == created

    def test_reopening_never_re_rolls_the_hero(self, fake_db):
        """The dice are thrown once, at creation, and never again."""
        created = start_game(BOOK, rng=random.Random(1))

        for _ in range(5):
            assert find_game(created["id"])["force"] == created["force"]
            assert find_game(created["id"])["vie_max"] == created["vie_max"]

    def test_an_unknown_id_finds_nothing(self, fake_db):
        assert find_game("nope") is None

    def test_a_blank_id_finds_nothing(self, fake_db):
        assert find_game("") is None
        assert find_game(None) is None


class TestBeginCombat:
    """Putting the hero into the fight a page holds."""

    def test_the_adversaries_are_fielded_at_full_vie(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        combat = begin_combat(game["id"], "22", SINGLE_FIGHT)["combat"]

        assert combat["page"] == "22"
        assert combat["status"] == ONGOING
        assert combat["enemies"] == [
            {
                "name": "collecteur",
                "force": 6,
                "vie_max": 10,
                "vie_actuelle": 10,
                "damage_adjustment": 0,
            }
        ]

    def test_the_outcome_branches_are_kept(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        combat = begin_combat(game["id"], "22", SINGLE_FIGHT)["combat"]

        assert combat["on_victory"] == "621"
        assert combat["on_defeat"] == "death"
        assert combat["on_flee"] is None

    def test_a_repeated_adversary_becomes_that_many_bodies(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        combat = begin_combat(game["id"], "74", MELEE_FIGHT)["combat"]

        assert [enemy["name"] for enemy in combat["enemies"]] == [
            "lepreux 1",
            "lepreux 2",
        ]
        assert all(enemy["vie_actuelle"] == 6 for enemy in combat["enemies"])

    def test_a_lone_adversary_is_not_numbered(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        combat = begin_combat(game["id"], "22", SINGLE_FIGHT)["combat"]

        assert combat["enemies"][0]["name"] == "collecteur"

    def test_a_damage_adjustment_is_carried_over(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        fight = {
            "fight_type": "single",
            "enemies": [
                {"name": "djinn", "force": 16, "vie": 20, "damage_adjustment": 1}
            ],
            "outcome": {},
        }

        combat = begin_combat(game["id"], "133", fight)["combat"]

        assert combat["enemies"][0]["damage_adjustment"] == 1

    def test_a_page_with_special_rules_is_flagged(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert begin_combat(game["id"], "74", MELEE_FIGHT)["combat"][
            "has_special_rules"
        ]

    def test_an_ordinary_page_is_not(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert not begin_combat(game["id"], "22", SINGLE_FIGHT)["combat"][
            "has_special_rules"
        ]

    def test_re_entering_the_page_does_not_restart_the_fight(self, fake_db):
        """A reload must not hand back full-health adversaries."""
        game = start_game(BOOK, rng=random.Random(1))
        begin_combat(game["id"], "22", SINGLE_FIGHT)
        play_assault(game["id"], random.Random(2))
        wounded = find_game(game["id"])["combat"]["enemies"][0]["vie_actuelle"]

        again = begin_combat(game["id"], "22", SINGLE_FIGHT)["combat"]

        assert again["enemies"][0]["vie_actuelle"] == wounded
        assert len(again["assaults"]) == 1

    def test_another_page_starts_a_fresh_fight(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        begin_combat(game["id"], "22", SINGLE_FIGHT)
        play_assault(game["id"], random.Random(2))

        combat = begin_combat(game["id"], "74", MELEE_FIGHT)["combat"]

        assert combat["page"] == "74"
        assert combat["assaults"] == []

    def test_a_fight_with_no_adversary_arms_nothing(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        empty = {"fight_type": "single", "enemies": [], "outcome": {}}

        assert begin_combat(game["id"], "18", empty) is None

    def test_a_malformed_adversary_is_dropped(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        fight = {"fight_type": "single", "enemies": ["orc"], "outcome": {}}

        assert begin_combat(game["id"], "18", fight) is None

    def test_an_unknown_game_arms_nothing(self, fake_db):
        assert begin_combat("nope", "22", SINGLE_FIGHT) is None


class TestPlayAssault:
    """Advancing an open fight, one assault at a time."""

    def test_the_assault_is_logged_with_its_dice(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(4, 4, 4, 4, 1, 1, 1, 1))  # Force 14, Vie 26
        begin_combat(game["id"], "22", SINGLE_FIGHT)

        combat = play_assault(game["id"], FixedDice(3, 3, 1, 2))["combat"]

        assault = combat["assaults"][0]
        assert assault["number"] == 1
        assert assault["hero_dice"] == [3, 3]
        assert assault["hero_attack_force"] == 20
        assert assault["exchanges"][0]["enemy_dice"] == [1, 2]
        assert assault["exchanges"][0]["enemy_force"] == 6
        assert assault["exchanges"][0]["enemy_attack_force"] == 9
        assert assault["exchanges"][0]["damage"] == 11

    def test_the_wounds_are_persisted(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(4, 4, 4, 4, 1, 1, 1, 1))
        begin_combat(game["id"], "22", SINGLE_FIGHT)
        play_assault(game["id"], FixedDice(1, 1, 6, 6))  # hero loses

        reloaded = find_game(game["id"])

        assert reloaded["vie_actuelle"] < reloaded["vie_max"]

    def test_assaults_are_numbered_in_order(self, fake_db):
        # Vie enough that three assaults cannot decide it either way.
        tough = {
            "fight_type": "single",
            "enemies": [{"name": "thalos", "force": 18, "vie": 400}],
            "outcome": {"on_victory": "149", "on_defeat": "death"},
        }
        game = start_game(BOOK, rng=FixedDice(6, 5, 6, 6, 1, 1, 1, 1))  # Force 17, Vie 30
        begin_combat(game["id"], "503", tough)
        for _ in range(3):
            play_assault(game["id"], FixedDice(4, 4, 3, 3))

        combat = find_game(game["id"])["combat"]

        assert [a["number"] for a in combat["assaults"]] == [1, 2, 3]
        assert combat["status"] == ONGOING

    def test_a_won_fight_stops_accepting_assaults(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(6, 6, 6, 6, 1, 1, 1, 1))  # Force 18, Vie 30
        begin_combat(game["id"], "22", SINGLE_FIGHT)
        play_assault(game["id"], FixedDice(6, 6, 1, 2))  # double 6 kills

        assert find_game(game["id"])["combat"]["status"] == VICTORY
        assert play_assault(game["id"], random.Random(1)) is None

    def test_a_lost_fight_is_a_defeat(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(1, 1, 1, 1, 1, 1, 1, 1))  # Force 8, Vie 20
        begin_combat(game["id"], "22", SINGLE_FIGHT)

        combat = play_assault(game["id"], FixedDice(3, 4, 1, 1))["combat"]

        assert combat["status"] == DEFEAT
        assert find_game(game["id"])["vie_actuelle"] == 0

    def test_a_game_not_in_a_fight_has_nothing_to_play(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert play_assault(game["id"], random.Random(1)) is None

    def test_an_unknown_game_has_nothing_to_play(self, fake_db):
        assert play_assault("nope", random.Random(1)) is None


class TestEndCombat:
    """Leaving a fight behind."""

    def test_the_fight_is_cleared(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        begin_combat(game["id"], "22", SINGLE_FIGHT)

        assert end_combat(game["id"])["combat"] is None
        assert find_game(game["id"])["combat"] is None

    def test_the_wounds_it_cost_are_kept(self, fake_db):
        """Leaving the fight does not heal the hero."""
        game = start_game(BOOK, rng=FixedDice(1, 1, 4, 4, 1, 1, 1, 1))
        begin_combat(game["id"], "22", SINGLE_FIGHT)
        play_assault(game["id"], FixedDice(1, 1, 6, 5))
        wounded = find_game(game["id"])["vie_actuelle"]

        assert end_combat(game["id"])["vie_actuelle"] == wounded

    def test_an_unknown_game_clears_nothing(self, fake_db):
        assert end_combat("nope") is None


class TestGameRow:
    """Corners the browser flow never reaches on its own."""

    def test_a_blank_id_loads_no_row(self, fake_db):
        """The three writers share the same guard as find_game()."""
        assert end_combat("") is None
        assert play_assault("", random.Random(1)) is None

    def test_a_game_prints_its_hero(self, fake_db):
        from haute_tension.core.models.game import Game

        game = Game.from_dict(
            {"id": "x", "book": BOOK, "mode": "easy", "force": 14, "vie_max": 27, "vie_actuelle": 20}
        )

        assert str(game) == f"{BOOK} (easy) — Force 14, Vie 20/27"


GOLD_FIGHT_PAGE = "39"
COSTLY_CHOICE = {
    "goto": "40",
    "gains": [{"element": "meal", "label_fr": "repas", "amount": 1}],
    "losses": [{"element": "gold coin", "label_fr": "pièce d'or", "amount": 5}],
}
CONDITIONAL_CHOICE = {
    "goto": "41",
    "gains": [
        {
            "element": "strength point",
            "label_fr": "point de Force",
            "amount": 1,
            "condition": "pendant tout le temps où vous les porterez",
        }
    ],
    "losses": [],
}


class TestStartingHoldings:
    """What the hero sets out with."""

    def test_he_carries_a_sword_a_bag_and_four_rations(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert {item["element"]: item["count"] for item in game["bag"]} == {
            "sword": 1,
            "bag": 1,
            "ration": 4,
        }

    def test_his_purse_is_two_throws_of_two_dice(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(1, 1, 1, 1, 3, 4, 5, 2))

        assert game["gold_dice"] == [3, 4, 5, 2]
        assert game["gold"] == 14

    def test_the_purse_never_leaves_four_to_twenty_four(self, fake_db):
        purses = [
            start_game(BOOK, rng=random.Random(s))["gold"] for s in range(60)
        ]

        assert min(purses) >= 4
        assert max(purses) <= 24

    def test_he_owes_nothing_and_waits_on_nothing(self, fake_db):
        assert start_game(BOOK, rng=random.Random(1))["pending"] == []


class TestFollowingAChoice:
    """What a choice costs and gives."""

    def test_an_unconditional_loss_is_paid(self, fake_db):
        game = start_game(BOOK, rng=FixedDice(1, 1, 1, 1, 6, 6, 6, 6))
        assert game["gold"] == 24

        after = follow_choice(game["id"], GOLD_FIGHT_PAGE, COSTLY_CHOICE)

        assert after["gold"] == 19

    def test_an_unconditional_gain_is_taken(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        after = follow_choice(game["id"], GOLD_FIGHT_PAGE, COSTLY_CHOICE)

        assert {item["element"] for item in after["bag"]} >= {"meal"}

    def test_it_is_written_to_the_database(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        follow_choice(game["id"], GOLD_FIGHT_PAGE, COSTLY_CHOICE)

        assert find_game(game["id"])["gold"] == game["gold"] - 5

    def test_a_conditional_change_waits_for_the_reader(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        after = follow_choice(game["id"], "11", CONDITIONAL_CHOICE)

        assert after["force"] == game["force"]
        assert len(after["pending"]) == 1
        assert after["pending"][0]["element"] == "strength point"
        assert after["pending"][0]["page"] == "11"
        assert "porterez" in after["pending"][0]["condition"]

    def test_a_choice_carrying_nothing_changes_nothing(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        after = follow_choice(game["id"], "1", {"goto": "2"})

        assert after["gold"] == game["gold"]
        assert after["bag"] == game["bag"]
        assert after["pending"] == []

    def test_following_again_replaces_what_was_waiting(self, fake_db):
        """A reader who walked past one condition is not asked about it forever."""
        game = start_game(BOOK, rng=random.Random(1))
        follow_choice(game["id"], "11", CONDITIONAL_CHOICE)

        after = follow_choice(game["id"], "1", {"goto": "2"})

        assert after["pending"] == []

    def test_an_unknown_game_follows_nothing(self, fake_db):
        assert follow_choice("nope", "1", COSTLY_CHOICE) is None


class TestRulingOnAWaitingChange:
    """The reader's word on a condition only he can judge."""

    def test_applying_it_moves_the_hero(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        follow_choice(game["id"], "11", CONDITIONAL_CHOICE)

        after = apply_pending(game["id"], 0)

        assert after["force"] == game["force"] + 1
        assert after["pending"] == []

    def test_dismissing_it_leaves_the_hero_alone(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        follow_choice(game["id"], "11", CONDITIONAL_CHOICE)

        after = dismiss_pending(game["id"], 0)

        assert after["force"] == game["force"]
        assert after["pending"] == []

    def test_dismissing_everything_clears_the_lot(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))
        follow_choice(
            game["id"],
            "11",
            {
                "goto": "12",
                "gains": CONDITIONAL_CHOICE["gains"] * 2,
                "losses": [],
            },
        )

        assert len(find_game(game["id"])["pending"]) == 2
        assert dismiss_pending(game["id"])["pending"] == []

    def test_an_index_that_names_nothing_does_nothing(self, fake_db):
        game = start_game(BOOK, rng=random.Random(1))

        assert apply_pending(game["id"], 0) is None
        assert dismiss_pending(game["id"], 3) is None

    def test_an_unknown_game_rules_on_nothing(self, fake_db):
        assert apply_pending("nope", 0) is None
        assert dismiss_pending("nope") is None
