"""The shape of a play-through, as the routes and templates read it.

The dict side of `core.models.game`: `core.db` converts on the way out, so
nothing above it holds a document. Same role the `StoryPage` TypedDicts play for
the book.
"""

from typing import TypedDict

# What the session cookie carries, and all it carries: the id of the
# play-through. Read by the routes that drive a game and by the request trace,
# which is why it sits here rather than in either.
SESSION_KEY = "game_id"


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


class Item(TypedDict):
    """One thing in the hero's bag, and how many of it he has."""

    element: str
    label: str
    count: int


class Pending(TypedDict):
    """A gain or loss waiting on the reader's word."""

    index: int
    element: str
    label: str
    amount: int | None
    condition: str | None
    note: str | None
    sign: int
    page: str | None
    described: str


class GameDict(TypedDict):
    """A hero, rolled once, and the fight he is in the middle of."""

    id: int
    book: str
    mode: str
    mode_label: str
    force_throw: str
    vie_throw: str
    force_base: int
    vie_base: int
    force: int
    vie_max: int
    vie_actuelle: int
    force_dice: list[int]
    vie_dice: list[int]
    damage_adjustment: int
    gold: int
    gold_dice: list[int]
    # Named `bag` and not `items`: a template reading `game.items` would find
    # `dict.items` — the method — before the key, and fail well away from here.
    bag: list[Item]
    pending: list[Pending]
    combat: Combat | None
    is_dead: bool
    died_on_page: str | None
    died_of: str | None


class FallenHero(TypedDict):
    """One hero who did not come back, as the memorial lists him."""

    id: int
    mode_label: str
    force: int
    vie_max: int
    gold: int
    bag: list[Item]
    died_on_page: str | None
    died_of: str | None
    died_at: str | None
