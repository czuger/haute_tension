"""Two six-sided dice, and what the rules read off them.

The bottom of the game rules: everything else in `core` that needs chance goes
through `roll_2d6`, so a test seeds one `random.Random` and the whole run becomes
deterministic.

A roll is kept as its two faces rather than as a total, because the rules read
both: `8` says nothing, but `4 + 4` and `6 + 2` are the same total and only one
of them can be a double.
"""

import random
from typing import NamedTuple

DICE_PER_ROLL = 2
DIE_FACES = 6

_DEFAULT_RNG = random.Random()


class Roll(NamedTuple):
    """One throw of two dice, kept face by face."""

    dice: tuple[int, ...]

    @property
    def total(self) -> int:
        """The sum of the faces."""
        return sum(self.dice)

    @property
    def is_double_six(self) -> bool:
        """Whether every face came up six — the hero's divine judgement."""
        return all(die == DIE_FACES for die in self.dice)

    @property
    def is_double_one(self) -> bool:
        """Whether every face came up one — the adversary's."""
        return all(die == 1 for die in self.dice)

    def __str__(self) -> str:
        return " + ".join(str(die) for die in self.dice) + f" = {self.total}"


def roll_2d6(rng: random.Random | None = None) -> Roll:
    """Throw two six-sided dice.

    Args:
        rng: Source of chance. Seed one to make a run reproducible; the default
            is a module-level generator.

    Returns:
        The roll, face by face.
    """
    generator = rng or _DEFAULT_RNG
    return Roll(
        tuple(generator.randint(1, DIE_FACES) for _ in range(DICE_PER_ROLL))
    )
