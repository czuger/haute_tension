"""Dice, and what the rules read off them.

The bottom of the game rules: everything else in `core` that needs chance goes
through here, so a test seeds one `random.Random` and the whole run becomes
deterministic.

A roll is kept face by face rather than as a total, because the rules read both:
`8` says nothing, but `4 + 4` and `6 + 2` are the same total and only one of them
can be a double.

Combat is always two six-sided dice (`roll_2d6`), which is the throw the rules
describe. `roll_dice` is the general one, and character creation is what needs it:
the easy mode rolls four-sided dice, and three of them for Vie.
"""

import random
from typing import NamedTuple

DICE_PER_ROLL = 2
DIE_FACES = 6

_DEFAULT_RNG = random.Random()


class Roll(NamedTuple):
    """One throw, kept face by face."""

    dice: tuple[int, ...]

    @property
    def total(self) -> int:
        """The sum of the faces."""
        return sum(self.dice)

    @property
    def is_double_six(self) -> bool:
        """Whether every face came up six — the hero's divine judgement.

        Read on combat throws only, which are always two six-sided dice. It is
        not asked of a creation throw, whose dice may have four faces and could
        never answer yes.
        """
        return all(die == DIE_FACES for die in self.dice)

    @property
    def is_double_one(self) -> bool:
        """Whether every face came up one — the adversary's divine judgement.

        Read on combat throws only. Three four-sided dice can all come up one,
        and that throw rolls up a hero rather than settling an assault.
        """
        return all(die == 1 for die in self.dice)

    def __str__(self) -> str:
        return " + ".join(str(die) for die in self.dice) + f" = {self.total}"


def roll_dice(
    count: int, faces: int, rng: random.Random | None = None
) -> Roll:
    """Throw `count` dice of `faces` faces each.

    Args:
        count: How many dice.
        faces: How many faces each of them has.
        rng: Source of chance. Seed one to make a run reproducible; the default
            is a module-level generator.

    Returns:
        The roll, face by face.
    """
    generator = rng or _DEFAULT_RNG
    return Roll(tuple(generator.randint(1, faces) for _ in range(count)))


def roll_2d6(rng: random.Random | None = None) -> Roll:
    """Throw two six-sided dice — the throw every rule of combat is written on.

    Args:
        rng: Source of chance.

    Returns:
        The roll, face by face.
    """
    return roll_dice(DICE_PER_ROLL, DIE_FACES, rng)
