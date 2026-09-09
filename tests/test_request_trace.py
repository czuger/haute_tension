"""Every request written into the general log, and the three bodies it will not copy."""

import logging

import pytest

LOGGER_NAME = "haute_tension.general"


@pytest.fixture(autouse=True)
def the_log_is_listened_to(caplog):
    """Capture the general log at DEBUG for every test here."""
    caplog.set_level(logging.DEBUG, logger=LOGGER_NAME)


def lines(caplog, opening):
    """Every logged line starting with `opening`."""
    return [
        record.getMessage()
        for record in caplog.records
        if record.getMessage().startswith(opening)
    ]


class TestEveryRequestIsTraced:
    """Two lines per request, and nothing outside the trace."""

    def test_a_page_leaves_a_request_and_an_answer(self, client, caplog):
        client.get("/book/1")

        assert lines(caplog, "Request")
        assert lines(caplog, "Answer")

    def test_the_request_names_what_was_asked_for(self, client, caplog):
        client.get("/book/1")

        assert "path='/book/1'" in lines(caplog, "Request")[0]
        assert "method='GET'" in lines(caplog, "Request")[0]
        assert "endpoint='web.read_page'" in lines(caplog, "Request")[0]

    def test_the_answer_names_its_status_and_time(self, client, caplog):
        client.get("/book/1")
        answer = lines(caplog, "Answer")[0]

        assert "status=200" in answer
        assert " ms'" in answer

    def test_an_address_matching_no_route_is_traced_too(self, client, caplog):
        client.get("/nowhere-at-all")

        assert "path='/nowhere-at-all'" in lines(caplog, "Request")[0]
        assert "status=404" in lines(caplog, "Answer")[0]

    def test_the_query_string_is_written(self, client, caplog):
        client.get("/book/1?from=here")

        assert 'query={"from": "here"}' in lines(caplog, "Request")[0]

    def test_the_route_arguments_are_written(self, client, caplog):
        client.get("/book/1")

        assert 'route={"number": 1}' in lines(caplog, "Request")[0]


class TestBodies:
    """What is copied, what is described, and what is left alone."""

    def test_a_json_answer_is_written_in_full(self, client, caplog):
        client.get("/data/1")

        assert "Le début de l'aventure." in lines(caplog, "Answer")[0]

    def test_a_page_is_described_rather_than_copied(self, client, caplog):
        client.get("/book/1")

        assert "bytes of text/html" in lines(caplog, "Answer")[0]
        assert "<!doctype html>" not in lines(caplog, "Answer")[0]

    def test_a_refusal_is_read_whatever_its_shape(self, client, caplog):
        """From 400 up the body carries the sentence explaining it."""
        client.get("/book/999")

        assert "Not Found" in lines(caplog, "Answer")[0]

    def test_a_posted_form_is_written(self, fighting_client, caplog):
        fighting_client.post("/game/new", data={"chosen": "yes"})

        assert 'payload={"chosen": "yes"}' in lines(caplog, "Request")[0]

    def test_a_request_carrying_nothing_says_so(self, client, caplog):
        client.get("/book/1")

        assert "payload=None" in lines(caplog, "Request")[0]

    def test_a_body_that_is_not_a_form_is_described(self, client, caplog):
        client.post(
            "/book/1", data=b"\x00\x01\x02", content_type="application/octet-stream"
        )

        assert "3 bytes, application/octet-stream" in lines(caplog, "Request")[0]

    def test_a_json_request_is_parsed(self, client, caplog):
        client.post("/book/1", json={"page": "22"})

        assert 'payload={"page": "22"}' in lines(caplog, "Request")[0]


class TestRedirects:
    """Where a redirect sends is the whole of what it says."""

    def test_a_redirect_names_its_destination(self, hero, caplog):
        caplog.clear()
        hero.post("/combat/1/assault")

        assert "destination=" in lines(caplog, "Answer")[0]
        assert "/combat/1" in lines(caplog, "Answer")[0]

    def test_an_ordinary_answer_carries_no_destination(self, client, caplog):
        client.get("/book/1")

        assert "destination=" not in lines(caplog, "Answer")[0]


class TestTheGameId:
    """The credential that travels in the session cookie."""

    def test_a_reader_without_a_game_shows_none(self, client, caplog):
        client.get("/book/1")

        assert "game=<absent>" in lines(caplog, "Request")[0]

    def test_a_reader_with_one_is_followed_by_its_head(self, hero, caplog, fake_db):
        game_id = fake_db["games"].docs[0]["id"]
        caplog.clear()
        hero.get("/book/1")

        assert game_id[:8] in lines(caplog, "Request")[0]
        assert game_id not in lines(caplog, "Request")[0]


class TestFailures:
    """What a request died of."""

    def test_a_raising_route_is_logged_with_its_traceback(
        self, client, caplog, monkeypatch
    ):
        from haute_tension.application import web_routes

        def boom(*args, **kwargs):
            raise RuntimeError("the templates are gone")

        monkeypatch.setattr(web_routes, "render_template", boom)

        with pytest.raises(RuntimeError):
            client.get("/book/1")

        assert lines(caplog, "Request failed")
        assert "RuntimeError" in caplog.text
        assert "the templates are gone" in caplog.text

    def test_a_handled_refusal_is_not_a_failure(self, client, caplog):
        """`abort(404)` left as an answer with a status of its own."""
        client.get("/book/999")

        assert not lines(caplog, "Request failed")


class TestBodiesNotRead:
    """The two answers the trace describes instead of reading."""

    def test_a_streamed_answer_is_left_untouched(self):
        """Reading it to log it would consume it."""
        from flask import Flask

        from haute_tension.application.logs.request_trace import what_goes_out

        app = Flask(__name__)
        with app.test_request_context("/"):
            response = app.response_class(iter([b"a", b"b"]), mimetype="text/plain")

            assert what_goes_out(response) == "<streamed, left untouched>"

    def test_a_file_handed_over_untouched_is_not_read(self, tmp_path):
        from flask import Flask, send_file

        from haute_tension.application.logs.request_trace import what_goes_out

        path = tmp_path / "a.txt"
        path.write_text("hello", encoding="utf-8")
        app = Flask(__name__)
        with app.test_request_context("/"):
            response = send_file(path)

            try:
                assert what_goes_out(response) == "<streamed, left untouched>"
            finally:
                response.close()


class TestTiming:
    """How long a request has been in the server's hands."""

    def test_an_unmarked_arrival_reads_as_unknown(self):
        """A `before_request` that raised would leave the mark unset."""
        from flask import Flask

        from haute_tension.application.logs.request_trace import how_long_it_took

        app = Flask(__name__)
        with app.test_request_context("/"):
            assert how_long_it_took() == "unknown"

    def test_a_marked_arrival_reads_in_milliseconds(self, client, caplog):
        client.get("/book/1")

        assert " ms'" in lines(caplog, "Answer")[0]
