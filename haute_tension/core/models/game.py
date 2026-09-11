"""One play-through: the hero, and the fight he is in the middle of.

The second thing this application stores, and the first that is *state* rather
than a trace. The book stays on disk and the reading history stays a log; a game
is the row that is read, changed and written back.

The id is an integer the database counts up, and never reuses. It travels in
the session cookie, whose signature is what keeps a reader from writing another
hero's number into it; never reusing one is what keeps an old cookie from ever
naming somebody else's hero.

Real columns: the id; the book every query is scoped to; the hero's Force, Vie
and purse, which may never go below zero and which the database holds to that;
when he died, which the memorial filters and sorts on; and when the row was
created and last changed. His bag is rows of their own, `Item`. **Everything
else is in the blob**:

    mode            "normal" or "easy" — which set of rules rolled him up
    force_dice, vie_dice        the throws that made him, for the sheet
    gold_dice
    pending         [{element, label, amount, condition, note, sign, page}]
    combat          None while he is reading, or {page, fight_type, status,
                    on_victory, on_defeat, on_flee, has_special_rules,
                    enemies: [{name, force, vie_max, vie_actuelle,
                    damage_adjustment}], assaults: [{number, hero_dice,
                    hero_attack_force, exchanges: [{enemy_name, enemy_force,
                    enemy_dice, enemy_attack_force, winner, damage,
                    divine_judgement}]}]}
    died_on_page, died_of       set with `died_at`, see below
    legacy_id       the uuid a game created before migration 001 was known by,
                    kept so an old log line can still be traced to it

`died_at` is unset while he lives, which is what tells a game apart from an
epitaph: a hero abandoned in good health is simply left behind, and does not
join the fallen.
"""

from datetime import datetime

from sqlalchemy import CheckConstraint, Index, Integer, SmallInteger, String
from sqlalchemy.orm import Mapped, mapped_column, relationship

from haute_tension.core.models.base import Base
from haute_tension.core.models.hybrid_document import HybridDocument
from haute_tension.core.models.item import Item
from haute_tension.core.models.timestamped import Timestamped
from haute_tension.core.models.utc_datetime import UtcDateTime


class Game(Timestamped, HybridDocument, Base):
    """A hero, rolled once, and the fight he is in the middle of."""

    __tablename__ = "games"
    __table_args__ = (
        CheckConstraint("force >= 0", name="ck_games_force_not_negative"),
        CheckConstraint("vie_max >= 0", name="ck_games_vie_max_not_negative"),
        CheckConstraint("vie_actuelle >= 0", name="ck_games_vie_actuelle_not_negative"),
        CheckConstraint("gold >= 0", name="ck_games_gold_not_negative"),
        # The memorial: the fallen of one book, most recent first.
        Index("ix_games_book_died_at", "book", "died_at"),
        {"sqlite_autoincrement": True},
    )

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    book: Mapped[str] = mapped_column(String, nullable=False, index=True)

    # Force and Vie stay in the tens and the purse in the hundreds at most.
    # SQLite enforces no width, so the checks above are what hold the floor.
    force: Mapped[int] = mapped_column(SmallInteger, nullable=False)
    vie_max: Mapped[int] = mapped_column(SmallInteger, nullable=False)
    vie_actuelle: Mapped[int] = mapped_column(SmallInteger, nullable=False)
    gold: Mapped[int] = mapped_column(SmallInteger, nullable=False, default=0)

    died_at: Mapped[datetime | None] = mapped_column(UtcDateTime)

    # Loaded with the game rather than on first access, so a game read inside a
    # session can still be turned into a dict once the session has closed.
    items: Mapped[list[Item]] = relationship(
        order_by=Item.id, cascade="all, delete-orphan", lazy="selectin"
    )

    @property
    def is_dead(self) -> bool:
        """Whether this hero has already been laid to rest."""
        return self.died_at is not None

    def __str__(self) -> str:
        state = "mort" if self.is_dead else f"Vie {self.vie_actuelle}/{self.vie_max}"
        mode = (self.data or {}).get("mode")
        return f"{self.book} ({mode}) — Force {self.force}, {state}"
