"""The routes a play-through needs: creating a hero, his sheet, and the fights.

The site header is a menu of four entries, and three of them land here:
**Nouveau** (`/game/new`) offers to roll a hero up, **Partie en cours**
(`/game/resume`) puts the reader back where his adventure stands, **Feuille**
(`/game`) is the sheet, and **Les tombés** (`/heroes`) the memorial.

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
from haute_tension.application.web_routes import OPENING_PAGE
from haute_tension.core.character import MODES, named_mode
from haute_tension.core.combat import DEFEAT, ONGOING, VICTORY
from haute_tension.core.db import (
    apply_pending,
    begin_combat,
    dismiss_pending,
    end_combat,
    find_game,
    follow_choice,
    play_assault,
    fallen_heroes,
    last_page_read,
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
        """Roll up Prêtre Jean and show the sheet the dice made.

        The mode comes off the form. An unknown one rolls a hero by the book
        rather than refusing to roll one: a posted value is whatever was posted.
        """
        game = start_game(
            book, mode=named_mode(request.form.get("mode")), rng=rng
        )
        session[SESSION_KEY] = game["id"]
        return redirect(url_for("game.show_character", rolled=1))

    @blueprint.get("/game/new")
    def offer_game() -> str:
        """Offer to roll a hero up, saying what it costs when one is alive.

        The menu's "Nouveau". Nothing is written here: the dice are only thrown
        by the POST, so the page is the confirmation. A living hero is shown
        as he stands, because creating another one leaves him behind — his
        game stays in the database, but the session forgets him.
        """
        return _offer_a_hero(_current_game())

    @blueprint.get("/game/resume")
    def resume_game() -> Response:
        """Put the reader back where his adventure stands.

        The menu's "Partie en cours": the fight he is in, the death page if he
        fell, the last paragraph he read, or the opening page of a hero who has
        read nothing. Without a hero, the offer to roll one up.
        """
        game = _current_game()
        if game is None:
            return redirect(url_for("game.offer_game"))
        if game["combat"] is not None:
            return redirect(
                url_for("game.show_combat", number=int(game["combat"]["page"]))
            )
        if game["is_dead"]:
            return redirect(url_for("game.show_death"))
        page = last_page_read(game["id"]) or str(OPENING_PAGE)
        return redirect(url_for("web.read_page", number=int(page)))

    @blueprint.get("/game")
    def show_character() -> str:
        """Show the hero's sheet, or offer to roll one up.

        Everything about him on one page — his Vie, his Force, his purse and his
        bag — because that is what a sheet is. It carries the way back to the
        story too: it is reached mid-adventure, not instead of one.
        """
        game = _current_game()
        if game is None:
            return _offer_a_hero(None)
        return render_template(
            "character.html",
            book_series=book_series,
            book_title=book_title,
            game=game,
            rolled=request.args.get("rolled") == "1",
            back_page=last_page_read(game["id"]),
        )

    @blueprint.get("/heroes")
    def show_fallen() -> str:
        """Remember the heroes who did not come back."""
        return render_template(
            "heroes.html",
            book_series=book_series,
            book_title=book_title,
            fallen=fallen_heroes(book),
            game=_current_game(),
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

    @blueprint.post("/book/<int:number>/choice/<int:index>")
    def take_choice(number: int, index: int) -> Response:
        """Follow one choice of a page, paying what it costs.

        A POST and not a link: following a choice spends rations and gold, and
        the redirect afterwards is what keeps a reload from spending them twice.
        """
        page = story_data.get(str(number))
        if page is None or not 0 <= index < len(page.get("choices") or []):
            abort(404)

        choice = page["choices"][index]
        game = _current_game()
        if game is not None:
            follow_choice(game["id"], str(number), choice)
        return redirect(url_for("web.read_page", number=int(choice["goto"])))

    @blueprint.post("/game/pending/<int:index>/apply")
    def apply_change(index: int) -> Response:
        """Apply a waiting change, because the reader says it applies."""
        game = _current_game()
        if game is not None:
            apply_pending(game["id"], index)
        return redirect(_back_to_the_reader())

    @blueprint.post("/game/pending/<int:index>/dismiss")
    def dismiss_change(index: int) -> Response:
        """Wave one waiting change away."""
        game = _current_game()
        if game is not None:
            dismiss_pending(game["id"], index)
        return redirect(_back_to_the_reader())

    @blueprint.post("/game/pending/dismiss")
    def dismiss_all_changes() -> Response:
        """Wave every waiting change away."""
        game = _current_game()
        if game is not None:
            dismiss_pending(game["id"])
        return redirect(_back_to_the_reader())

    @blueprint.get("/game/death")
    def show_death() -> str:
        """Tell the reader the adventure is over, and offer another hero."""
        return render_template(
            "death.html",
            book_series=book_series,
            book_title=book_title,
            game=_current_game(),
            modes=list(MODES.values()),
        )

    def _offer_a_hero(game: GameDict | None) -> str:
        """Render the page that rolls a hero up.

        Args:
            game: The hero the session has, so the page can say what starting
                over leaves behind, or `None` when it has none.
        """
        return render_template(
            "new_game.html",
            book_series=book_series,
            book_title=book_title,
            game=game,
            modes=list(MODES.values()),
            fallen=fallen_heroes(book),
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


def _back_to_the_reader() -> str:
    """The page the reader was on, or the sheet when the form said nothing.

    The waiting changes are shown on a story page and acted on from there, so
    the answer to a form posted from it is that same page.
    """
    number = request.form.get("page", "").strip()
    if number.isdigit():
        return url_for("web.read_page", number=int(number))
    return url_for("game.show_character")


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
