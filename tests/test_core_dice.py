"""Tests for the two dice everything else reads."""

import random

from haute_tension.core.dice import DIE_FACES, Roll, roll_2d6


class FixedDice:
    """A `random.Random` stand-in handing out faces in a set order."""

    def __init__(self, *faces):
        self._faces = list(faces)

    def randint(self, low, high):
        return self._faces.pop(0)


class TestRoll:
    """What the rules read off a throw."""

    def test_the_total_is_the_sum_of_the_faces(self):
        assert Roll((4, 5)).total == 9

    def test_a_double_six_is_recognised(self):
        assert Roll((6, 6)).is_double_six
        assert not Roll((6, 5)).is_double_six

    def test_a_double_one_is_recognised(self):
        assert Roll((1, 1)).is_double_one
        assert not Roll((1, 2)).is_double_one

    def test_a_double_six_is_not_a_double_one(self):
        assert not Roll((6, 6)).is_double_one

    def test_the_faces_are_kept_apart(self):
        """A total of 8 could be 4+4 or 6+2, and only one of them is a double."""
        assert Roll((4, 4)).total == Roll((6, 2)).total
        assert Roll((4, 4)).dice != Roll((6, 2)).dice

    def test_it_prints_its_working(self):
        assert str(Roll((3, 5))) == "3 + 5 = 8"


class TestRoll2d6:
    """Throwing them."""

    def test_two_dice_are_thrown(self):
        assert len(roll_2d6(random.Random(1)).dice) == 2

    def test_the_faces_come_from_the_generator(self):
        assert roll_2d6(FixedDice(2, 5)).dice == (2, 5)

    def test_a_seed_makes_a_run_reproducible(self):
        assert roll_2d6(random.Random(4)).dice == roll_2d6(random.Random(4)).dice

    def test_every_face_stays_on_the_die(self):
        faces = {
            die
            for seed in range(200)
            for die in roll_2d6(random.Random(seed)).dice
        }

        assert faces == set(range(1, DIE_FACES + 1))

    def test_it_falls_back_to_its_own_generator(self):
        assert 2 <= roll_2d6().total <= 12
