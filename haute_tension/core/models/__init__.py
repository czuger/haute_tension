"""One mongoengine document per collection — the shape of what is stored.

The models describe the collections and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands plain values back, so only this package and `core.db` ever see
a document object.

    page_views -> PageView, one row per page asked for
    games      -> Game, one play-through: the hero and the fight he is in

The book is in neither: it is static and read into memory at startup.
"""

from .game import (
    Game,
    GameAssault,
    GameCombat,
    GameEnemy,
    GameExchange,
    GameItem,
    PendingChange,
)
from .page_view import PageView

__all__ = [
    "Game",
    "GameAssault",
    "GameCombat",
    "GameEnemy",
    "GameExchange",
    "GameItem",
    "PageView",
    "PendingChange",
]
