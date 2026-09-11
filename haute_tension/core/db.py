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
    DEFAULT_MODE,
    Mode,
    damage_adjustment,
    generate_character,
    named_mode,
)
from haute_tension.core.combat import (
    ENEMY,
    FIGHTS_WITH_SPECIAL_RULES,
    ONGOING,
    Combatant,
    combat_status,
    resolve_assault,
)
from haute_tension.core.logs.general_log import event, note
from haute_tension.application.models.game import Combat as CombatDict
from haute_tension.application.models.game import FallenHero
from haute_tension.application.models.game import GameDict
from haute_tension.application.models.inspection import InspectionDict
from haute_tension.core.config import current_db_name
from haute_tension.core.inventory import (
    STARTING_ITEMS,
    Change,
    Holdings,
    read_changes,
    starting_gold,
)
from haute_tension.core.models import (
    Game,
    GameAssault,
    GameCombat,
    GameEnemy,
    GameExchange,
    GameItem,
    InspectionComment,
    PageInspection,
    PageView,
    PendingChange,
)
from haute_tension.core.models.page_inspection import OPEN, STATUSES

# What every caller catches around a database call. Two families rather than
# one: pymongo raises when the server cannot be reached or refuses a command,
# mongoengine when a document does not fit its model. Both mean "the database
# did not do what was asked", and every caller reports them the same way.
DatabaseError = (PyMongoError, MongoEngineException)

# What `connect_db` raises when there is no database to reach: none configured,
# or one configured wrong. An `EnvironmentError` at heart — it is the environment
# that is wrong — but a class of its own so the error handler can catch exactly
# this and not every `OSError` a request might raise.
class DatabaseUnavailable(EnvironmentError):
    """There is no database to talk to, and no request can pretend otherwise."""


# Every way the database can fail a request, which is what the application turns
# into one page: the driver refusing or timing out, and there being nothing
# configured to refuse in the first place.
DatabaseFailure = DatabaseError + (DatabaseUnavailable,)

# Long enough for a local mongod, short enough that a dead server shows up as an
# error rather than as a hung request.
SERVER_SELECTION_TIMEOUT_MS = 3000

# How many pages of reading history are kept. What MAX_PAGE_HISTORY used to bound
# in last_pages.json, now applied when the history is read.
MAX_PAGE_HISTORY = 10

# How many of the fallen the memorial remembers.
MAX_FALLEN_HEROES = 20

# The database the current connection was opened on, so a run that switches
# APP_ENV mid-flight (the tests do) reconnects instead of reading the wrong one.
_connected_to: str | None = None


def connect_db() -> None:
    """Register the connection the models use, once per database name.

    Every function below calls this first, which is also the single seam the
    tests replace: with it stubbed out, nothing here ever reaches for a real
    server.

    Raises:
        DatabaseUnavailable: If MONGO_URI is unset, or names a database other
            than the one APP_ENV asks for.
    """
    global _connected_to
    db_name = current_db_name()
    if _connected_to == db_name:
        return

    mongo_uri = os.environ.get("MONGO_URI")
    if not mongo_uri:
        raise DatabaseUnavailable(
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
        raise DatabaseUnavailable(
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


def record_page_view(book: str, page: str, game_id: str | None = None) -> bool:
    """Note that a page was asked for, unless it is already the last one read.

    Asking for the page one is already on is a reload, not a move, and a trail
    reading `22 › 22 › 22` says less than one reading `22`. Only *consecutive*
    repeats are dropped: coming back to a page after going elsewhere is a loop in
    the story, which is worth seeing.

    Args:
        book: The book being read, as `"<series>/<book>"`.
        page: The page number asked for, whether or not the book has it.
        game_id: The play-through that asked, when there is one.

    Returns:
        Whether a visit was actually recorded.
    """
    connect_db()
    if _latest_page(book) == page:
        return False

    PageView(
        book=book,
        game=game_id,
        page=page,
        viewed_at=datetime.now(timezone.utc),
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


def last_page_read(game_id: str) -> str | None:
    """The story page one play-through was on last.

    What the sheet offers as the way back; the sheet itself records no visit, so
    this is still the last paragraph of the story that was open. Scoped to the
    play-through and not to the book: two heroes of the same book each stopped
    somewhere of their own, and sending the living one back to where a dead one
    fell would be worse than offering nothing. A game belongs to one book, so
    naming the game is enough.

    Args:
        game_id: The play-through to answer for.

    Returns:
        The page number, or `None` when that hero has read nothing yet.
    """
    connect_db()
    latest = PageView.objects(game=game_id).order_by("-viewed_at", "-id").first()
    return latest.page if latest else None


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


def start_game(
    book: str,
    *,
    mode: Mode = DEFAULT_MODE,
    rng: random.Random | None = None,
) -> GameDict:
    """Roll up a hero and open a game for him.

    The only place the hero's Force and Vie are ever rolled: every later read
    loads what was written here, so reopening a game never re-rolls it.

    Args:
        book: The book being played, as `"<series>/<book>"`.
        mode: How to roll the hero up — by the book, or the easy way. Named
            rather than positional, so it can never be mistaken for the `rng`
            that used to sit in its place.
        rng: Source of chance, seeded by the tests.

    Returns:
        The freshly created game.
    """
    connect_db()
    character = generate_character(mode, rng)
    gold, gold_throws = starting_gold(rng)
    game = Game(
        id=uuid.uuid4().hex,
        book=book,
        mode=mode.name,
        force=character.force,
        vie_max=character.vie_max,
        vie_actuelle=character.vie_actuelle,
        force_dice=list(character.force_roll.dice),
        vie_dice=list(character.vie_roll.dice),
        gold=gold,
        gold_dice=[die for throw in gold_throws for die in throw.dice],
        items=[
            GameItem(element=element, label=label, count=count)
            for element, label, count in STARTING_ITEMS
        ],
        created_at=datetime.now(timezone.utc),
    )
    game.save(force_insert=True)
    event(
        "Hero rolled up",
        game=game.id,
        book=book,
        mode=mode.name,
        force=game.force,
        vie=game.vie_max,
        force_dice=list(game.force_dice),
        vie_dice=list(game.vie_dice),
        gold=gold,
    )
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
    event(
        "Combat armed",
        game=game.id,
        page=page,
        fight_type=game.combat.fight_type,
        enemies=[enemy.name for enemy in enemies],
        special_rules=game.combat.has_special_rules,
    )
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
    _lay_to_rest(game, game.combat.page, _killed_by(assault))
    game.save()
    note(
        "Assault played",
        game=game.id,
        page=game.combat.page,
        number=assault.number,
        hero_attack_force=assault.hero_attack_force,
        hero_vie=game.vie_actuelle,
        exchanges=[
            {
                "enemy": exchange.enemy_name,
                "attack_force": exchange.enemy_attack_force,
                "winner": exchange.winner,
                "damage": exchange.damage,
                "divine_judgement": exchange.divine_judgement,
            }
            for exchange in assault.exchanges
        ],
    )
    if game.combat.status != ONGOING:
        event(
            "Combat decided",
            game=game.id,
            page=game.combat.page,
            outcome=game.combat.status,
            assaults=len(game.combat.assaults),
            hero_vie=game.vie_actuelle,
        )
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
    decided = game.combat.status if game.combat else None
    game.combat = None
    game.save()
    note("Combat left", game=game.id, outcome=decided, hero_vie=game.vie_actuelle)
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
    mode = named_mode(game.mode)
    return {
        "id": game.id,
        "book": game.book,
        "mode": mode.name,
        "mode_label": mode.label,
        "force_throw": mode.force.notation,
        "vie_throw": mode.vie.notation,
        "force_base": mode.force.base,
        "vie_base": mode.vie.base,
        "force": game.force,
        "vie_max": game.vie_max,
        "vie_actuelle": game.vie_actuelle,
        "force_dice": list(game.force_dice or []),
        "vie_dice": list(game.vie_dice or []),
        "damage_adjustment": _hero_damage_adjustment(game.force),
        "gold": game.gold or 0,
        "gold_dice": list(game.gold_dice or []),
        "bag": [
            {"element": item.element, "label": item.label, "count": item.count}
            for item in game.items
        ],
        "pending": [
            _pending_dict(index, waiting)
            for index, waiting in enumerate(game.pending)
        ],
        "combat": _combat_dict(game.combat) if game.combat else None,
        "is_dead": game.is_dead,
        "died_on_page": game.died_on_page,
        "died_of": game.died_of,
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
    return damage_adjustment(force)


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


def follow_choice(game_id: str, page: str, choice: dict[str, object]) -> GameDict | None:
    """Apply what following a choice costs and gives, and say what is left over.

    The unconditional changes are applied here and now. A change carrying a
    condition — or an amount the page asks to be rolled — is not: it goes to
    `pending`, where the reader decides. See `core.inventory`.

    Args:
        game_id: The game being played.
        page: The page the choice was taken from.
        choice: The choice, as the book stores it.

    Returns:
        The game afterwards, or `None` when it is unknown.
    """
    game = _game_document(game_id)
    if game is None:
        return None

    applied, waiting = [], []
    for change in read_changes(choice):
        (waiting if change.is_conditional else applied).append(change)

    if applied:
        _write_holdings(game, _holdings(game), applied)
        _lay_to_rest(game, page, "les épreuves du chemin")
    game.pending = [_pending_document(change, page) for change in waiting]
    game.save()
    note(
        "Choice followed",
        game=game.id,
        page=page,
        goto=choice.get("goto"),
        applied=[change.described() for change in applied],
        pending=[change.described() for change in waiting],
    )
    return _game_dict(game)


def apply_pending(game_id: str, index: int) -> GameDict | None:
    """Apply one waiting change, because the reader says it applies.

    Args:
        game_id: The game being played.
        index: Which waiting change, as the page numbered them.

    Returns:
        The game afterwards, or `None` when the game or the change is unknown.
    """
    game = _game_document(game_id)
    if game is None or not 0 <= index < len(game.pending):
        return None

    waiting = game.pending[index]
    _write_holdings(game, _holdings(game), [_pending_change(waiting)])
    _lay_to_rest(game, waiting.page, "les épreuves du chemin")
    del game.pending[index]
    game.save()
    note("Pending change applied", game=game.id, element=waiting.element)
    return _game_dict(game)


def dismiss_pending(game_id: str, index: int | None = None) -> GameDict | None:
    """Wave a waiting change away, or all of them.

    Args:
        game_id: The game being played.
        index: Which one; `None` for every one of them.

    Returns:
        The game afterwards, or `None` when the game or the change is unknown.
    """
    game = _game_document(game_id)
    if game is None:
        return None
    if index is None:
        game.pending = []
    elif 0 <= index < len(game.pending):
        del game.pending[index]
    else:
        return None
    game.save()
    return _game_dict(game)


def _killed_by(assault: object) -> str:
    """What killed the hero this assault, named for the memorial.

    Args:
        assault: The assault just played.

    Returns:
        The adversary that struck the last blow, or the fight itself when no
        exchange settled it.
    """
    for exchange in reversed(assault.exchanges):
        if exchange.winner == ENEMY:
            return exchange.enemy_name
    return "un combat"


def _lay_to_rest(game: Game, page: str | None, cause: str) -> None:
    """Stamp a hero's death, once, the moment his Vie reaches zero.

    Called after every write that can empty it. A hero already laid to rest is
    left alone: the first death is the one that counts, and nothing afterwards
    should move the date on the stone.

    Args:
        game: The document, not saved here.
        page: Where he fell, when it is known.
        cause: What killed him, in French, for the memorial.
    """
    if game.vie_actuelle > 0 or game.is_dead:
        return
    game.died_at = datetime.now(timezone.utc)
    game.died_on_page = page
    game.died_of = cause
    event(
        "Hero fell",
        game=game.id,
        page=page,
        cause=cause,
        force=game.force,
        gold=game.gold or 0,
    )


def fallen_heroes(book: str, limit: int = MAX_FALLEN_HEROES) -> list[FallenHero]:
    """The heroes who did not come back, most recent first.

    Args:
        book: The book they died in, as `"<series>/<book>"`.
        limit: How many to remember.

    Returns:
        One epitaph each.
    """
    connect_db()
    games = (
        Game.objects(book=book, died_at__ne=None)
        .order_by("-died_at", "-id")
        .limit(limit)
    )
    return [_fallen_dict(game) for game in games]


def _fallen_dict(game: Game) -> FallenHero:
    """One dead hero as the memorial lists him."""
    return {
        "id": game.id,
        "mode_label": named_mode(game.mode).label,
        "force": game.force,
        "vie_max": game.vie_max,
        "gold": game.gold or 0,
        "bag": [
            {"element": item.element, "label": item.label, "count": item.count}
            for item in game.items
        ],
        "died_on_page": game.died_on_page,
        "died_of": game.died_of,
        "died_at": game.died_at.isoformat() if game.died_at else None,
    }


def _holdings(game: Game) -> Holdings:
    """What the hero is and carries, as `core.inventory` reads it."""
    return Holdings(
        force=game.force,
        vie_max=game.vie_max,
        vie_actuelle=game.vie_actuelle,
        gold=game.gold or 0,
        items={item.element: (item.label, item.count) for item in game.items},
    )


def _write_holdings(
    game: Game, holdings: Holdings, changes: list[Change]
) -> None:
    """Apply changes to a hero and write the result onto the document.

    Args:
        game: The document to change; not saved here.
        holdings: What the hero is and carries before the changes.
        changes: The changes to apply, in order.
    """
    for change in changes:
        holdings = holdings.with_change(change)
    game.force = holdings.force
    game.vie_actuelle = holdings.vie_actuelle
    game.gold = holdings.gold
    game.items = [
        GameItem(element=element, label=label, count=count)
        for element, (label, count) in holdings.items.items()
    ]


def _pending_document(change: Change, page: str) -> PendingChange:
    """Keep a change the reader has yet to rule on."""
    return PendingChange(
        element=change.element,
        label=change.label,
        amount=change.amount,
        condition=change.condition,
        note=change.note,
        sign=change.sign,
        page=page,
    )


def _pending_change(waiting: PendingChange) -> Change:
    """Read a waiting change back as `core.inventory` understands it."""
    return Change(
        element=waiting.element,
        label=waiting.label,
        amount=waiting.amount,
        condition=waiting.condition,
        note=waiting.note,
        sign=waiting.sign or 1,
    )


def _pending_dict(index: int, waiting: PendingChange) -> dict[str, object]:
    """A waiting change as the page shows it, numbered so it can be acted on."""
    change = _pending_change(waiting)
    return {
        "index": index,
        "element": waiting.element,
        "label": waiting.label,
        "amount": waiting.amount,
        "condition": waiting.condition,
        "note": waiting.note,
        "sign": waiting.sign or 1,
        "page": waiting.page,
        "described": change.described(),
    }


def flag_page(
    book: str, path: str, page_title: str | None, text: str
) -> InspectionDict:
    """Flag a page for inspection, or add to the file already open on it.

    Find-or-create on `(book, path)`: the first report opens the file, every
    later one appends a comment. A resolved page that is flagged again is
    reopened — a new remark means someone still sees a problem.

    Args:
        book: The book the page belongs to, as `"<series>/<book>"`.
        path: The page as the browser had it, `"/book/22"`.
        page_title: What its tab said, kept for the list.
        text: The remark, already checked to be non-blank by the caller.

    Returns:
        The file on that page, with the new comment last.
    """
    connect_db()
    now = datetime.now(timezone.utc)
    inspection = PageInspection.objects(book=book, path=path).first()
    if inspection is None:
        inspection = PageInspection(
            id=uuid.uuid4().hex, book=book, path=path, created_at=now
        )
    inspection.page_title = page_title or inspection.page_title
    inspection.status = OPEN
    inspection.updated_at = now
    inspection.comments.append(InspectionComment(text=text, created_at=now))
    inspection.save()
    event(
        "Page flagged",
        inspection=inspection.id,
        book=book,
        path=path,
        comments=len(inspection.comments),
        text=text,
    )
    return _inspection_dict(inspection)


def flagged_pages(book: str, status: str | None = None) -> list[InspectionDict]:
    """The pages flagged in one book, most recently commented first.

    Args:
        book: The book, as `"<series>/<book>"`.
        status: Keep only the files in that state; `None` lists them all.

    Returns:
        One file per flagged page.
    """
    connect_db()
    query = PageInspection.objects(book=book)
    if status is not None:
        query = query.filter(status=status)
    return [
        _inspection_dict(inspection)
        for inspection in query.order_by("-updated_at", "-id")
    ]


def find_inspection(inspection_id: str | None) -> InspectionDict | None:
    """Load the file on one flagged page, or `None` when there is none.

    Args:
        inspection_id: The id carried in the URL.

    Returns:
        The file, or `None` when the id is unknown or empty.
    """
    if not inspection_id:
        return None
    connect_db()
    inspection = PageInspection.objects(id=inspection_id).first()
    return _inspection_dict(inspection) if inspection else None


def set_inspection_status(inspection_id: str, status: str) -> InspectionDict | None:
    """Mark a flagged page resolved, or open it again.

    Args:
        inspection_id: The id carried in the URL.
        status: `"open"` or `"resolved"`.

    Returns:
        The file as it now stands, or `None` when the id is unknown.

    Raises:
        ValueError: If the status is neither.
    """
    if status not in STATUSES:
        raise ValueError(f"unknown inspection status {status!r}")
    connect_db()
    inspection = PageInspection.objects(id=inspection_id).first()
    if inspection is None:
        return None
    inspection.status = status
    inspection.updated_at = datetime.now(timezone.utc)
    inspection.save()
    event(
        "Flagged page status changed",
        inspection=inspection.id,
        path=inspection.path,
        status=status,
    )
    return _inspection_dict(inspection)


def _inspection_dict(inspection: PageInspection) -> InspectionDict:
    """One flagged page as the routes read it."""
    return {
        "id": inspection.id,
        "book": inspection.book,
        "path": inspection.path,
        "page_title": inspection.page_title,
        "status": inspection.status,
        "comments": [
            {"text": comment.text, "created_at": comment.created_at.isoformat()}
            for comment in inspection.comments
        ],
        "created_at": inspection.created_at.isoformat(),
        "updated_at": inspection.updated_at.isoformat(),
    }
