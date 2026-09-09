"""Tests for rolling up Prêtre Jean, by the book and the easy way."""

import random

import pytest

from haute_tension.core.character import (
    DEFAULT_MODE,
    EASY,
    MODES,
    NORMAL,
    Throw,
    damage_adjustment,
    generate_character,
    named_mode,
)
from tests.test_core_dice import FixedDice


class TestThrow:
    """A characteristic: a base, and the dice added to it."""

    def test_it_writes_its_dice_the_way_a_reader_does(self):
        assert Throw(6, 2, 6).notation == "2D6"
        assert Throw(26, 3, 4).notation == "3D4"

    def test_its_floor_is_every_die_at_one(self):
        assert Throw(6, 2, 6).lowest == 8
        assert Throw(26, 3, 4).lowest == 29

    def test_its_ceiling_is_every_die_at_its_top_face(self):
        assert Throw(6, 2, 6).highest == 18
        assert Throw(26, 3, 4).highest == 38

    def test_rolling_adds_the_base_to_the_dice(self):
        total, roll = Throw(12, 2, 4).roll(FixedDice(3, 4))

        assert roll.dice == (3, 4)
        assert total == 19

    def test_it_throws_as_many_dice_as_it_says(self):
        _, roll = Throw(26, 3, 4).roll(random.Random(1))

        assert len(roll.dice) == 3


class TestTheModes:
    """The two ways of rolling a hero up."""

    def test_the_book_is_the_default(self):
        assert DEFAULT_MODE is NORMAL

    def test_the_normal_mode_is_the_rules_as_quoted(self):
        assert (NORMAL.force.base, NORMAL.force.notation) == (6, "2D6")
        assert (NORMAL.vie.base, NORMAL.vie.notation) == (18, "2D6")

    def test_the_easy_mode_is_the_house_rule(self):
        assert (EASY.force.base, EASY.force.notation) == (12, "2D4")
        assert (EASY.vie.base, EASY.vie.notation) == (26, "3D4")

    def test_both_are_offered(self):
        assert set(MODES) == {"normal", "easy"}

    def test_the_easy_mode_cannot_roll_a_weakling(self):
        """Its floor is above the book's average, which is the whole point."""
        assert EASY.force.lowest > (NORMAL.force.lowest + NORMAL.force.highest) / 2
        assert EASY.vie.lowest > (NORMAL.vie.lowest + NORMAL.vie.highest) / 2


class TestNamedMode:
    """Reading a mode off a form."""

    @pytest.mark.parametrize("name", ["normal", "easy"])
    def test_a_known_name_is_found(self, name):
        assert named_mode(name).name == name

    def test_case_and_padding_are_ignored(self):
        assert named_mode("  EASY ") is EASY

    def test_an_unknown_name_rolls_by_the_book(self):
        """A posted value is whatever was posted; it must not refuse to roll."""
        assert named_mode("impossible") is NORMAL

    def test_nothing_at_all_rolls_by_the_book(self):
        assert named_mode(None) is NORMAL
        assert named_mode("") is NORMAL


class TestGeneration:
    """The throws each mode asks for."""

    def test_the_book_hero_is_six_plus_two_dice(self):
        character = generate_character(NORMAL, FixedDice(3, 4, 5, 2))

        assert character.force_roll.dice == (3, 4)
        assert character.force == 13
        assert character.vie_max == 25

    def test_the_easy_hero_is_twelve_plus_two_dice(self):
        character = generate_character(EASY, FixedDice(3, 4, 2, 3, 1))

        assert character.force_roll.dice == (3, 4)
        assert character.force == 19

    def test_the_easy_hero_rolls_three_dice_for_vie(self):
        character = generate_character(EASY, FixedDice(1, 1, 2, 3, 4))

        assert character.vie_roll.dice == (2, 3, 4)
        assert character.vie_max == 35

    @pytest.mark.parametrize("mode", [NORMAL, EASY])
    def test_a_mode_never_leaves_its_range(self, mode):
        heroes = [generate_character(mode, random.Random(s)) for s in range(400)]

        assert min(h.force for h in heroes) >= mode.force.lowest
        assert max(h.force for h in heroes) <= mode.force.highest
        assert min(h.vie_max for h in heroes) >= mode.vie.lowest
        assert max(h.vie_max for h in heroes) <= mode.vie.highest

    @pytest.mark.parametrize("mode", [NORMAL, EASY])
    def test_the_whole_range_is_reachable(self, mode):
        heroes = [generate_character(mode, random.Random(s)) for s in range(600)]

        assert min(h.force for h in heroes) == mode.force.lowest
        assert max(h.force for h in heroes) == mode.force.highest

    @pytest.mark.parametrize("mode", [NORMAL, EASY])
    def test_he_starts_at_full_vie(self, mode):
        character = generate_character(mode, random.Random(1))

        assert character.vie_actuelle == character.vie_max

    @pytest.mark.parametrize("mode", [NORMAL, EASY])
    def test_he_remembers_how_he_was_rolled(self, mode):
        assert generate_character(mode, random.Random(1)).mode is mode

    def test_the_book_is_used_when_no_mode_is_named(self):
        assert generate_character(rng=random.Random(1)).mode is NORMAL

    def test_force_and_vie_use_different_throws(self):
        character = generate_character(NORMAL, FixedDice(1, 1, 6, 6))

        assert character.force == 8
        assert character.vie_max == 30

    @pytest.mark.parametrize("mode", [NORMAL, EASY])
    def test_a_seed_makes_the_hero_reproducible(self, mode):
        assert generate_character(mode, random.Random(9)) == generate_character(
            mode, random.Random(9)
        )


class TestDamageAdjustment:
    """What a high Force adds to the damage its blows do."""

    def test_an_ordinary_force_earns_nothing(self):
        assert damage_adjustment(10) == 0
        assert damage_adjustment(16) == 0

    def test_seventeen_adds_one(self):
        assert damage_adjustment(17) == 1

    def test_eighteen_adds_two(self):
        assert damage_adjustment(18) == 2

    @pytest.mark.parametrize("force", [19, 20])
    def test_beyond_the_table_keeps_the_top_step(self, force):
        """The easy mode reaches 20; rolling higher must not be worse than 18."""
        assert damage_adjustment(force) == damage_adjustment(18) == 2

    def test_the_hero_reads_it_off_his_force(self):
        assert generate_character(NORMAL, FixedDice(6, 6, 1, 1)).damage_adjustment == 2
        assert generate_character(NORMAL, FixedDice(3, 4, 1, 1)).damage_adjustment == 0
