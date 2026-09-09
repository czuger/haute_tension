"""Tests for the reads and writes in `core.db`.

Only the reading history is stored, so only the reading history is tested here;
the book is read off disk and belongs to `test_story.py`.
"""

from datetime import datetime, timedelta, timezone

from haute_tension.core.db import get_oldest_page, last_pages, record_page_view

BOOK = "pretre_jean/forteresse_alamuth"
OTHER_BOOK = "pretre_jean/autre_livre"


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

    def test_a_reload_is_not_a_move(self, fake_db):
        """Asking again for the page one is on adds nothing."""
        assert record_page_view(BOOK, "1") is True
        assert record_page_view(BOOK, "1") is False

        assert last_pages(BOOK) == ["1"]

    def test_coming_back_to_a_page_is_a_move(self, fake_db):
        """Only consecutive repeats are dropped: a loop is worth seeing."""
        for number in ("1", "2", "1"):
            record_page_view(BOOK, number)

        assert last_pages(BOOK) == ["1", "2", "1"]

    def test_a_reload_of_another_book_is_still_a_move(self, fake_db):
        record_page_view(BOOK, "1")

        assert record_page_view(OTHER_BOOK, "1") is True

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


class TestPageView:
    """What a stored visit prints as, for a message about a run."""

    def test_it_names_what_was_read(self):
        from haute_tension.core.models import PageView

        view = PageView(book=BOOK, page="22", viewed_at="now")

        assert str(view) == f"{BOOK} p.22 @ now"
