"""The hero: how Prêtre Jean is rolled up, and what the sheet shows.

The rules of the series, quoted from `regles-du-jeu-spj1`:

- Vie — "lancez deux dés. Ajoutez 18 au chiffre que vous avez obtenu."
- Force — "lancez deux dés. Ajoutez 6 au chiffre que vous aurez obtenu."

So Force falls in 8–18 and Vie in 20–30, and both are rolled **once**, when the
game is created. Nothing here re-rolls: `core.db.start_game()` writes the result
and every later read loads it back.

Vie is kept twice over. `vie_max` is what was rolled and never moves again;
`vie_actuelle` is what is left, and is the only thing combat writes.
"""

import random
from typing import NamedTuple

from haute_tension.core.dice import Roll, roll_2d6

FORCE_BASE = 6
VIE_BASE = 18

# "Si, en calculant ce total de Force, vous avez obtenu 17, votre total bénéficie
# d'un Ajustement-Force": a high Force adds to the damage its blows do, the same
# way an adversary's own "AJUSTEMENT DOMMAGES" does. Only the top of the range
# earns one, which is why most heroes have none.
FORCE_DAMAGE_ADJUSTMENTS = {17: 1, 18: 2}


class Character(NamedTuple):
    """A freshly rolled hero, and the dice that made him."""

    force: int
    vie_max: int
    vie_actuelle: int
    force_roll: Roll
    vie_roll: Roll

    @property
    def damage_adjustment(self) -> int:
        """How much this hero's Force adds to the damage he deals."""
        return FORCE_DAMAGE_ADJUSTMENTS.get(self.force, 0)


def generate_force(rng: random.Random | None = None) -> tuple[int, Roll]:
    """Roll the hero's Force.

    Args:
        rng: Source of chance.

    Returns:
        The Force total and the roll that produced it.
    """
    roll = roll_2d6(rng)
    return FORCE_BASE + roll.total, roll


def generate_vie(rng: random.Random | None = None) -> tuple[int, Roll]:
    """Roll the hero's Vie.

    Args:
        rng: Source of chance.

    Returns:
        The Vie total and the roll that produced it.
    """
    roll = roll_2d6(rng)
    return VIE_BASE + roll.total, roll


def generate_character(rng: random.Random | None = None) -> Character:
    """Roll up Prêtre Jean, once, at the start of a game.

    Args:
        rng: Source of chance.

    Returns:
        The hero, with full Vie and the two rolls that made him.
    """
    force, force_roll = generate_force(rng)
    vie, vie_roll = generate_vie(rng)
    return Character(
        force=force,
        vie_max=vie,
        vie_actuelle=vie,
        force_roll=force_roll,
        vie_roll=vie_roll,
    )
