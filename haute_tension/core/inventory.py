"""What a gain or a loss means, and what the hero carries.

Pure: no database, no Flask. It reads the `gains` and `losses` the import
pipeline attached to every choice and says what they do to a hero.

Three elements are not things in a bag — they are the hero himself:

    life point      -> Vie, which may never rise above the Vie he started with
    strength point  -> Force
    gold coin       -> the purse

Everything else is an item, counted: two vials are one line saying two.

**A change carrying a `condition` is never applied here.** The conditions are
free French — "Si vous avez des provisions dans votre sac", "pendant tout le
temps où vous porterez cette cuirasse" — a prerequisite or a duration, and
neither is anything code can evaluate. They are handed to the reader, who is the
one who knows. `note: "dice"` is the same case by another road: the amount is
rolled, and how many dice the page's text says.

`note: "all"` is the one note that *is* machine-readable: everything of that
element goes, however much there was.

The starting hero, from `regles-du-jeu-spj1`: "vous êtes équipé de votre épée et
d'un sac", "4 rations de provisions", and a purse of two throws of two dice —
"vous voilà donc riche d'un montant allant de 4 à 24 pièces d'or".
"""

import random
from dataclasses import dataclass, replace

from haute_tension.core.dice import Roll, roll_2d6

LIFE = "life point"
STRENGTH = "strength point"
GOLD = "gold coin"

# The three elements that change the hero rather than what he carries.
CHARACTERISTICS = (LIFE, STRENGTH, GOLD)

# The one note the code can act on: take all of it, whatever the amount was.
ALL = "all"
# The note that means the page's text says how many dice to roll for the amount.
DICE = "dice"

GOLD_THROWS = 2

STARTING_ITEMS = (
    ("sword", "épée", 1),
    ("bag", "sac", 1),
    ("ration", "ration", 4),
)


@dataclass(frozen=True)
class Change:
    """One gain or loss, as a choice carries it."""

    element: str
    label: str
    amount: int | None
    condition: str | None = None
    note: str | None = None
    # A gain adds, a loss takes away. The sign the caller read off the key.
    sign: int = 1

    @property
    def is_conditional(self) -> bool:
        """Whether only the reader can say if, or how much, this applies.

        A stated condition, or an amount the page's text asks to be rolled: both
        need someone who has read the page.
        """
        return bool(self.condition) or self.note == DICE

    @property
    def takes_everything(self) -> bool:
        """Whether this empties the element rather than moving it by an amount."""
        return self.note == ALL

    def described(self) -> str:
        """The change as a reader reads it: `-2 points de Vie`, `tout l'or`."""
        if self.takes_everything:
            return f"{self.marker} tout ({self.label})"
        if self.amount is None:
            return f"{self.marker}? {self.label}"
        return f"{self.marker}{self.amount} {pluralised(self.label, self.amount)}"

    @property
    def marker(self) -> str:
        """`+` for a gain, `-` for a loss."""
        return "+" if self.sign > 0 else "-"


@dataclass(frozen=True)
class Holdings:
    """What a hero is and carries, as far as gains and losses are concerned."""

    force: int
    vie_max: int
    vie_actuelle: int
    gold: int
    # element -> (label, count), in the order things were first picked up.
    items: dict[str, tuple[str, int]]

    def with_change(self, change: Change) -> "Holdings":
        """Return these holdings with one change applied.

        Args:
            change: The gain or loss to apply. A conditional one is applied here
                too — deciding whether it *should* be is the caller's job, and
                `core.db` only ever hands over the ones the reader agreed to.

        Returns:
            The holdings afterwards.
        """
        if change.element == LIFE:
            return replace(self, vie_actuelle=self._moved_vie(change))
        if change.element == STRENGTH:
            return replace(self, force=max(0, self.force + self._delta(change)))
        if change.element == GOLD:
            return replace(self, gold=max(0, self.gold + self._delta(change)))
        return replace(self, items=self._moved_items(change))

    def _moved_vie(self, change: Change) -> int:
        """Vie after a change, floored at zero and capped at the starting total.

        "vous devrez toujours garder trace de votre total de Vie de départ […]
        Vous ne pourrez le dépasser en aucun cas": a hero healed past what he
        began with keeps only what he began with.
        """
        moved = self.vie_actuelle + self._delta(change)
        return max(0, min(self.vie_max, moved))

    def _moved_items(self, change: Change) -> dict[str, tuple[str, int]]:
        """The bag after a change, an item dropped once none of it is left."""
        items = dict(self.items)
        label, held = items.get(change.element, (change.label, 0))
        left = 0 if change.takes_everything and change.sign < 0 else (
            held + self._delta(change)
        )
        if left <= 0:
            items.pop(change.element, None)
        else:
            items[change.element] = (label, left)
        return items

    def _delta(self, change: Change) -> int:
        """How far a change moves a number, and in which direction.

        `takes_everything` is answered by the caller for a characteristic —
        losing "all" of one's gold empties the purse — and by `_moved_items` for
        a thing in the bag.
        """
        if change.takes_everything:
            return -self._held(change.element) if change.sign < 0 else 0
        return change.sign * (change.amount or 0)

    def _held(self, element: str) -> int:
        """How much of a characteristic the hero has right now.

        Only ever asked of the three: a thing in the bag that is taken away
        entirely is emptied by `_moved_items`, which sets it to zero without
        needing to know how much was there.

        Args:
            element: One of `CHARACTERISTICS`.

        Returns:
            The amount held.

        Raises:
            KeyError: If asked for anything else, which would be a bug here.
        """
        return {
            GOLD: self.gold,
            LIFE: self.vie_actuelle,
            STRENGTH: self.force,
        }[element]


def read_changes(choice: dict[str, object]) -> list[Change]:
    """Every gain and loss a choice carries, gains first.

    Args:
        choice: One choice, as the book stores it.

    Returns:
        The changes, each knowing whether it adds or takes away.
    """
    changes: list[Change] = []
    for key, sign in (("gains", 1), ("losses", -1)):
        for entry in choice.get(key) or []:
            if isinstance(entry, dict):
                changes.append(_read_change(entry, sign))
    return changes


def starting_gold(rng: random.Random | None = None) -> tuple[int, list[Roll]]:
    """Roll the purse the hero sets out with: two throws of two dice.

    Args:
        rng: Source of chance.

    Returns:
        The total, 4 to 24, and the throws that made it.
    """
    throws = [roll_2d6(rng) for _ in range(GOLD_THROWS)]
    return sum(throw.total for throw in throws), throws


def _read_change(entry: dict[str, object], sign: int) -> Change:
    """Build one change from the dict the pipeline wrote.

    Args:
        entry: One entry of a choice's `gains` or `losses`.
        sign: `1` for a gain, `-1` for a loss.

    Returns:
        The change.
    """
    element = str(entry.get("element") or "")
    amount = entry.get("amount")
    return Change(
        element=element,
        label=str(entry.get("label_fr") or element),
        amount=int(amount) if isinstance(amount, int) else None,
        condition=_text(entry.get("condition")),
        note=_text(entry.get("note")),
        sign=sign,
    )


def pluralised(label: str, count: int) -> str:
    """A French label for a count: `2 points de Vie`, not `2 point de Vies`.

    French marks the plural on the head noun, which is the first word here —
    "pièce d'or" becomes "pièces d'or" and "point de Vie" becomes "points de
    Vie". Every label in the book is a head noun followed by a complement, so
    the first word is the one to move.

    Args:
        label: The label as the book writes it, in the singular.
        count: How many.

    Returns:
        The label, pluralised where it should be.
    """
    if count <= 1 or not label:
        return label
    head, _, rest = label.partition(" ")
    if head.endswith(("s", "x")):
        return label
    return f"{head}s {rest}".strip()


def _text(value: object) -> str | None:
    """A field's text, or `None` when it holds nothing worth reading."""
    written = str(value).strip() if value is not None else ""
    return written or None
