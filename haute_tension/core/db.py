"""The MongoDB database: the connection, and every read and write.

The only module here that talks to the database — `core.config` is pure and
`core.models` only describes the collection. Callers get plain values back:
nothing outside this module and `core/models/` ever holds a document object,
which is what keeps the Flask routes free of the ORM.

**Only the reading history is stored.** The book is static — 668 pages that
change only when the import pipeline is re-run — so `core.story` reads it off
disk into memory at startup, and it is deliberately not in here: putting it in a
database would buy nothing and would make a page unservable without a reachable
server.

The connection is opened on the first call rather than at import time, so
importing `core.db` never needs a reachable server — and a test can stand a fake
one in by replacing `connect_db()`.

The history is **scoped to a book**, named `"<series>/<book>"`: the functions
below take a `book`, and it is part of every query, so a second book never shows
up in the first one's history.
"""

import os
import random
import uuid
from datetime import datetime, timezone

from mongoengine import connect, disconnect
from mongoengine import get_db as mongoengine_db
from mongoengine.errors import MongoEngineException
from pymongo.errors import PyMongoError

from haute_tension.core.character import (
    FORCE_DAMAGE_ADJUSTMENTS,
    generate_character,
)
from haute_tension.core.combat import (
    FIGHTS_WITH_SPECIAL_RULES,
    ONGOING,
    Combatant,
    combat_status,
    resolve_assault,
)
from haute_tension.application.models.game import Combat as CombatDict
from haute_tension.application.models.game import GameDict
from haute_tension.core.config import current_db_name
from haute_tension.core.models import (
    Game,
    GameAssault,
    GameCombat,
    GameEnemy,
    GameExchange,
    PageView,
)

# What every caller catches around a database call. Two families rather than
# one: pymongo raises when the server cannot be reached or refuses a command,
# mongoengine when a document does not fit its model. Both mean "the database
# did not do what was asked", and every caller reports them the same way.
DatabaseError = (PyMongoError, MongoEngineException)

# What a caller catches when it can do without the database entirely. Wider than
# DatabaseError by one case, and it is the case that matters most: a reader who
# has never configured a server at all gets EnvironmentError out of connect_db(),
# not a driver error, and would otherwise be unable to read a book that is sitting
# on disk. Anything that only *reads better* with a history catches this; anything
# that exists to write one catches DatabaseError and reports the failure.
HistoryUnavailable = DatabaseError + (EnvironmentError,)

# Long enough for a local mongod, short enough that a dead server shows up as an
# error rather than as a hung request.
SERVER_SELECTION_TIMEOUT_MS = 3000

# How many pages of reading history are kept. What MAX_PAGE_HISTORY used to bound
# in last_pages.json, now applied when the history is read.
MAX_PAGE_HISTORY = 10

# The database the current connection was opened on, so a run that switches
# APP_ENV mid-flight (the tests do) reconnects instead of reading the wrong one.
_connected_to: str | None = None


def connect_db() -> None:
    """Register the connection the models use, once per database name.

    Every function below calls this first, which is also the single seam the
    tests replace: with it stubbed out, nothing here ever reaches for a real
    server.

    Raises:
        EnvironmentError: If MONGO_URI is unset, or names a database other than
            the one APP_ENV asks for.
    """
    global _connected_to
    db_name = current_db_name()
    if _connected_to == db_name:
        return

    mongo_uri = os.environ.get("MONGO_URI")
    if not mongo_uri:
        raise EnvironmentError(
            "Missing MONGO_URI environment variable. "
            "Copy .env.example to .env and fill it in."
        )

    if _connected_to is not None:
        disconnect()
    connect(
        db=db_name,
        host=mongo_uri,
        serverSelectionTimeoutMS=SERVER_SELECTION_TIMEOUT_MS,
        # Explicit only to keep pymongo from warning about its legacy default;
        # no UUID is ever stored here.
        uuidRepresentation="standard",
    )

    # A database named in MONGO_URI ("…:27017/somewhere") wins over `db` above,
    # and would quietly take a dev run onto another database. APP_ENV is what
    # picks it.
    connected = mongoengine_db().name
    if connected != db_name:
        disconnect()
        raise EnvironmentError(
            f"MONGO_URI points at the database '{connected}', but APP_ENV asks "
            f"for '{db_name}'. Leave the database out of MONGO_URI."
        )

    _connected_to = db_name


def reset_connection() -> None:
    """Close the open connection, so the next call reconnects.

    Only the tests need this: a process normally works on one database for its
    whole life. The connection is dropped and not merely forgotten, because
    mongoengine registers it under a fixed alias and refuses to open a second
    one under the same name.
    """
    global _connected_to
    if _connected_to is not None:
        disconnect()
    _connected_to = None


def record_page_view(book: str, page: str) -> bool:
    """Note that a page was asked for, unless it is already the last one read.

    Asking for the page one is already on is a reload, not a move, and a trail
    reading `22 › 22 › 22` says less than one reading `22`. Only *consecutive*
    repeats are dropped: coming back to a page after going elsewhere is a loop in
    the story, which is worth seeing.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        page: The page number asked for, whether or not the book has it.

    Returns:
        Whether a visit was actually recorded.
    """
    connect_db()
    if _latest_page(book) == page:
        return False

    PageView(
        book=book, page=page, viewed_at=datetime.now(timezone.utc)
    ).save(force_insert=True)
    return True


def last_pages(book: str, limit: int = MAX_PAGE_HISTORY) -> list[str]:
    """Return the pages most recently asked for, oldest first.

    Bounding happens here rather than on write, so recording a visit stays a
    plain insert. The result reads like the old `last_pages.json` did.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        limit: How many pages to keep at most.

    Returns:
        Up to `limit` page numbers, oldest first.
    """
    connect_db()
    views = PageView.objects(book=book).order_by("-viewed_at", "-id").limit(limit)
    return [view.page for view in views][::-1]


def _latest_page(book: str) -> str | None:
    """Return the page most recently read, or `None` when nothing has been.

    Args:
        book: The book being read, as `"<series>/<book>"`.

    Returns:
        The last page number recorded for that book.
    """
    latest = PageView.objects(book=book).order_by("-viewed_at", "-id").first()
    return latest.page if latest else None


def get_oldest_page(book: str) -> str | None:
    """Return the oldest retained page number.

    Args:
        book: The book being read, as `"<series>/<book>"`.

    Returns:
        The oldest page still in the bounded history, or `None` when nothing has
        been read yet.
    """
    history = last_pages(book)
    return history[0] if history else None


def start_game(book: str, rng: random.Random | None = None) -> GameDict:
    """Roll up a hero and open a game for him.

    The only place the hero's Force and Vie are ever rolled: every later read
    loads what was written here, so reopening a game never re-rolls it.

    Args:
        book: The book being played, as `"<series>/<book>"`.
        rng: Source of chance, seeded by the tests.

    Returns:
        The freshly created game.
    """
    connect_db()
    character = generate_character(rng)
    game = Game(
        id=uuid.uuid4().hex,
        book=book,
        force=character.force,
        vie_max=character.vie_max,
        vie_actuelle=character.vie_actuelle,
        force_dice=list(character.force_roll.dice),
        vie_dice=list(character.vie_roll.dice),
        created_at=datetime.now(timezone.utc),
    )
    game.save(force_insert=True)
    return _game_dict(game)


def find_game(game_id: str | None) -> GameDict | None:
    """Load one game, or `None` — a blank id matches nothing.

    Args:
        game_id: The id carried in the session cookie.

    Returns:
        The game, or `None` when it is unknown or the id is empty.
    """
    if not game_id:
        return None
    connect_db()
    game = Game.objects(id=game_id).first()
    return _game_dict(game) if game else None


def begin_combat(
    game_id: str, page: str, fight: dict[str, object]
) -> GameDict | None:
    """Put the hero into the fight a page holds, unless he is already in one.

    Re-entering a page mid-fight must not restart it — a reload would otherwise
    hand back full-health adversaries — so an open combat on the same page is
    left exactly as it stands.

    Args:
        game_id: The game to arm.
        page: The page number the fight belongs to.
        fight: The page's `fight` object, as the book stores it.

    Returns:
        The game with a combat on it, or `None` when the game is unknown or the
        fight has no adversaries to field.
    """
    game = _game_document(game_id)
    if game is None:
        return None
    if game.combat is not None and game.combat.page == page:
        return _game_dict(game)

    enemies = _combat_enemies(fight)
    if not enemies:
        return None

    outcome = fight.get("outcome") or {}
    game.combat = GameCombat(
        page=page,
        fight_type=str(fight.get("fight_type") or ""),
        enemies=enemies,
        assaults=[],
        status=ONGOING,
        on_victory=_outcome_branch(outcome.get("on_victory")),
        on_defeat=_outcome_branch(outcome.get("on_defeat")),
        on_flee=_outcome_branch(outcome.get("on_flee")),
        has_special_rules=page in FIGHTS_WITH_SPECIAL_RULES,
    )
    game.save()
    return _game_dict(game)


def play_assault(
    game_id: str, rng: random.Random | None = None
) -> GameDict | None:
    """Play one assault of the open fight and write the result back.

    Args:
        game_id: The game to advance.
        rng: Source of chance, seeded by the tests.

    Returns:
        The game after the assault, or `None` when there is no fight to advance
        — no game, no combat, or one already decided.
    """
    game = _game_document(game_id)
    if game is None or game.combat is None or game.combat.status != ONGOING:
        return None

    hero = Combatant(
        name="Prêtre Jean",
        force=game.force,
        vie_max=game.vie_max,
        vie_actuelle=game.vie_actuelle,
        damage_adjustment=_hero_damage_adjustment(game.force),
    )
    enemies = [
        Combatant(
            name=enemy.name,
            force=enemy.force,
            vie_max=enemy.vie_max,
            vie_actuelle=enemy.vie_actuelle,
            damage_adjustment=enemy.damage_adjustment or 0,
        )
        for enemy in game.combat.enemies
    ]

    hero, enemies, assault = resolve_assault(
        hero,
        enemies,
        game.combat.fight_type,
        number=len(game.combat.assaults) + 1,
        rng=rng,
    )

    game.vie_actuelle = hero.vie_actuelle
    for stored, fought in zip(game.combat.enemies, enemies):
        stored.vie_actuelle = fought.vie_actuelle
    game.combat.assaults.append(_stored_assault(assault))
    game.combat.status = combat_status(hero, enemies)
    game.save()
    return _game_dict(game)


def end_combat(game_id: str) -> GameDict | None:
    """Take the hero out of the fight, leaving his Vie where the fight left it.

    Args:
        game_id: The game to clear.

    Returns:
        The game with no combat on it, or `None` when it is unknown.
    """
    game = _game_document(game_id)
    if game is None:
        return None
    game.combat = None
    game.save()
    return _game_dict(game)


def _game_document(game_id: str | None) -> Game | None:
    """The game as a document, for the three functions that write it back.

    The one place inside this module that keeps a document: everything public
    converts before returning, so no caller ever holds one.

    Args:
        game_id: The game to load.

    Returns:
        The document, or `None` when the id is unknown or empty.
    """
    if not game_id:
        return None
    connect_db()
    return Game.objects(id=game_id).first()


def _game_dict(game: Game) -> GameDict:
    """Turn a game document into the dict the routes and templates read."""
    return {
        "id": game.id,
        "book": game.book,
        "force": game.force,
        "vie_max": game.vie_max,
        "vie_actuelle": game.vie_actuelle,
        "force_dice": list(game.force_dice or []),
        "vie_dice": list(game.vie_dice or []),
        "damage_adjustment": _hero_damage_adjustment(game.force),
        "combat": _combat_dict(game.combat) if game.combat else None,
    }


def _combat_dict(combat: GameCombat) -> CombatDict:
    """Turn an open fight into the dict the combat template reads."""
    return {
        "page": combat.page,
        "fight_type": combat.fight_type,
        "status": combat.status,
        "on_victory": combat.on_victory,
        "on_defeat": combat.on_defeat,
        "on_flee": combat.on_flee,
        "has_special_rules": bool(combat.has_special_rules),
        "enemies": [
            {
                "name": enemy.name,
                "force": enemy.force,
                "vie_max": enemy.vie_max,
                "vie_actuelle": enemy.vie_actuelle,
                "damage_adjustment": enemy.damage_adjustment or 0,
            }
            for enemy in combat.enemies
        ],
        "assaults": [
            {
                "number": assault.number,
                "hero_dice": list(assault.hero_dice or []),
                "hero_attack_force": assault.hero_attack_force,
                "exchanges": [
                    {
                        "enemy_name": exchange.enemy_name,
                        "enemy_force": exchange.enemy_force,
                        "enemy_dice": list(exchange.enemy_dice or []),
                        "enemy_attack_force": exchange.enemy_attack_force,
                        "winner": exchange.winner,
                        "damage": exchange.damage or 0,
                        "divine_judgement": bool(exchange.divine_judgement),
                    }
                    for exchange in assault.exchanges
                ],
            }
            for assault in combat.assaults
        ],
    }


def _hero_damage_adjustment(force: int) -> int:
    """How much a Force of this size adds to the hero's blows."""
    return FORCE_DAMAGE_ADJUSTMENTS.get(force, 0)


def _combat_enemies(fight: dict[str, object]) -> list[GameEnemy]:
    """Field one adversary per body the fight puts in front of the hero.

    A `count` on an adversary means the book prints one set of statistics for
    several identical creatures ("LEPREUX ... count: 2"), so it is expanded into
    that many adversaries, each with its own Vie to whittle down.

    Args:
        fight: The page's `fight` object.

    Returns:
        The adversaries, at full Vie.
    """
    enemies: list[GameEnemy] = []
    for entry in fight.get("enemies") or []:
        if not isinstance(entry, dict):
            continue
        vie = int(entry.get("vie") or 0)
        for copy_number in range(max(1, int(entry.get("count") or 1))):
            enemies.append(
                GameEnemy(
                    name=_enemy_name(entry, copy_number, entry.get("count")),
                    force=int(entry.get("force") or 0),
                    vie_max=vie,
                    vie_actuelle=vie,
                    damage_adjustment=int(entry.get("damage_adjustment") or 0),
                )
            )
    return enemies


def _enemy_name(entry: dict[str, object], copy_number: int, count: object) -> str:
    """Name one adversary, numbering the copies of a repeated one apart."""
    name = str(entry.get("name") or "adversaire")
    if not count or int(count) < 2:
        return name
    return f"{name} {copy_number + 1}"


def _outcome_branch(value: object) -> str | None:
    """The page (or `"death"`) an outcome points at, or `None` when it has none."""
    return None if value is None else str(value)


def _stored_assault(assault: object) -> GameAssault:
    """Turn an assault the engine produced into the document that keeps it."""
    return GameAssault(
        number=assault.number,
        hero_dice=list(assault.hero_roll.dice),
        hero_attack_force=assault.hero_attack_force,
        exchanges=[
            GameExchange(
                enemy_name=exchange.enemy_name,
                enemy_force=exchange.enemy_force,
                enemy_dice=list(exchange.enemy_roll.dice),
                enemy_attack_force=exchange.enemy_attack_force,
                winner=exchange.winner,
                damage=exchange.damage,
                divine_judgement=exchange.divine_judgement,
            )
            for exchange in assault.exchanges
        ],
    )
