"""The hybrid pattern every table follows: a few columns, one JSON blob."""

import json
from datetime import date, datetime, timezone

import pytest
from sqlalchemy import select
from sqlalchemy.orm import Session

from haute_tension.core.models.game import Game
from haute_tension.core.models.hybrid_document import json_default
from haute_tension.core.models.page_inspection import PageInspection
from haute_tension.core.models.page_view import PageView
from haute_tension.core.models.utc_datetime import to_text

BOOK = "pretre_jean/forteresse_alamuth"
NOON = datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc)


class TestIndexedColumns:
    """Which fields are real columns, and which live in the blob."""

    def test_a_game_indexes_what_the_memorial_queries(self):
        assert Game.indexed_columns() == ("id", "book", "died_at")

    def test_a_visit_indexes_what_the_history_sorts_on(self):
        assert PageView.indexed_columns() == ("id", "book", "game", "viewed_at")

    def test_a_flagged_page_indexes_what_the_list_filters_on(self):
        assert PageInspection.indexed_columns() == (
            "id",
            "book",
            "path",
            "status",
            "updated_at",
        )

    def test_the_blob_itself_is_not_listed(self):
        for model in (Game, PageView, PageInspection):
            assert "data" not in model.indexed_columns()


class TestFromDict:
    """Sorting one flat dict into columns and blob."""

    def test_a_column_key_becomes_the_column(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "died_at": NOON, "force": 12})

        assert (game.id, game.book, game.died_at) == ("g1", BOOK, NOON)

    def test_every_other_key_goes_in_the_blob(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "force": 12, "items": []})

        assert json.loads(game.data) == {"force": 12, "items": []}

    def test_nothing_but_columns_is_an_empty_blob(self):
        assert json.loads(Game.from_dict({"id": "g1", "book": BOOK}).data) == {}

    def test_an_instant_in_the_blob_is_written_as_text(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "created_at": NOON})

        assert json.loads(game.data)["created_at"] == "2026-09-10T12:00:00+00:00"

    def test_the_blob_keeps_its_accents(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "died_of": "épée"})

        assert '"épée"' in game.data


class TestToDict:
    """Reading one flat dict back."""

    def test_columns_and_blob_are_one_dict(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "force": 12, "gold": 3})

        assert game.to_dict() == {
            "id": "g1",
            "book": BOOK,
            "died_at": None,
            "force": 12,
            "gold": 3,
        }

    def test_columns_come_first(self):
        game = Game.from_dict({"force": 12, "id": "g1", "book": BOOK})

        assert list(game.to_dict())[:3] == ["id", "book", "died_at"]

    def test_the_dict_is_a_copy(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "items": [{"count": 1}]})

        game.to_dict()["items"].append("nothing")

        assert game.to_dict()["items"] == [{"count": 1}]

    def test_an_empty_blob_reads_as_the_columns_alone(self):
        game = Game(id="g1", book=BOOK)

        assert game.to_dict() == {"id": "g1", "book": BOOK, "died_at": None}


class TestUpdateFromDict:
    """Writing a changed dict back onto a row."""

    def test_the_columns_are_set(self):
        game = Game.from_dict({"id": "g1", "book": BOOK})

        game.update_from_dict({"id": "g1", "book": BOOK, "died_at": NOON})

        assert game.died_at == NOON

    def test_the_blob_is_replaced_whole(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "force": 12, "gold": 3})

        game.update_from_dict({"id": "g1", "book": BOOK, "force": 13})

        assert game.to_dict() == {"id": "g1", "book": BOOK, "died_at": None, "force": 13}

    def test_a_column_left_out_keeps_its_value(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "died_at": NOON})

        game.update_from_dict({"force": 13})

        assert (game.id, game.book, game.died_at) == ("g1", BOOK, NOON)


class TestToJson:
    """The row as text."""

    def test_it_is_the_dict_as_json(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "force": 12})

        assert json.loads(game.to_json()) == game.to_dict()

    def test_a_column_instant_is_written_as_iso_8601(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "died_at": NOON})

        assert json.loads(game.to_json())["died_at"] == "2026-09-10T12:00:00+00:00"

    def test_accents_are_kept(self):
        game = Game.from_dict({"id": "g1", "book": BOOK, "died_of": "épée"})

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
        from datetime import timedelta

        paris = NOON.astimezone(timezone(timedelta(hours=2)))
        fake_db["page_views"].docs = [{"book": BOOK, "page": "1", "viewed_at": paris}]

        assert fake_db["page_views"].docs[0]["viewed_at"] == NOON

    def test_the_text_sorts_in_time_order(self):
        from datetime import timedelta

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
        fake_db["games"].docs = [{"id": "g1", "book": BOOK, "died_at": None}]

        assert fake_db["games"].docs[0]["died_at"] is None


class TestRowsPrint:
    """What a row prints as, for a message about a run."""

    def test_a_living_hero(self):
        game = Game.from_dict(
            {"id": "g1", "book": BOOK, "mode": "easy", "force": 14, "vie_max": 27, "vie_actuelle": 20}
        )

        assert str(game) == f"{BOOK} (easy) — Force 14, Vie 20/27"

    def test_a_dead_hero(self):
        game = Game.from_dict(
            {"id": "g1", "book": BOOK, "mode": "normal", "force": 9, "died_at": NOON}
        )

        assert str(game) == f"{BOOK} (normal) — Force 9, mort"
        assert game.is_dead

    def test_a_flagged_page_counts_its_comments(self):
        filed = PageInspection.from_dict(
            {"id": "i1", "book": BOOK, "path": "/book/2", "status": "open", "comments": [{}, {}]}
        )

        assert str(filed) == f"{BOOK} /book/2 (open, 2 comments)"


class TestThroughTheDatabase:
    """A row written and read back is the same flat dict."""

    def test_a_game_round_trips(self, fake_db):
        hero = {
            "id": "g1",
            "book": BOOK,
            "died_at": NOON,
            "mode": "normal",
            "force": 12,
            "items": [{"element": "sword", "label": "épée", "count": 1}],
            "combat": None,
        }
        fake_db["games"].docs = [hero]

        assert fake_db["games"].docs == [hero]

    def test_the_blob_is_one_text_column(self, fake_db):
        fake_db["games"].docs = [{"id": "g1", "book": BOOK, "force": 12}]

        with Session(fake_db._engine) as session:
            [game] = session.scalars(select(Game))
            assert json.loads(game.data) == {"force": 12}
