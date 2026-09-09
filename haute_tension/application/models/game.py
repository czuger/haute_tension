"""The shape of a play-through, as the routes and templates read it.

The dict side of `core.models.game`: `core.db` converts on the way out, so
nothing above it holds a document. Same role the `StoryPage` TypedDicts play for
the book.
"""

from typing import TypedDict


class Exchange(TypedDict):
    """What happened between the hero and one adversary in one assault."""

    enemy_name: str
    enemy_force: int
    enemy_dice: list[int]
    enemy_attack_force: int
    # "hero", "enemy", or None when the two Forces d'Attaque were equal.
    winner: str | None
    damage: int
    divine_judgement: bool


class Assault(TypedDict):
    """One assault: the hero's single roll, and every exchange it settled."""

    number: int
    hero_dice: list[int]
    hero_attack_force: int
    exchanges: list[Exchange]


class Enemy(TypedDict):
    """One adversary of the current fight, as it stands."""

    name: str
    force: int
    vie_max: int
    vie_actuelle: int
    damage_adjustment: int


class Combat(TypedDict):
    """The fight the hero is in, and everything it has cost so far."""

    page: str
    fight_type: str
    status: str
    on_victory: str | None
    on_defeat: str | None
    on_flee: str | None
    has_special_rules: bool
    enemies: list[Enemy]
    assaults: list[Assault]


class GameDict(TypedDict):
    """A hero, rolled once, and the fight he is in the middle of."""

    id: str
    book: str
    force: int
    vie_max: int
    vie_actuelle: int
    force_dice: list[int]
    vie_dice: list[int]
    damage_adjustment: int
    combat: Combat | None
