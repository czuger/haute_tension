"""Tests for the reads and writes in `core.db`."""

from datetime import datetime, timedelta, timezone

import pytest

from haute_tension.core.db import (
    get_oldest_page,
    last_pages,
    load_story,
    record_page_view,
    save_story,
)

BOOK = "pretre_jean/forteresse_alamuth"
OTHER_BOOK = "pretre_jean/autre_livre"


def page(number, **overrides):
    """One well-formed page, with anything the test cares about overridden."""
    return {
        "page": number,
        "language": "fr",
        "text": [f"Page {number}."],
        "file_path": f"raw_data/{number}.html",
        "choices": [],
        **overrides,
    }


class TestSaveStory:
    """Writing a parsed book into the database."""

    def test_pages_are_written_and_counted(self, fake_db):
        written = save_story(BOOK, [page("1"), page("2")])

        assert written == 2
        assert {doc["page"] for doc in fake_db["story_pages"].docs} == {"1", "2"}

    def test_pages_are_stamped_with_their_book(self, fake_db):
        save_story(BOOK, [page("1")])

        assert fake_db["story_pages"].docs[0]["book"] == BOOK

    def test_choices_are_stored_with_their_element_changes(self, fake_db):
        save_story(
            BOOK,
            [
                page(
                    "1",
                    choices=[
                        {
                            "goto": "2",
                            "gains": [{"element": "sword", "amount": 1}],
                            "losses": [{"element": "gold", "amount": 3}],
                        }
                    ],
                )
            ],
        )

        stored = fake_db["story_pages"].docs[0]["choices"][0]
        assert stored["goto"] == "2"
        assert stored["gains"] == [{"element": "sword", "amount": 1}]
        assert stored["losses"] == [{"element": "gold", "amount": 3}]

    def test_a_fight_is_stored_as_it_came(self, fake_db):
        fight = {"fight_type": "single", "enemies": [{"name": "orc", "force": 6}]}
        save_story(BOOK, [page("1", fight=fight)])

        assert fake_db["story_pages"].docs[0]["fight"] == fight

    def test_undeclared_keys_are_dropped(self, fake_db):
        save_story(BOOK, [page("1", scraped_at="yesterday")])

        assert "scraped_at" not in fake_db["story_pages"].docs[0]

    def test_importing_again_replaces_the_book(self, fake_db):
        save_story(BOOK, [page("1"), page("2")])
        written = save_story(BOOK, [page("1")])

        assert written == 1
        assert [doc["page"] for doc in fake_db["story_pages"].docs] == ["1"]

    def test_importing_leaves_other_books_alone(self, fake_db):
        save_story(OTHER_BOOK, [page("9")])
        save_story(BOOK, [page("1")])

        assert {doc["book"] for doc in fake_db["story_pages"].docs} == {
            BOOK,
            OTHER_BOOK,
        }

    def test_an_empty_book_writes_nothing(self, fake_db):
        assert save_story(BOOK, []) == 0
        assert fake_db["story_pages"].docs == []

    def test_a_non_object_page_is_rejected(self, fake_db):
        with pytest.raises(ValueError, match="must be an object"):
            save_story(BOOK, ["1"])

    def test_a_page_without_string_number_is_rejected(self, fake_db):
        with pytest.raises(ValueError, match="string page number"):
            save_story(BOOK, [page(1)])

    def test_a_duplicate_page_number_is_rejected(self, fake_db):
        with pytest.raises(ValueError, match="Duplicate page number: 1"):
            save_story(BOOK, [page("1"), page("1")])

    def test_a_rejected_import_leaves_the_book_untouched(self, fake_db):
        save_story(BOOK, [page("1")])

        with pytest.raises(ValueError):
            save_story(BOOK, [page("2"), page("2")])

        assert [doc["page"] for doc in fake_db["story_pages"].docs] == ["1"]


class TestLoadStory:
    """Reading a book back out."""

    def test_pages_are_indexed_by_page_number(self, fake_db):
        save_story(BOOK, [page("1"), page("2")])

        story_data = load_story(BOOK)

        assert sorted(story_data) == ["1", "2"]
        assert story_data["2"]["text"] == ["Page 2."]

    def test_choices_come_back_as_plain_dicts(self, fake_db):
        save_story(
            BOOK,
            [
                page(
                    "1",
                    choices=[
                        {
                            "goto": "2",
                            "gains": [{"element": "sword", "amount": 1}],
                            "losses": [],
                        }
                    ],
                )
            ],
        )

        choice = load_story(BOOK)["1"]["choices"][0]
        assert choice == {
            "goto": "2",
            "gains": [{"element": "sword", "amount": 1}],
            "losses": [],
        }

    def test_every_declared_field_is_present(self, fake_db):
        save_story(BOOK, [{"page": "1"}])

        assert load_story(BOOK)["1"] == {
            "page": "1",
            "book": BOOK,
            "language": None,
            "text": [],
            "file_path": None,
            "choices": [],
            "fight": {},
        }

    def test_only_the_named_book_is_read(self, fake_db):
        save_story(BOOK, [page("1")])
        save_story(OTHER_BOOK, [page("9")])

        assert sorted(load_story(BOOK)) == ["1"]

    def test_a_book_never_imported_is_empty(self, fake_db):
        assert load_story(BOOK) == {}


class TestPageHistory:
    """Recording page views and reading the bounded history back."""

    def test_a_view_is_recorded(self, fake_db):
        record_page_view(BOOK, "1")

        stored = fake_db["page_views"].docs
        assert len(stored) == 1
        assert stored[0]["book"] == BOOK
        assert stored[0]["page"] == "1"

    def test_history_is_oldest_first(self, fake_db):
        for number in ("1", "2", "3"):
            record_page_view(BOOK, number)

        assert last_pages(BOOK) == ["1", "2", "3"]

    def test_history_keeps_only_the_most_recent_pages(self, fake_db):
        for number in range(1, 12):
            record_page_view(BOOK, str(number))

        assert last_pages(BOOK) == [str(number) for number in range(2, 12)]

    def test_history_limit_is_adjustable(self, fake_db):
        for number in ("1", "2", "3"):
            record_page_view(BOOK, number)

        assert last_pages(BOOK, limit=2) == ["2", "3"]

    def test_history_is_scoped_to_its_book(self, fake_db):
        record_page_view(BOOK, "1")
        record_page_view(OTHER_BOOK, "9")

        assert last_pages(BOOK) == ["1"]

    def test_the_same_page_can_be_read_twice(self, fake_db):
        record_page_view(BOOK, "1")
        record_page_view(BOOK, "1")

        assert last_pages(BOOK) == ["1", "1"]

    def test_views_sharing_a_timestamp_keep_insertion_order(self, fake_db):
        stamp = datetime(2026, 9, 9, 12, 0, tzinfo=timezone.utc)
        fake_db["page_views"].docs = [
            {"book": BOOK, "page": number, "viewed_at": stamp}
            for number in ("1", "2", "3")
        ]

        assert last_pages(BOOK) == ["1", "2", "3"]

    def test_oldest_page_is_the_first_retained_entry(self, fake_db):
        base = datetime(2026, 9, 9, 12, 0, tzinfo=timezone.utc)
        fake_db["page_views"].docs = [
            {"book": BOOK, "page": "4", "viewed_at": base},
            {"book": BOOK, "page": "7", "viewed_at": base + timedelta(minutes=1)},
        ]

        assert get_oldest_page(BOOK) == "4"

    def test_no_history_has_no_oldest_page(self, fake_db):
        assert get_oldest_page(BOOK) is None
