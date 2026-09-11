"""The SQLite database: the connection, and every read and write.

The only module here that talks to the database — `core.config` is pure and
`core.models` only describes the tables. Callers get plain dicts back: nothing
outside this module and `core/models/` ever holds a row object, which is what
keeps the Flask routes free of the ORM.

**Only the reading history, the play-throughs and the flagged pages are stored.**
The book is static — 668 pages that change only when the import pipeline is
re-run — so `core.story` reads it off disk into memory at startup, and it is
deliberately not in here: putting it in a database would buy nothing.

Every table is hybrid (see `core.models.hybrid_document`): a few real columns
for what a query filters or sorts on, and one JSON blob for the rest. A row is
therefore read as one flat dict, changed as a dict, and written back whole with
`update_from_dict()` — the functions below never touch a column by hand.

The engine is opened on the first call rather than at import time, so importing
`core.db` never needs a database file — and a test can stand an in-memory one
in by replacing `connect_db()`.

The history is **scoped to a book**, named `"<series>/<book>"`: the functions
below take a `book`, and it is part of every query, so a second book never shows
up in the first one's history.
"""

import random
import uuid
from collections.abc import Iterator
from contextlib import contextmanager
from datetime import datetime, timezone

from sqlalchemy import Engine, create_engine, event as sqlalchemy_event, select
from sqlalchemy.exc import SQLAlchemyError
from sqlalchemy.orm import Session

from haute_tension.application.models.game import Combat as CombatDict
from haute_tension.application.models.game import FallenHero, GameDict
from haute_tension.application.models.inspection import InspectionDict
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
from haute_tension.core.config import DATABASE_DIR_VAR, current_db_name, database_path
from haute_tension.core.inventory import (
    STARTING_ITEMS,
    Change,
    Holdings,
    read_changes,
    starting_gold,
)
from haute_tension.core.logs.general_log import event, note
from haute_tension.core.models.base import Base
from haute_tension.core.models.game import Game
from haute_tension.core.models.page_inspection import OPEN, STATUSES, PageInspection
from haute_tension.core.models.page_view import PageView

# What every caller catches around a database call: the driver failing — a
# file that cannot be opened or written, a constraint refused, a query the
# engine will not run. Every caller reports it the same way.
DatabaseError = (SQLAlchemyError,)


# What `connect_db` raises when there is no database to reach: none configured,
# or one configured wrong. An `EnvironmentError` at heart — it is the environment
# that is wrong — but a class of its own so the error handler can catch exactly
# this and not every `OSError` a request might raise.
class DatabaseUnavailable(EnvironmentError):
    """There is no database to talk to, and no request can pretend otherwise."""


# Every way the database can fail a request, which is what the application turns
# into one page: the driver refusing, and there being nothing configured to
# refuse in the first place.
DatabaseFailure = DatabaseError + (DatabaseUnavailable,)

# How many pages of reading history are kept. What MAX_PAGE_HISTORY used to bound
# in last_pages.json, now applied when the history is read.
MAX_PAGE_HISTORY = 10

# How many of the fallen the memorial remembers.
MAX_FALLEN_HEROES = 20

# The row as one flat dict — the columns and the blob merged, the shape
# `HybridDocument.to_dict()` gives and `update_from_dict()` takes back.
State = dict[str, object]

# The engine every session below opens on, and the database it was opened on,
# so a run that switches APP_ENV mid-flight (the tests do) reopens instead of
# reading the wrong file.
_engine: Engine | None = None
_connected_to: str | None = None


def connect_db() -> None:
    """Open the engine on the file APP_ENV picks, once per database name.

    Every function below calls this first, which is also the single seam the
    tests replace: with it stubbed out and `_engine` bound to a database of
    their own, nothing here ever reaches for a file.

    The tables are created if the file does not have them yet: there is no
    migration step, the schema is what the models say.

    Raises:
        DatabaseUnavailable: If DATABASE_DIR is unset, or names a file rather
            than a directory.
    """
    global _engine, _connected_to
    db_name = current_db_name()
    if _connected_to == db_name:
        return

    path = database_path()
    if path is None:
        raise DatabaseUnavailable(
            f"Missing {DATABASE_DIR_VAR} environment variable. "
            f"Copy .env.example to .env and fill it in."
        )
    if path.parent.exists() and not path.parent.is_dir():
        raise DatabaseUnavailable(
            f"{DATABASE_DIR_VAR} points at '{path.parent}', which is not a "
            f"directory. It must name the directory the database files live in."
        )

    reset_connection()
    path.parent.mkdir(parents=True, exist_ok=True)
    _engine = _open_engine(f"sqlite:///{path}")
    Base.metadata.create_all(_engine)
    _connected_to = db_name


def _open_engine(url: str, **options: object) -> Engine:
    """Build an engine on a SQLite URL, with foreign keys enforced.

    SQLite checks foreign keys only when asked to, connection by connection.

    Args:
        url: The database, as `sqlite:///<path>`.
        **options: Passed on to `create_engine`; the tests pick a pool.

    Returns:
        The engine, nothing opened yet.
    """
    engine = create_engine(url, **options)

    @sqlalchemy_event.listens_for(engine, "connect")
    def enforce_foreign_keys(connection: object, _record: object) -> None:
        # `connection` is the driver's own sqlite3 connection, not SQLAlchemy's.
        connection.execute("PRAGMA foreign_keys=ON")

    return engine


def reset_connection() -> None:
    """Dispose of the open engine, so the next call reopens it.

    Only the tests need this: a process normally works on one database for its
    whole life.
    """
    global _engine, _connected_to
    if _engine is not None:
        _engine.dispose()
    _engine = None
    _connected_to = None


@contextmanager
def _session() -> Iterator[Session]:
    """One unit of work: a session that commits when the block ends.

    An exception in the block rolls back and propagates; nothing is committed
    half-done. Rows stay readable after the commit, so a function can build its
    answer from what it just wrote.

    Yields:
        The session every read and write below goes through.
    """
    connect_db()
    with Session(_engine, expire_on_commit=False) as session:
        yield session
        session.commit()


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
    with _session() as session:
        if _latest_page(session, book) == page:
            return False
        session.add(
            PageView.from_dict(
                {
                    "book": book,
                    "game": game_id,
                    "page": page,
                    "viewed_at": datetime.now(timezone.utc),
                }
            )
        )
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
    with _session() as session:
        views = session.scalars(
            select(PageView)
            .where(PageView.book == book)
            .order_by(PageView.viewed_at.desc(), PageView.id.desc())
            .limit(limit)
        )
        return [_page_of(view) for view in views][::-1]


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
    with _session() as session:
        latest = session.scalars(
            select(PageView)
            .where(PageView.game == game_id)
            .order_by(PageView.viewed_at.desc(), PageView.id.desc())
            .limit(1)
        ).first()
        return _page_of(latest) if latest else None


def _latest_page(session: Session, book: str) -> str | None:
    """Return the page most recently read, or `None` when nothing has been.

    Args:
        session: The unit of work the caller is in.
        book: The book being read, as `"<series>/<book>"`.

    Returns:
        The last page number recorded for that book.
    """
    latest = session.scalars(
        select(PageView)
        .where(PageView.book == book)
        .order_by(PageView.viewed_at.desc(), PageView.id.desc())
        .limit(1)
    ).first()
    return _page_of(latest) if latest else None


def _page_of(view: PageView) -> str:
    """The page number a visit was for, out of its blob."""
    return str(view.to_dict()["page"])


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
    character = generate_character(mode, rng)
    gold, gold_throws = starting_gold(rng)
    hero: State = {
        "id": uuid.uuid4().hex,
        "book": book,
        "mode": mode.name,
        "force": character.force,
        "vie_max": character.vie_max,
        "vie_actuelle": character.vie_actuelle,
        "force_dice": list(character.force_roll.dice),
        "vie_dice": list(character.vie_roll.dice),
        "gold": gold,
        "gold_dice": [die for throw in gold_throws for die in throw.dice],
        "items": [
            {"element": element, "label": label, "count": count}
            for element, label, count in STARTING_ITEMS
        ],
        "pending": [],
        "combat": None,
        "created_at": datetime.now(timezone.utc),
        "died_at": None,
    }
    with _session() as session:
        game = Game.from_dict(hero)
        session.add(game)
    event(
        "Hero rolled up",
        game=game.id,
        book=book,
        mode=mode.name,
        force=hero["force"],
        vie=hero["vie_max"],
        force_dice=hero["force_dice"],
        vie_dice=hero["vie_dice"],
        gold=gold,
    )
    return _game_dict(game.to_dict())


def find_game(game_id: str | None) -> GameDict | None:
    """Load one game, or `None` — a blank id matches nothing.

    Args:
        game_id: The id carried in the session cookie.

    Returns:
        The game, or `None` when it is unknown or the id is empty.
    """
    if not game_id:
        return None
    with _session() as session:
        game = session.get(Game, game_id)
        return _game_dict(game.to_dict()) if game else None


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
    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        combat = _combat_of(state)
        if combat is not None and combat.get("page") == page:
            return _game_dict(state)

        enemies = _combat_enemies(fight)
        if not enemies:
            return None

        outcome = fight.get("outcome") or {}
        state["combat"] = {
            "page": page,
            "fight_type": str(fight.get("fight_type") or ""),
            "enemies": enemies,
            "assaults": [],
            "status": ONGOING,
            "on_victory": _outcome_branch(outcome.get("on_victory")),
            "on_defeat": _outcome_branch(outcome.get("on_defeat")),
            "on_flee": _outcome_branch(outcome.get("on_flee")),
            "has_special_rules": page in FIGHTS_WITH_SPECIAL_RULES,
        }
        game.update_from_dict(state)
    event(
        "Combat armed",
        game=game.id,
        page=page,
        fight_type=state["combat"]["fight_type"],
        enemies=[enemy["name"] for enemy in enemies],
        special_rules=state["combat"]["has_special_rules"],
    )
    return _game_dict(game.to_dict())


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
    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        combat = _combat_of(state)
        if combat is None or combat.get("status") != ONGOING:
            return None

        hero, enemies, assault = resolve_assault(
            _hero_combatant(state),
            [_enemy_combatant(enemy) for enemy in combat["enemies"]],
            str(combat.get("fight_type") or ""),
            number=len(combat["assaults"]) + 1,
            rng=rng,
        )

        state["vie_actuelle"] = hero.vie_actuelle
        for stored, fought in zip(combat["enemies"], enemies):
            stored["vie_actuelle"] = fought.vie_actuelle
        combat["assaults"].append(_stored_assault(assault))
        combat["status"] = combat_status(hero, enemies)
        _lay_to_rest(state, combat.get("page"), _killed_by(assault))
        game.update_from_dict(state)
    _log_assault(game.id, combat, assault, state["vie_actuelle"])
    return _game_dict(game.to_dict())


def _log_assault(
    game_id: str, combat: dict[str, object], assault: object, hero_vie: int
) -> None:
    """Write an assault to the log, and the fight's outcome when it decided it."""
    note(
        "Assault played",
        game=game_id,
        page=combat.get("page"),
        number=assault.number,
        hero_attack_force=assault.hero_attack_force,
        hero_vie=hero_vie,
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
    if combat["status"] != ONGOING:
        event(
            "Combat decided",
            game=game_id,
            page=combat.get("page"),
            outcome=combat["status"],
            assaults=len(combat["assaults"]),
            hero_vie=hero_vie,
        )


def end_combat(game_id: str) -> GameDict | None:
    """Take the hero out of the fight, leaving his Vie where the fight left it.

    Args:
        game_id: The game to clear.

    Returns:
        The game with no combat on it, or `None` when it is unknown.
    """
    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        combat = _combat_of(state)
        decided = combat.get("status") if combat else None
        state["combat"] = None
        game.update_from_dict(state)
    note("Combat left", game=game.id, outcome=decided, hero_vie=state["vie_actuelle"])
    return _game_dict(game.to_dict())


def _game_row(session: Session, game_id: str | None) -> Game | None:
    """The game as a row, for the functions that write it back.

    The one place inside this module that fetches a row by id: everything
    public converts before returning, so no caller ever holds one.

    Args:
        session: The unit of work the caller is in.
        game_id: The game to load.

    Returns:
        The row, or `None` when the id is unknown or empty.
    """
    if not game_id:
        return None
    return session.get(Game, game_id)


def _combat_of(state: State) -> dict[str, object] | None:
    """The open fight in a game's state, or `None` while the hero is reading."""
    combat = state.get("combat")
    return combat if isinstance(combat, dict) else None


def _hero_combatant(state: State) -> Combatant:
    """The hero as the combat engine fights him."""
    return Combatant(
        name="Prêtre Jean",
        force=int(state["force"]),
        vie_max=int(state["vie_max"]),
        vie_actuelle=int(state["vie_actuelle"]),
        damage_adjustment=_hero_damage_adjustment(int(state["force"])),
    )


def _enemy_combatant(enemy: dict[str, object]) -> Combatant:
    """One stored adversary as the combat engine fights it."""
    return Combatant(
        name=str(enemy.get("name")),
        force=int(enemy.get("force") or 0),
        vie_max=int(enemy.get("vie_max") or 0),
        vie_actuelle=int(enemy.get("vie_actuelle") or 0),
        damage_adjustment=int(enemy.get("damage_adjustment") or 0),
    )


def _game_dict(state: State) -> GameDict:
    """Turn a game's stored state into the dict the routes and templates read."""
    mode = named_mode(str(state["mode"]))
    force = int(state["force"])
    combat = _combat_of(state)
    return {
        "id": str(state["id"]),
        "book": str(state["book"]),
        "mode": mode.name,
        "mode_label": mode.label,
        "force_throw": mode.force.notation,
        "vie_throw": mode.vie.notation,
        "force_base": mode.force.base,
        "vie_base": mode.vie.base,
        "force": force,
        "vie_max": int(state["vie_max"]),
        "vie_actuelle": int(state["vie_actuelle"]),
        "force_dice": list(state.get("force_dice") or []),
        "vie_dice": list(state.get("vie_dice") or []),
        "damage_adjustment": _hero_damage_adjustment(force),
        "gold": int(state.get("gold") or 0),
        "gold_dice": list(state.get("gold_dice") or []),
        "bag": _bag(state),
        "pending": [
            _pending_dict(index, waiting)
            for index, waiting in enumerate(state.get("pending") or [])
        ],
        "combat": _combat_dict(combat) if combat else None,
        "is_dead": _is_dead(state),
        "died_on_page": state.get("died_on_page"),
        "died_of": state.get("died_of"),
    }


def _bag(state: State) -> list[dict[str, object]]:
    """What the hero carries, one line per kind of thing."""
    return [
        {"element": item["element"], "label": item["label"], "count": item["count"]}
        for item in state.get("items") or []
    ]


def _is_dead(state: State) -> bool:
    """Whether the hero of this state has been laid to rest."""
    return state.get("died_at") is not None


def _combat_dict(combat: dict[str, object]) -> CombatDict:
    """Turn an open fight into the dict the combat template reads."""
    return {
        "page": combat.get("page"),
        "fight_type": combat.get("fight_type"),
        "status": combat.get("status"),
        "on_victory": combat.get("on_victory"),
        "on_defeat": combat.get("on_defeat"),
        "on_flee": combat.get("on_flee"),
        "has_special_rules": bool(combat.get("has_special_rules")),
        "enemies": [
            {
                "name": enemy.get("name"),
                "force": enemy.get("force"),
                "vie_max": enemy.get("vie_max"),
                "vie_actuelle": enemy.get("vie_actuelle"),
                "damage_adjustment": enemy.get("damage_adjustment") or 0,
            }
            for enemy in combat.get("enemies") or []
        ],
        "assaults": [
            {
                "number": assault.get("number"),
                "hero_dice": list(assault.get("hero_dice") or []),
                "hero_attack_force": assault.get("hero_attack_force"),
                "exchanges": [
                    {
                        "enemy_name": exchange.get("enemy_name"),
                        "enemy_force": exchange.get("enemy_force"),
                        "enemy_dice": list(exchange.get("enemy_dice") or []),
                        "enemy_attack_force": exchange.get("enemy_attack_force"),
                        "winner": exchange.get("winner"),
                        "damage": exchange.get("damage") or 0,
                        "divine_judgement": bool(exchange.get("divine_judgement")),
                    }
                    for exchange in assault.get("exchanges") or []
                ],
            }
            for assault in combat.get("assaults") or []
        ],
    }


def _hero_damage_adjustment(force: int) -> int:
    """How much a Force of this size adds to the hero's blows."""
    return damage_adjustment(force)


def _combat_enemies(fight: dict[str, object]) -> list[dict[str, object]]:
    """Field one adversary per body the fight puts in front of the hero.

    A `count` on an adversary means the book prints one set of statistics for
    several identical creatures ("LEPREUX ... count: 2"), so it is expanded into
    that many adversaries, each with its own Vie to whittle down.

    Args:
        fight: The page's `fight` object.

    Returns:
        The adversaries, at full Vie, as the blob stores them.
    """
    enemies: list[dict[str, object]] = []
    for entry in fight.get("enemies") or []:
        if not isinstance(entry, dict):
            continue
        vie = int(entry.get("vie") or 0)
        for copy_number in range(max(1, int(entry.get("count") or 1))):
            enemies.append(
                {
                    "name": _enemy_name(entry, copy_number, entry.get("count")),
                    "force": int(entry.get("force") or 0),
                    "vie_max": vie,
                    "vie_actuelle": vie,
                    "damage_adjustment": int(entry.get("damage_adjustment") or 0),
                }
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


def _stored_assault(assault: object) -> dict[str, object]:
    """Turn an assault the engine produced into what the blob keeps of it."""
    return {
        "number": assault.number,
        "hero_dice": list(assault.hero_roll.dice),
        "hero_attack_force": assault.hero_attack_force,
        "exchanges": [
            {
                "enemy_name": exchange.enemy_name,
                "enemy_force": exchange.enemy_force,
                "enemy_dice": list(exchange.enemy_roll.dice),
                "enemy_attack_force": exchange.enemy_attack_force,
                "winner": exchange.winner,
                "damage": exchange.damage,
                "divine_judgement": exchange.divine_judgement,
            }
            for exchange in assault.exchanges
        ],
    }


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
    applied, waiting = [], []
    for change in read_changes(choice):
        (waiting if change.is_conditional else applied).append(change)

    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        if applied:
            _write_holdings(state, _holdings(state), applied)
            _lay_to_rest(state, page, "les épreuves du chemin")
        state["pending"] = [_pending_entry(change, page) for change in waiting]
        game.update_from_dict(state)
    note(
        "Choice followed",
        game=game.id,
        page=page,
        goto=choice.get("goto"),
        applied=[change.described() for change in applied],
        pending=[change.described() for change in waiting],
    )
    return _game_dict(game.to_dict())


def apply_pending(game_id: str, index: int) -> GameDict | None:
    """Apply one waiting change, because the reader says it applies.

    Args:
        game_id: The game being played.
        index: Which waiting change, as the page numbered them.

    Returns:
        The game afterwards, or `None` when the game or the change is unknown.
    """
    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        pending = list(state.get("pending") or [])
        if not 0 <= index < len(pending):
            return None

        waiting = pending.pop(index)
        _write_holdings(state, _holdings(state), [_pending_change(waiting)])
        _lay_to_rest(state, waiting.get("page"), "les épreuves du chemin")
        state["pending"] = pending
        game.update_from_dict(state)
    note("Pending change applied", game=game.id, element=waiting["element"])
    return _game_dict(game.to_dict())


def dismiss_pending(game_id: str, index: int | None = None) -> GameDict | None:
    """Wave a waiting change away, or all of them.

    Args:
        game_id: The game being played.
        index: Which one; `None` for every one of them.

    Returns:
        The game afterwards, or `None` when the game or the change is unknown.
    """
    with _session() as session:
        game = _game_row(session, game_id)
        if game is None:
            return None
        state = game.to_dict()
        pending = list(state.get("pending") or [])
        if index is None:
            pending = []
        elif 0 <= index < len(pending):
            del pending[index]
        else:
            return None
        state["pending"] = pending
        game.update_from_dict(state)
    return _game_dict(game.to_dict())


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


def _lay_to_rest(state: State, page: str | None, cause: str) -> None:
    """Stamp a hero's death, once, the moment his Vie reaches zero.

    Called after every write that can empty it. A hero already laid to rest is
    left alone: the first death is the one that counts, and nothing afterwards
    should move the date on the stone.

    Args:
        state: The game's state, not written here.
        page: Where he fell, when it is known.
        cause: What killed him, in French, for the memorial.
    """
    if int(state["vie_actuelle"]) > 0 or _is_dead(state):
        return
    state["died_at"] = datetime.now(timezone.utc)
    state["died_on_page"] = page
    state["died_of"] = cause
    event(
        "Hero fell",
        game=state["id"],
        page=page,
        cause=cause,
        force=state["force"],
        gold=state.get("gold") or 0,
    )


def fallen_heroes(book: str, limit: int = MAX_FALLEN_HEROES) -> list[FallenHero]:
    """The heroes who did not come back, most recent first.

    Args:
        book: The book they died in, as `"<series>/<book>"`.
        limit: How many to remember.

    Returns:
        One epitaph each.
    """
    with _session() as session:
        games = session.scalars(
            select(Game)
            .where(Game.book == book, Game.died_at.is_not(None))
            .order_by(Game.died_at.desc(), Game.id.desc())
            .limit(limit)
        )
        return [_fallen_dict(game.to_dict()) for game in games]


def _fallen_dict(state: State) -> FallenHero:
    """One dead hero as the memorial lists him, `died_at` being set."""
    return {
        "id": str(state["id"]),
        "mode_label": named_mode(str(state["mode"])).label,
        "force": int(state["force"]),
        "vie_max": int(state["vie_max"]),
        "gold": int(state.get("gold") or 0),
        "bag": _bag(state),
        "died_on_page": state.get("died_on_page"),
        "died_of": state.get("died_of"),
        "died_at": state["died_at"].isoformat(),
    }


def _holdings(state: State) -> Holdings:
    """What the hero is and carries, as `core.inventory` reads it."""
    return Holdings(
        force=int(state["force"]),
        vie_max=int(state["vie_max"]),
        vie_actuelle=int(state["vie_actuelle"]),
        gold=int(state.get("gold") or 0),
        items={
            item["element"]: (item["label"], item["count"])
            for item in state.get("items") or []
        },
    )


def _write_holdings(state: State, holdings: Holdings, changes: list[Change]) -> None:
    """Apply changes to a hero and write the result onto his state.

    Args:
        state: The game's state to change; not written here.
        holdings: What the hero is and carries before the changes.
        changes: The changes to apply, in order.
    """
    for change in changes:
        holdings = holdings.with_change(change)
    state["force"] = holdings.force
    state["vie_actuelle"] = holdings.vie_actuelle
    state["gold"] = holdings.gold
    state["items"] = [
        {"element": element, "label": label, "count": count}
        for element, (label, count) in holdings.items.items()
    ]


def _pending_entry(change: Change, page: str) -> dict[str, object]:
    """Keep a change the reader has yet to rule on."""
    return {
        "element": change.element,
        "label": change.label,
        "amount": change.amount,
        "condition": change.condition,
        "note": change.note,
        "sign": change.sign,
        "page": page,
    }


def _pending_change(waiting: dict[str, object]) -> Change:
    """Read a waiting change back as `core.inventory` understands it."""
    return Change(
        element=str(waiting["element"]),
        label=str(waiting["label"]),
        amount=waiting.get("amount"),
        condition=waiting.get("condition"),
        note=waiting.get("note"),
        sign=int(waiting.get("sign") or 1),
    )


def _pending_dict(index: int, waiting: dict[str, object]) -> dict[str, object]:
    """A waiting change as the page shows it, numbered so it can be acted on."""
    change = _pending_change(waiting)
    return {
        "index": index,
        "element": change.element,
        "label": change.label,
        "amount": change.amount,
        "condition": change.condition,
        "note": change.note,
        "sign": change.sign,
        "page": waiting.get("page"),
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
    now = datetime.now(timezone.utc)
    with _session() as session:
        inspection = session.scalars(
            select(PageInspection).where(
                PageInspection.book == book, PageInspection.path == path
            )
        ).first()
        if inspection is None:
            inspection = PageInspection.from_dict(
                {
                    "id": uuid.uuid4().hex,
                    "book": book,
                    "path": path,
                    "status": OPEN,
                    "comments": [],
                    "created_at": now,
                    "updated_at": now,
                }
            )
            session.add(inspection)
        state = inspection.to_dict()
        state["page_title"] = page_title or state.get("page_title")
        state["status"] = OPEN
        state["updated_at"] = now
        state["comments"] = [
            *(state.get("comments") or []),
            {"text": text, "created_at": now},
        ]
        inspection.update_from_dict(state)
    event(
        "Page flagged",
        inspection=inspection.id,
        book=book,
        path=path,
        comments=len(state["comments"]),
        text=text,
    )
    return _inspection_dict(inspection.to_dict())


def flagged_pages(book: str, status: str | None = None) -> list[InspectionDict]:
    """The pages flagged in one book, most recently commented first.

    Args:
        book: The book, as `"<series>/<book>"`.
        status: Keep only the files in that state; `None` lists them all.

    Returns:
        One file per flagged page.
    """
    query = select(PageInspection).where(PageInspection.book == book)
    if status is not None:
        query = query.where(PageInspection.status == status)
    query = query.order_by(PageInspection.updated_at.desc(), PageInspection.id.desc())
    with _session() as session:
        return [
            _inspection_dict(inspection.to_dict())
            for inspection in session.scalars(query)
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
    with _session() as session:
        inspection = session.get(PageInspection, inspection_id)
        return _inspection_dict(inspection.to_dict()) if inspection else None


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
    with _session() as session:
        inspection = session.get(PageInspection, inspection_id)
        if inspection is None:
            return None
        state = inspection.to_dict()
        state["status"] = status
        state["updated_at"] = datetime.now(timezone.utc)
        inspection.update_from_dict(state)
    event(
        "Flagged page status changed",
        inspection=inspection.id,
        path=inspection.path,
        status=status,
    )
    return _inspection_dict(inspection.to_dict())


def _inspection_dict(state: State) -> InspectionDict:
    """One flagged page as the routes read it.

    The blob's instants are already ISO 8601 text; only `updated_at`, a real
    column, still needs printing.
    """
    return {
        "id": str(state["id"]),
        "book": str(state["book"]),
        "path": str(state["path"]),
        "page_title": state.get("page_title"),
        "status": str(state["status"]),
        "comments": [
            {"text": comment["text"], "created_at": comment["created_at"]}
            for comment in state.get("comments") or []
        ],
        "created_at": str(state["created_at"]),
        "updated_at": state["updated_at"].isoformat(),
    }
