"""Tests for rolling up Prêtre Jean."""

import random

from haute_tension.core.character import (
    FORCE_BASE,
    VIE_BASE,
    generate_character,
    generate_force,
    generate_vie,
)
from tests.test_core_dice import FixedDice


class TestGeneration:
    """The two throws the rules ask for."""

    def test_force_is_six_plus_two_dice(self):
        force, roll = generate_force(FixedDice(3, 4))

        assert roll.dice == (3, 4)
        assert force == FORCE_BASE + 7 == 13

    def test_vie_is_eighteen_plus_two_dice(self):
        vie, roll = generate_vie(FixedDice(5, 2))

        assert roll.dice == (5, 2)
        assert vie == VIE_BASE + 7 == 25

    def test_force_never_leaves_its_range(self):
        forces = [generate_force(random.Random(seed))[0] for seed in range(300)]

        assert min(forces) >= FORCE_BASE + 2
        assert max(forces) <= FORCE_BASE + 12

    def test_vie_never_leaves_its_range(self):
        vies = [generate_vie(random.Random(seed))[0] for seed in range(300)]

        assert min(vies) >= VIE_BASE + 2
        assert max(vies) <= VIE_BASE + 12


class TestCharacter:
    """The hero the two throws make."""

    def test_he_starts_at_full_vie(self):
        character = generate_character(random.Random(1))

        assert character.vie_actuelle == character.vie_max

    def test_both_rolls_are_kept_for_the_sheet(self):
        character = generate_character(FixedDice(3, 4, 5, 2))

        assert character.force_roll.dice == (3, 4)
        assert character.vie_roll.dice == (5, 2)

    def test_force_and_vie_use_different_throws(self):
        character = generate_character(FixedDice(1, 1, 6, 6))

        assert character.force == FORCE_BASE + 2
        assert character.vie_max == VIE_BASE + 12

    def test_an_ordinary_force_earns_no_adjustment(self):
        assert generate_character(FixedDice(3, 4, 1, 1)).damage_adjustment == 0

    def test_a_force_of_seventeen_adds_one(self):
        character = generate_character(FixedDice(5, 6, 1, 1))

        assert character.force == 17
        assert character.damage_adjustment == 1

    def test_a_force_of_eighteen_adds_two(self):
        character = generate_character(FixedDice(6, 6, 1, 1))

        assert character.force == 18
        assert character.damage_adjustment == 2

    def test_a_seed_makes_the_hero_reproducible(self):
        assert generate_character(random.Random(9)) == generate_character(
            random.Random(9)
        )
