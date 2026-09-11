"""Flagging a page for inspection, and reviewing the pages flagged."""

import re
from datetime import datetime, timezone

import pytest

from haute_tension.core import db as core_db
from haute_tension.core.db import (
    find_inspection,
    flag_page,
    flagged_pages,
    set_inspection_status,
)

BOOK = "pretre_jean/forteresse_alamuth"
OTHER_BOOK = "pretre_jean/autre_livre"
AS_SCRIPT = {"Accept": "application/json"}


def text_of(response):
    """The response body as text."""
    return response.get_data(as_text=True)


def report(client, path="/book/1", comment="Une coquille.", **headers):
    """Send the dialog's form the way the browser would."""
    return client.post(
        "/flag-page",
        data={"path": path, "title": "Page 1 · Titre", "comment": comment},
        headers=headers or None,
    )


class TestFlaggingInTheDatabase:
    """One file per page, whatever the number of reports."""

    def test_the_first_report_opens_a_file(self, fake_db):
        filed = flag_page(BOOK, "/book/22", "Page 22", "Le choix mène au 23.")

        assert filed["path"] == "/book/22"
        assert filed["status"] == "open"
        assert [c["text"] for c in filed["comments"]] == ["Le choix mène au 23."]
        assert len(fake_db["page_inspections"].docs) == 1

    def test_a_second_report_adds_to_it(self, fake_db):
        flag_page(BOOK, "/book/22", "Page 22", "Première remarque.")
        filed = flag_page(BOOK, "/book/22", "Page 22", "Seconde remarque.")

        assert [c["text"] for c in filed["comments"]] == [
            "Première remarque.",
            "Seconde remarque.",
        ]
        assert len(fake_db["page_inspections"].docs) == 1

    def test_the_title_is_kept_and_not_blanked_by_a_later_report(self, fake_db):
        flag_page(BOOK, "/book/22", "Page 22", "Une.")
        filed = flag_page(BOOK, "/book/22", None, "Deux.")

        assert filed["page_title"] == "Page 22"

    def test_a_resolved_page_flagged_again_is_reopened(self, fake_db):
        filed = flag_page(BOOK, "/book/22", "Page 22", "Une.")
        set_inspection_status(filed["id"], "resolved")

        assert flag_page(BOOK, "/book/22", "Page 22", "Encore.")["status"] == "open"

    def test_each_comment_is_dated(self, fake_db):
        filed = flag_page(BOOK, "/book/22", "Page 22", "Une.")

        assert filed["comments"][0]["created_at"].startswith("20")
        assert filed["created_at"] == filed["updated_at"]

    def test_the_document_says_what_it_is(self, fake_db):
        from haute_tension.core.models import PageInspection

        flag_page(BOOK, "/book/22", "Page 22", "Une.")

        assert str(PageInspection.objects.first()) == f"{BOOK} /book/22 (open, 1 comments)"

    def test_pages_of_another_book_are_kept_apart(self, fake_db):
        flag_page(BOOK, "/book/22", "Page 22", "Ici.")
        flag_page(OTHER_BOOK, "/book/22", "Page 22", "Ailleurs.")

        assert len(fake_db["page_inspections"].docs) == 2
        assert [f["comments"][0]["text"] for f in flagged_pages(BOOK)] == ["Ici."]


class TestTheList:
    """What `flagged_pages` gives back."""

    def test_the_most_recently_commented_comes_first(self, fake_db):
        first = flag_page(BOOK, "/book/1", "Page 1", "Une.")
        second = flag_page(BOOK, "/book/2", "Page 2", "Deux.")
        touched = {
            "/book/1": datetime(2026, 9, 10, 12, 0, tzinfo=timezone.utc),
            "/book/2": datetime(2026, 9, 10, 11, 0, tzinfo=timezone.utc),
        }
        fake_db["page_inspections"].docs = [
            {**doc, "updated_at": touched[doc["path"]]}
            for doc in fake_db["page_inspections"].docs
        ]

        assert [f["id"] for f in flagged_pages(BOOK)] == [first["id"], second["id"]]

    def test_it_can_be_narrowed_to_one_status(self, fake_db):
        flag_page(BOOK, "/book/1", "Page 1", "Une.")
        done = flag_page(BOOK, "/book/2", "Page 2", "Deux.")
        set_inspection_status(done["id"], "resolved")

        assert [f["path"] for f in flagged_pages(BOOK, "open")] == ["/book/1"]
        assert [f["path"] for f in flagged_pages(BOOK, "resolved")] == ["/book/2"]
        assert len(flagged_pages(BOOK)) == 2

    def test_an_unknown_file_is_none(self, fake_db):
        assert find_inspection("nope") is None
        assert find_inspection(None) is None
        assert set_inspection_status("nope", "resolved") is None

    def test_an_unknown_status_is_refused(self, fake_db):
        filed = flag_page(BOOK, "/book/1", "Page 1", "Une.")

        with pytest.raises(ValueError):
            set_inspection_status(filed["id"], "lost")

    def test_two_files_on_one_page_cannot_coexist(self, fake_db):
        """The unique index is the safety net under the find-or-create."""
        from mongoengine.errors import NotUniqueError

        from haute_tension.core.models import PageInspection

        now = datetime.now(timezone.utc)
        for number in ("a", "b"):
            twin = PageInspection(
                id=number, book=BOOK, path="/book/1", created_at=now, updated_at=now
            )
            if number == "a":
                twin.save(force_insert=True)
                continue
            with pytest.raises(NotUniqueError):
                twin.save(force_insert=True)


class TestTheDialog:
    """The menu entry and the form behind it, on every page."""

    @pytest.mark.parametrize("path", ["/", "/book/1", "/game", "/heroes", "/flagged-pages"])
    def test_every_page_carries_the_entry_and_the_form(self, hero, path):
        body = text_of(hero.get(path))

        assert 'href="#signaler">Signaler<' in body
        assert 'action="/flag-page"' in body
        assert f'name="path" value="{path}"' in body

    def test_the_form_carries_the_page_title(self, client):
        body = text_of(client.get("/book/1"))

        assert re.search(r'name="title" value="Page 1 · [^"]+"', body)

    def test_the_landing_page_needs_no_database_for_it(self, books_path, monkeypatch):
        from haute_tension.application.factory import create_app

        def unreachable():
            raise core_db.DatabaseUnavailable("none")

        monkeypatch.setattr(core_db, "connect_db", unreachable)
        app = create_app(BOOK, books_path)
        app.secret_key = "test-secret"

        assert "Signaler" in text_of(app.test_client().get("/"))

    def test_the_dialog_links_to_the_review_list(self, client):
        assert 'href="/flagged-pages"' in text_of(client.get("/"))


class TestReportingAsAForm:
    """Without a script: a flash message and back to the page."""

    def test_the_report_is_filed_and_the_reader_sent_back(self, client, fake_db):
        response = report(client)

        assert response.status_code == 302
        assert response.headers["Location"] == "/book/1"
        [filed] = fake_db["page_inspections"].docs
        assert filed["path"] == "/book/1"
        assert filed["page_title"] == "Page 1 · Titre"
        assert filed["comments"][0]["text"] == "Une coquille."

    def test_the_page_then_says_thank_you(self, client):
        response = report(client)
        body = text_of(client.get(response.headers["Location"]))

        assert "la page est signalée" in body
        assert 'class="toast success"' in body

    def test_the_thanks_is_said_once(self, client):
        report(client)
        client.get("/book/1")

        assert "la page est signalée" not in text_of(client.get("/book/1"))

    def test_a_blank_remark_is_refused(self, client, fake_db):
        response = report(client, comment="   ")

        assert response.status_code == 302
        assert fake_db["page_inspections"].docs == []
        assert "Dites en quelques mots" in text_of(client.get("/book/1"))

    def test_a_second_report_joins_the_first(self, client, fake_db):
        report(client, comment="Une.")
        report(client, comment="Deux.")

        [filed] = fake_db["page_inspections"].docs
        assert [c["text"] for c in filed["comments"]] == ["Une.", "Deux."]

    @pytest.mark.parametrize("path", ["https://elsewhere.example/x", "//evil", "book/1", ""])
    def test_only_a_local_path_is_filed(self, client, fake_db, path):
        response = report(client, path=path)

        assert response.headers["Location"] == "/"
        assert fake_db["page_inspections"].docs[0]["path"] == "/"


class TestReportingAsAScript:
    """With the page's script: JSON, and the reader stays where he is."""

    def test_the_answer_is_json(self, client, fake_db):
        response = report(client, **AS_SCRIPT)

        assert response.status_code == 200
        assert response.get_json() == {
            "success": True,
            "message": "Merci, la page est signalée pour inspection.",
            "comments": 1,
        }
        assert len(fake_db["page_inspections"].docs) == 1

    def test_the_comment_count_grows(self, client):
        report(client, **AS_SCRIPT)

        assert report(client, **AS_SCRIPT).get_json()["comments"] == 2

    def test_a_blank_remark_is_a_400(self, client, fake_db):
        response = report(client, comment="", **AS_SCRIPT)

        assert response.status_code == 400
        assert response.get_json()["success"] is False
        assert fake_db["page_inspections"].docs == []

    def test_no_flash_is_left_behind(self, client):
        report(client, **AS_SCRIPT)

        assert 'class="toast success"' not in text_of(client.get("/book/1"))

    def test_an_unreachable_database_answers_json(self, client, monkeypatch):
        def unreachable():
            raise core_db.DatabaseUnavailable("none")

        monkeypatch.setattr(core_db, "connect_db", unreachable)
        response = report(client, **AS_SCRIPT)

        assert response.status_code == 503
        assert response.get_json()["success"] is False

    def test_an_unreachable_database_answers_a_page_to_a_form(self, client, monkeypatch):
        def unreachable():
            raise core_db.DatabaseUnavailable("none")

        monkeypatch.setattr(core_db, "connect_db", unreachable)
        response = report(client)

        assert response.status_code == 503
        assert response.content_type.startswith("text/html")


class TestTheReviewList:
    """`/flagged-pages`."""

    def test_it_says_when_nothing_is_flagged(self, client):
        assert "Aucune page n'a été signalée" in text_of(client.get("/flagged-pages"))

    def test_a_flagged_page_is_listed(self, client):
        report(client, comment="Le choix mène au mauvais paragraphe.")
        body = text_of(client.get("/flagged-pages"))

        assert "Page 1 · Titre" in body
        assert "/book/1" in body
        assert "1 remarque" in body
        assert "Le choix mène au mauvais paragraphe." in body
        assert "À traiter" in body

    def test_it_links_to_the_file(self, client, fake_db):
        report(client)
        [filed] = fake_db["page_inspections"].docs

        assert f'href="/flagged-pages/{filed["id"]}"' in text_of(client.get("/flagged-pages"))

    def test_the_last_remark_is_the_one_shown(self, client):
        report(client, comment="Première.")
        report(client, comment="Dernière.")
        body = text_of(client.get("/flagged-pages"))

        assert "Dernière." in body
        assert "Première." not in body
        assert "2 remarques" in body

    def test_it_filters_by_status(self, client, fake_db):
        report(client, path="/book/1")
        report(client, path="/book/2")
        [_, second] = fake_db["page_inspections"].docs
        client.post(f"/flagged-pages/{second['id']}/resolve")

        assert "/book/2" not in text_of(client.get("/flagged-pages?status=open"))
        assert "/book/1" not in text_of(client.get("/flagged-pages?status=resolved"))

    def test_an_empty_filter_says_which(self, client):
        assert "Aucune page à traiter" in text_of(client.get("/flagged-pages?status=open"))
        assert "Aucune page réglée" in text_of(client.get("/flagged-pages?status=resolved"))

    def test_an_unknown_status_shows_everything(self, client):
        report(client)

        assert "/book/1" in text_of(client.get("/flagged-pages?status=lost"))


class TestTheFile:
    """`/flagged-pages/<id>`: the whole thread, and the verdict."""

    def test_every_remark_is_shown(self, client, fake_db):
        report(client, comment="Première.")
        report(client, comment="Dernière.")
        [filed] = fake_db["page_inspections"].docs
        body = text_of(client.get(f"/flagged-pages/{filed['id']}"))

        assert "Première." in body
        assert "Dernière." in body
        assert "2 remarques" in body
        assert 'href="/book/1"' in body

    def test_an_unknown_file_is_not_found(self, client):
        assert client.get("/flagged-pages/nope").status_code == 404

    def test_it_can_be_resolved(self, client, fake_db):
        report(client)
        [filed] = fake_db["page_inspections"].docs

        response = client.post(f"/flagged-pages/{filed['id']}/resolve")

        assert response.headers["Location"].endswith(f"/flagged-pages/{filed['id']}")
        assert fake_db["page_inspections"].docs[0]["status"] == "resolved"
        assert "Rouvrir" in text_of(client.get(response.headers["Location"]))

    def test_it_can_be_reopened(self, client, fake_db):
        report(client)
        [filed] = fake_db["page_inspections"].docs
        client.post(f"/flagged-pages/{filed['id']}/resolve")

        client.post(f"/flagged-pages/{filed['id']}/reopen")

        assert fake_db["page_inspections"].docs[0]["status"] == "open"
        assert "Marquer comme réglée" in text_of(client.get(f"/flagged-pages/{filed['id']}"))

    def test_resolving_an_unknown_file_is_not_found(self, client):
        assert client.post("/flagged-pages/nope/resolve").status_code == 404
        assert client.post("/flagged-pages/nope/reopen").status_code == 404
