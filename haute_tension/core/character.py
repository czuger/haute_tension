"""The hero: how Prêtre Jean is rolled up, and what the sheet shows.

The rules of the series, quoted from `regles-du-jeu-spj1`:

- Vie — "lancez deux dés. Ajoutez 18 au chiffre que vous avez obtenu."
- Force — "lancez deux dés. Ajoutez 6 au chiffre que vous aurez obtenu."

So a hero rolled by the book has a Force of 8–18 and a Vie of 20–30. That is the
`NORMAL` mode below.

`EASY` is a house rule and not the book's: Force 12 + 2D4 and Vie 26 + 3D4, which
is 14–20 and 29–38. It cannot roll a weakling — the floor of each is above the
book's average — and it is there for reading the story rather than surviving it.
The mode is chosen once, when the game is created, and never changes.

Both are rolled **once**, when the game is created. Nothing here re-rolls:
`core.db.start_game()` writes the result and every later read loads it back.

Vie is kept twice over. `vie_max` is what was rolled and never moves again;
`vie_actuelle` is what is left, and is the only thing combat writes.
"""

import random
from typing import NamedTuple

from haute_tension.core.dice import Roll, roll_dice


class Throw(NamedTuple):
    """A characteristic: a fixed base, and the dice added to it."""

    base: int
    count: int
    faces: int

    @property
    def notation(self) -> str:
        """The dice as a reader writes them, `2D6` or `3D4`."""
        return f"{self.count}D{self.faces}"

    @property
    def lowest(self) -> int:
        """The smallest total this throw can give."""
        return self.base + self.count

    @property
    def highest(self) -> int:
        """The largest total this throw can give."""
        return self.base + self.count * self.faces

    def roll(self, rng: random.Random | None = None) -> tuple[int, Roll]:
        """Throw the dice and add the base.

        Args:
            rng: Source of chance.

        Returns:
            The total and the roll that produced it.
        """
        thrown = roll_dice(self.count, self.faces, rng)
        return self.base + thrown.total, thrown


class Mode(NamedTuple):
    """One way of rolling a hero up."""

    name: str
    label: str
    description: str
    force: Throw
    vie: Throw


NORMAL = Mode(
    name="normal",
    label="Normale",
    description="Les règles du livre.",
    force=Throw(base=6, count=2, faces=6),
    vie=Throw(base=18, count=2, faces=6),
)
EASY = Mode(
    name="easy",
    label="Facile",
    description="Un héros plus robuste, pour lire l'histoire sans y laisser la peau.",
    force=Throw(base=12, count=2, faces=4),
    vie=Throw(base=26, count=3, faces=4),
)

MODES = {mode.name: mode for mode in (NORMAL, EASY)}
DEFAULT_MODE = NORMAL

# "Si, en calculant ce total de Force, vous avez obtenu 17, votre total bénéficie
# d'un Ajustement-Force": a high Force adds to the damage its blows do, the same
# way an adversary's own "AJUSTEMENT DOMMAGES" does.
#
# The book's table stops at 18 because its own Force does. The easy mode reaches
# 20, so the top step is read as "18 or more" rather than "18 exactly" — a hero
# who rolled higher than the table foresaw should not be worse off than one who
# rolled 18.
FORCE_DAMAGE_ADJUSTMENTS = {17: 1, 18: 2}
TOP_ADJUSTMENT = max(FORCE_DAMAGE_ADJUSTMENTS.items())


def damage_adjustment(force: int) -> int:
    """How much a Force of this size adds to the damage its blows do.

    Args:
        force: The hero's Force.

    Returns:
        The adjustment, `0` for a Force the table does not reach.
    """
    top_force, top_adjustment = TOP_ADJUSTMENT
    if force >= top_force:
        return top_adjustment
    return FORCE_DAMAGE_ADJUSTMENTS.get(force, 0)


class Character(NamedTuple):
    """A freshly rolled hero, and the dice that made him."""

    mode: Mode
    force: int
    vie_max: int
    vie_actuelle: int
    force_roll: Roll
    vie_roll: Roll

    @property
    def damage_adjustment(self) -> int:
        """How much this hero's Force adds to the damage he deals."""
        return damage_adjustment(self.force)


def named_mode(name: str | None) -> Mode:
    """The mode a name asks for, the normal one for anything else.

    A name comes off a form, so it is whatever was posted: an unknown one rolls a
    hero by the book rather than refusing to roll one at all.

    Args:
        name: The mode's name, or nothing.

    Returns:
        The mode.
    """
    return MODES.get((name or "").strip().lower(), DEFAULT_MODE)


def generate_character(
    mode: Mode = DEFAULT_MODE, rng: random.Random | None = None
) -> Character:
    """Roll up Prêtre Jean, once, at the start of a game.

    Args:
        mode: How to roll him — by the book, or the easy way.
        rng: Source of chance.

    Returns:
        The hero, with full Vie and the two rolls that made him.
    """
    force, force_roll = mode.force.roll(rng)
    vie, vie_roll = mode.vie.roll(rng)
    return Character(
        mode=mode,
        force=force,
        vie_max=vie,
        vie_actuelle=vie,
        force_roll=force_roll,
        vie_roll=vie_roll,
    )
