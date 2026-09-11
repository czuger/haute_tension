"""The hybrid pattern every table follows: columns, child rows, one JSON blob."""

import json
from datetime import date, datetime, timedelta, timezone

import pytest
from sqlalchemy import text
from sqlalchemy.exc import IntegrityError
from sqlalchemy.orm import Session

from haute_tension.core.models.game import Game
from haute_tension.core.models.hybrid_document import json_default
from haute_tension.core.models.item import Item
from haute_tension.core.models.page_inspection import PageInspection
from haute_tension.core.models.page_view import PageView
from haute_tension.core.models.utc_datetime import to_text

BOOK = "pretre_jean/forteresse_alamuth"
NOON = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)
HERO = {"book": BOOK, "force": 12, "vie_max": 20, "vie_actuelle": 20, "gold": 5}
SWORD = {"element": "sword", "label": "épée", "count": 1}
KEY = {"element": "key", "label": "clé", "count": 2}
STATS = ("force", "vie_max", "vie_actuelle", "gold")


def without(values, key):
    """A copy of a dict with one key left out."""
    return {name: value for name, value in values.items() if name != key}


def bag_line(item):
    """What an item row says, without its id and its dates."""
    return {name: item[name] for name in ("element", "label", "count")}


class TestIndexedColumns:
    """Which fields are real columns, and which live in the blob."""

    def test_a_game_indexes_what_is_queried_or_constrained(self):
        assert Game.indexed_columns() == (
            "id",
            "book",
            *STATS,
            "died_at",
            "created_at",
            "updated_at",
        )

    def test_an_item_indexes_its_game(self):
        assert Item.indexed_columns() == ("id", "game_id", "created_at", "updated_at")

    def test_a_visit_indexes_what_the_history_sorts_on(self):
        assert PageView.indexed_columns() == ("id", "book", "game_id", "viewed_at")

    def test_a_flagged_page_indexes_what_the_list_filters_on(self):
        assert PageInspection.indexed_columns() == (
            "id",
            "book",
            "path",
            "status",
            "created_at",
            "updated_at",
        )

    def test_the_blob_itself_is_not_listed(self):
        for model in (Game, Item, PageView, PageInspection):
            assert "data" not in model.indexed_columns()


class TestChildTables:
    """Which rows travel inside another row's dict."""

    def test_a_game_carries_its_bag(self):
        assert Game.child_tables() == {"items": Item}

    def test_the_other_tables_carry_nothing(self):
        for model in (Item, PageView, PageInspection):
            assert model.child_tables() == {}


class TestFromDict:
    """Sorting one flat dict into columns, children and blob."""

    def test_a_column_key_becomes_the_column(self):
        game = Game.from_dict({**HERO, "id": 1, "died_at": NOON})

        assert (game.id, game.book, game.force, game.gold, game.died_at) == (
            1,
            BOOK,
            12,
            5,
            NOON,
        )

    def test_every_other_key_goes_in_the_blob(self):
        game = Game.from_dict({**HERO, "mode": "easy", "pending": []})

        assert game.data == {"mode": "easy", "pending": []}

    def test_a_relationship_key_becomes_child_rows(self):
        game = Game.from_dict({**HERO, "items": [SWORD, KEY]})

        assert [item.data for item in game.items] == [SWORD, KEY]
        assert "items" not in game.data

    def test_nothing_but_columns_is_an_empty_blob(self):
        assert Game.from_dict(HERO).data == {}

    def test_an_instant_in_the_blob_is_text_at_once(self):
        view = PageView.from_dict({"book": BOOK, "viewed_at": NOON, "seen": NOON})

        assert view.data == {"seen": "2026-09-10T12:00:00+00:00"}

    def test_the_blob_does_not_share_what_it_was_given(self):
        pending = [{"element": "key"}]
        game = Game.from_dict({**HERO, "pending": pending})

        pending.append("nothing")

        assert game.data["pending"] == [{"element": "key"}]


class TestToDict:
    """Reading one flat dict back."""

    def test_columns_children_and_blob_are_one_dict(self):
        game = Game.from_dict({**HERO, "id": 1, "mode": "easy", "items": [SWORD]})

        assert game.to_dict() == {
            "id": 1,
            **HERO,
            "died_at": None,
            "created_at": None,
            "updated_at": None,
            "items": [
                {"id": None, "game_id": None, "created_at": None, "updated_at": None, **SWORD}
            ],
            "mode": "easy",
        }

    def test_columns_come_first_then_children_then_the_blob(self):
        game = Game.from_dict({"mode": "easy", "items": [], **HERO, "id": 1})

        assert list(game.to_dict()) == [*Game.indexed_columns(), "items", "mode"]

    def test_the_dict_is_a_copy(self):
        game = Game.from_dict({**HERO, "pending": [{"count": 1}]})

        game.to_dict()["pending"].append("nothing")

        assert game.to_dict()["pending"] == [{"count": 1}]

    def test_an_unset_blob_reads_as_the_columns_alone(self):
        assert PageView(id=1, book=BOOK).to_dict() == {
            "id": 1,
            "book": BOOK,
            "game_id": None,
            "viewed_at": None,
        }


class TestUpdateFromDict:
    """Writing a changed dict back onto a row."""

    def test_the_columns_are_set(self):
        game = Game.from_dict(HERO)

        game.update_from_dict({**HERO, "vie_actuelle": 3, "died_at": NOON})

        assert (game.vie_actuelle, game.died_at) == (3, NOON)

    def test_the_blob_is_replaced_whole(self):
        game = Game.from_dict({**HERO, "mode": "easy", "combat": None})

        game.update_from_dict({**HERO, "mode": "normal"})

        assert game.data == {"mode": "normal"}

    def test_a_column_left_out_keeps_its_value(self):
        game = Game.from_dict({**HERO, "died_at": NOON})

        game.update_from_dict({"mode": "easy"})

        assert (game.book, game.force, game.died_at) == (BOOK, 12, NOON)

    def test_children_left_out_are_kept(self):
        game = Game.from_dict({**HERO, "items": [SWORD]})

        game.update_from_dict(HERO)

        assert [item.data for item in game.items] == [SWORD]


class TestTheBagAsRows:
    """A game's items, written and kept in step as rows of their own."""

    def rewrite_the_bag(self, fake_db, lines):
        """Load the one game, give it these bag lines, write it back."""
        with Session(fake_db._engine) as session:
            game = session.get(Game, 1)
            game.update_from_dict({**game.to_dict(), "items": lines})
            session.commit()
        return fake_db["items"].docs

    def test_one_row_per_line_of_the_bag(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD, KEY]}]

        rows = fake_db["items"].docs

        assert [(row["game_id"], bag_line(row)) for row in rows] == [(1, SWORD), (1, KEY)]

    def test_a_line_named_by_its_id_is_updated_in_place(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD]}]
        [sword] = fake_db["items"].docs

        [row] = self.rewrite_the_bag(fake_db, [{**sword, "count": 2}])

        assert (row["id"], row["count"]) == (sword["id"], 2)
        assert row["created_at"] == sword["created_at"]
        assert row["updated_at"] > sword["updated_at"]

    def test_a_line_without_an_id_is_a_new_row(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD]}]
        [sword] = fake_db["items"].docs

        rows = self.rewrite_the_bag(fake_db, [sword, KEY])

        assert [row["id"] for row in rows] == [sword["id"], sword["id"] + 1]
        assert bag_line(rows[1]) == KEY

    def test_a_line_left_out_is_deleted(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD, KEY]}]
        [_, key] = fake_db["items"].docs

        assert self.rewrite_the_bag(fake_db, [key]) == [key]

    def test_the_bag_reads_in_the_order_it_was_filled(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [KEY, SWORD]}]

        [game] = fake_db["games"].docs

        assert [bag_line(item) for item in game["items"]] == [KEY, SWORD]

    def test_a_deleted_game_takes_its_bag_with_it(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD]}]

        fake_db["games"].docs = []

        assert fake_db["items"].docs == []

    def test_a_line_needs_a_game(self, fake_db):
        with pytest.raises(IntegrityError, match="FOREIGN KEY"):
            fake_db["items"].docs = [{"game_id": 99, **SWORD}]


class TestTimestamps:
    """When a row was created, and when it last changed."""

    def test_a_new_row_was_last_changed_when_it_was_created(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1}]

        [game] = fake_db["games"].docs

        assert game["created_at"] == game["updated_at"]
        assert game["created_at"].tzinfo is not None

    def test_a_change_moves_updated_at_and_nothing_else(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1}]
        [before] = fake_db["games"].docs

        fake_db["games"].docs = [{**before, "vie_actuelle": 4}]
        [after] = fake_db["games"].docs

        assert after["created_at"] == before["created_at"]
        assert after["updated_at"] > before["updated_at"]

    def test_writing_the_same_values_moves_nothing(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD]}]
        before = fake_db["games"].docs

        fake_db["games"].docs = before

        assert fake_db["games"].docs == before

    def test_an_update_that_sets_updated_at_keeps_it(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1}]
        [game] = fake_db["games"].docs

        fake_db["games"].docs = [{**game, "gold": 1, "updated_at": NOON}]

        assert fake_db["games"].docs[0]["updated_at"] == NOON

    def test_a_line_changing_does_not_move_its_game(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "items": [SWORD]}]
        [before] = fake_db["games"].docs

        fake_db["games"].docs = [
            {**before, "items": [{**before["items"][0], "count": 3}]}
        ]
        [after] = fake_db["games"].docs

        assert after["updated_at"] == before["updated_at"]
        assert after["items"][0]["updated_at"] > before["items"][0]["updated_at"]

    def test_a_row_inserted_by_hand_is_stamped_by_the_database(self, fake_db):
        with fake_db._engine.begin() as connection:
            connection.execute(
                text(
                    "INSERT INTO games (book, force, vie_max, vie_actuelle, gold, data) "
                    "VALUES ('livre', 1, 1, 1, 0, '{}')"
                )
            )
            created, updated = connection.execute(
                text("SELECT created_at, updated_at FROM games")
            ).one()

        assert created == updated
        assert len(created) == len(to_text(NOON))
        assert fake_db["games"].docs[0]["created_at"].tzinfo is not None


class TestStatColumns:
    """Force, Vie and gold: small numbers that never go below zero."""

    @pytest.mark.parametrize("stat", STATS)
    def test_a_negative_value_is_refused(self, fake_db, stat):
        with pytest.raises(IntegrityError, match=f"ck_games_{stat}_not_negative"):
            fake_db["games"].docs = [{**HERO, "id": 1, stat: -1}]

    @pytest.mark.parametrize("stat", ["force", "vie_max", "vie_actuelle"])
    def test_a_missing_value_is_refused(self, fake_db, stat):
        with pytest.raises(IntegrityError, match=f"NOT NULL constraint failed: games.{stat}"):
            fake_db["games"].docs = [{**without(HERO, stat), "id": 1}]

    def test_an_unset_purse_is_empty(self, fake_db):
        fake_db["games"].docs = [{**without(HERO, "gold"), "id": 1}]

        assert fake_db["games"].docs[0]["gold"] == 0

    def test_zero_is_allowed(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "vie_actuelle": 0, "gold": 0}]

        assert fake_db["games"].docs[0]["vie_actuelle"] == 0

    @pytest.mark.parametrize("stat", STATS)
    def test_each_is_a_small_integer(self, stat):
        assert Game.__table__.c[stat].type.__visit_name__ == "small_integer"


class TestIds:
    """Integers the database counts up, and never hands out twice."""

    def test_they_count_up_from_one(self, fake_db):
        fake_db["games"].docs = [HERO, HERO]

        assert [game["id"] for game in fake_db["games"].docs] == [1, 2]

    def test_an_id_is_never_reused(self, fake_db):
        fake_db["games"].docs = [HERO, HERO]
        [first, _] = fake_db["games"].docs

        fake_db["games"].docs = [first]
        fake_db["games"].docs = [first, HERO]

        assert [game["id"] for game in fake_db["games"].docs] == [1, 3]


class TestToJson:
    """The row as text."""

    def test_it_is_the_dict_as_json(self):
        game = Game.from_dict({**HERO, "id": 1, "mode": "easy"})

        assert json.loads(game.to_json()) == game.to_dict()

    def test_a_column_instant_is_written_as_iso_8601(self):
        game = Game.from_dict({**HERO, "died_at": NOON})

        assert json.loads(game.to_json())["died_at"] == "2026-09-10T12:00:00+00:00"

    def test_the_bag_is_written_too(self):
        game = Game.from_dict({**HERO, "items": [SWORD]})

        assert bag_line(json.loads(game.to_json())["items"][0]) == SWORD

    def test_accents_are_kept(self):
        game = Game.from_dict({**HERO, "died_of": "épée"})

        assert "épée" in game.to_json()


class TestJsonDefault:
    """What the blob does with what JSON cannot write."""

    def test_a_datetime_is_its_iso_text(self):
        assert json_default(NOON) == "2026-09-10T12:00:00+00:00"

    def test_a_date_is_its_iso_text(self):
        assert json_default(date(2026, 9, 10)) == "2026-09-10"

    def test_anything_else_is_refused(self):
        with pytest.raises(TypeError, match="set cannot be written"):
            json_default({1, 2})


class TestUtcDateTime:
    """The timestamp column, through a real database."""

    def test_an_aware_instant_comes_back_aware_and_equal(self, fake_db):
        fake_db["page_views"].docs = [{"book": BOOK, "page": "1", "viewed_at": NOON}]

        [view] = fake_db["page_views"].docs

        assert view["viewed_at"] == NOON
        assert view["viewed_at"].tzinfo is not None

    def test_a_naive_instant_is_taken_to_be_utc(self, fake_db):
        fake_db["page_views"].docs = [
            {"book": BOOK, "page": "1", "viewed_at": NOON.replace(tzinfo=None)}
        ]

        assert fake_db["page_views"].docs[0]["viewed_at"] == NOON

    def test_another_timezone_is_stored_as_utc(self, fake_db):
        paris = NOON.astimezone(timezone(timedelta(hours=2)))
        fake_db["page_views"].docs = [{"book": BOOK, "page": "1", "viewed_at": paris}]

        assert fake_db["page_views"].docs[0]["viewed_at"] == NOON

    def test_the_text_sorts_in_time_order(self):
        instants = [
            NOON + timedelta(microseconds=500000),
            NOON + timedelta(seconds=1),
            NOON,
            NOON - timedelta(days=1),
        ]

        assert sorted(instants) == sorted(instants, key=to_text)

    def test_the_text_is_fixed_width(self):
        assert to_text(NOON) == "2026-09-10T12:00:00.000000+00:00"

    def test_nothing_stays_nothing(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "died_at": None}]

        assert fake_db["games"].docs[0]["died_at"] is None


class TestRowsPrint:
    """What a row prints as, for a message about a run."""

    def test_a_living_hero(self):
        game = Game.from_dict(
            {**HERO, "mode": "easy", "force": 14, "vie_max": 27, "vie_actuelle": 20}
        )

        assert str(game) == f"{BOOK} (easy) — Force 14, Vie 20/27"

    def test_a_dead_hero(self):
        game = Game.from_dict({**HERO, "mode": "normal", "force": 9, "died_at": NOON})

        assert str(game) == f"{BOOK} (normal) — Force 9, mort"
        assert game.is_dead

    def test_a_line_of_the_bag(self):
        ration = Item.from_dict({"element": "ration", "label": "ration", "count": 4})

        assert str(ration) == "ration ×4"

    def test_a_flagged_page_counts_its_comments(self):
        filed = PageInspection.from_dict(
            {"book": BOOK, "path": "/book/2", "status": "open", "comments": [{}, {}]}
        )

        assert str(filed) == f"{BOOK} /book/2 (open, 2 comments)"


class TestThroughTheDatabase:
    """A row written and read back is the same flat dict."""

    def test_a_game_round_trips(self, fake_db):
        hero = {**HERO, "id": 1, "died_at": NOON, "mode": "normal", "combat": None}
        fake_db["games"].docs = [{**hero, "items": [SWORD]}]

        [stored] = fake_db["games"].docs

        assert {key: stored[key] for key in hero} == hero
        assert [bag_line(item) for item in stored["items"]] == [SWORD]

    def test_the_blob_is_json_text_with_its_accents(self, fake_db):
        fake_db["games"].docs = [{**HERO, "id": 1, "died_of": "épée"}]

        with fake_db._engine.connect() as connection:
            stored = connection.execute(text("SELECT data FROM games")).scalar_one()

        assert json.loads(stored) == {"died_of": "épée"}
        assert "épée" in stored
