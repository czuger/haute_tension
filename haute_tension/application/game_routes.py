"""The routes a play-through needs: the character sheet, and the fights.

The session cookie carries nothing but a game id; everything else is loaded from
the database on each request, so two tabs of the same browser are the same hero
and nothing about him lives in the cookie.

Unlike the reader, these routes **need** the database — a fight cannot be rolled
without somewhere to keep its wounds — so they report a failure rather than
degrading. `/book/<n>` stays readable without one; only combat does not.
"""

import random

from flask import (
    Blueprint,
    abort,
    redirect,
    render_template,
    request,
    session,
    url_for,
)
from werkzeug.wrappers.response import Response

from haute_tension.application.models.game import SESSION_KEY, GameDict
from haute_tension.application.models.story_page import StoryData
from haute_tension.core.combat import DEFEAT, ONGOING, VICTORY
from haute_tension.core.db import (
    begin_combat,
    end_combat,
    find_game,
    play_assault,
    start_game,
)

DEATH = "death"


def create_game_blueprint(
    story_data: StoryData,
    book: str,
    book_series: str,
    book_title: str,
    rng: random.Random | None = None,
) -> Blueprint:
    """Create the routes for rolling up a hero and fighting with him.

    Args:
        story_data: Story pages indexed by page number.
        book: The book being played, as `"<series>/<book>"`.
        book_series: Display name of the book series.
        book_title: Display title of the current book.
        rng: Source of chance, seeded by the tests.

    Returns:
        A configured Flask blueprint.
    """
    blueprint = Blueprint("game", __name__)

    @blueprint.post("/game/new")
    def create_game() -> Response:
        """Roll up Prêtre Jean and show the sheet the dice made."""
        game = start_game(book, rng)
        session[SESSION_KEY] = game["id"]
        return redirect(url_for("game.show_character", rolled=1))

    @blueprint.get("/game")
    def show_character() -> str | Response:
        """Show the hero's sheet, or offer to roll one up."""
        game = _current_game()
        if game is None:
            return render_template(
                "no_game.html", book_series=book_series, book_title=book_title
            )
        return render_template(
            "character.html",
            book_series=book_series,
            book_title=book_title,
            game=game,
            rolled=request.args.get("rolled") == "1",
        )

    @blueprint.get("/combat/<int:number>")
    def show_combat(number: int) -> str | Response:
        """Show the fight a page holds, arming it on first arrival."""
        page = story_data.get(str(number))
        if page is None or not (page.get("fight") or {}).get("enemies"):
            abort(404)

        game = _current_game()
        if game is None:
            return redirect(url_for("game.show_character"))

        game = begin_combat(game["id"], str(number), page["fight"])
        if game is None or game["combat"] is None:
            abort(404)
        return _render_combat(game)

    @blueprint.post("/combat/<int:number>/assault")
    def fight_assault(number: int) -> Response:
        """Play one assault, then come back to the fight.

        The fight is armed here too, not only on the way in: `begin_combat`
        leaves an open fight on the same page untouched, so this costs nothing
        in the browser and keeps the route from depending on a GET having
        happened first.
        """
        game = _current_game()
        page = story_data.get(str(number))
        if game is not None and page is not None and page.get("fight"):
            if begin_combat(game["id"], str(number), page["fight"]) is not None:
                play_assault(game["id"], rng)
        return redirect(url_for("game.show_combat", number=number))

    @blueprint.post("/combat/<int:number>/resolve")
    def leave_combat(number: int) -> Response:
        """Close a decided fight and follow the book to what comes next."""
        game = _current_game()
        if game is None or game["combat"] is None:
            return redirect(url_for("web.read_page", number=number))

        destination = _destination(game["combat"])
        if game["combat"]["status"] != ONGOING:
            end_combat(game["id"])
        if destination == DEATH:
            return redirect(url_for("game.show_death"))
        if destination is None:
            return redirect(url_for("web.read_page", number=number))
        return redirect(url_for("web.read_page", number=int(destination)))

    @blueprint.get("/game/death")
    def show_death() -> str:
        """Tell the reader the adventure is over, and offer another hero."""
        return render_template(
            "death.html",
            book_series=book_series,
            book_title=book_title,
            game=_current_game(),
        )

    def _render_combat(game: GameDict) -> str:
        """Render the fight the game is in."""
        return render_template(
            "combat.html",
            book_series=book_series,
            book_title=book_title,
            game=game,
            combat=game["combat"],
            page_number=game["combat"]["page"],
        )

    return blueprint


def _current_game() -> GameDict | None:
    """The game the session is on, or `None` when it has none or it is stale."""
    game = find_game(session.get(SESSION_KEY))
    if game is None:
        session.pop(SESSION_KEY, None)
    return game


def _destination(combat: dict) -> str | None:
    """Where the book sends the hero once this fight is decided.

    Args:
        combat: The fight, decided or not.

    Returns:
        A page number, `"death"`, or `None` when the book records no branch —
        which the parser left empty often enough to be worth handling.
    """
    if combat["status"] == VICTORY:
        return combat["on_victory"]
    if combat["status"] == DEFEAT:
        return combat["on_defeat"] or DEATH
    return None
