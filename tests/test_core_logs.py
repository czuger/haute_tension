"""The general log: what it writes, and what it must never write.

The lines are read through `caplog` and not off the disk: `logs/general.log` is
the same file for the whole run, and a test that read it would be reading every
other test's trace as well.
"""

import logging

import pytest

from haute_tension.core.logs import general_log
from haute_tension.core.logs.general_log import (
    DEFAULT_LEVEL,
    IDENTIFIER_LENGTH,
    cut,
    event,
    failure,
    hidden,
    is_a_secret,
    is_an_identifier,
    note,
    readable,
    sanitised,
    shortened,
    shown,
    spell_out,
    the_level,
    the_limit,
    without_the_secrets,
)
from haute_tension.core.logs.rotating_log import MAX_BYTES, open_the_log

LOGGER_NAME = "haute_tension.general"


@pytest.fixture(autouse=True)
def the_log_is_listened_to(caplog):
    """Capture the general log at DEBUG, whatever `LOG_LEVEL` the machine carries."""
    caplog.set_level(logging.DEBUG, logger=LOGGER_NAME)


class TestWritingALine:
    """The two ways in, and the shape of what they write."""

    def test_a_note_is_a_step(self, caplog):
        note("Combat armed", page="22")

        assert caplog.records[0].levelno == logging.DEBUG

    def test_an_event_is_something_that_happened(self, caplog):
        event("Hero rolled up", force=15)

        assert caplog.records[0].levelno == logging.INFO

    def test_a_failure_is_an_error(self, caplog):
        failure("It broke")

        assert caplog.records[0].levelno == logging.ERROR

    def test_a_failure_carries_its_traceback(self, caplog):
        try:
            raise ValueError("no reachable servers")
        except ValueError as trouble:
            failure("The database refused", trouble=trouble)

        assert "ValueError" in caplog.text
        assert "no reachable servers" in caplog.text

    def test_a_failure_without_an_exception_still_writes(self, caplog):
        failure("Nothing was raised", page="22")

        assert "Nothing was raised" in caplog.text

    def test_a_message_alone_is_written_alone(self):
        assert spell_out("Application built", {}) == "Application built"

    def test_variables_follow_an_em_dash(self):
        assert spell_out("Combat armed", {"page": "22", "enemies": 1}) == (
            "Combat armed — page='22', enemies=1"
        )

    def test_the_order_given_is_the_order_written(self):
        line = spell_out("x", {"b": 1, "a": 2})

        assert line.index("b=") < line.index("a=")


class TestSecrets:
    """What the log must never write."""

    @pytest.mark.parametrize(
        "name",
        ["secret", "token", "password", "authorization", "cookie", "session", "key"],
    )
    def test_every_secret_name_is_recognised(self, name):
        assert is_a_secret(name)

    def test_a_name_that_merely_contains_one_is_recognised(self):
        assert is_a_secret("FLASK_SECRET_KEY")
        assert is_a_secret("api_token_header")

    def test_an_ordinary_name_is_not(self):
        assert not is_a_secret("page")
        assert not is_a_secret("force")

    def test_a_secret_is_written_as_its_length(self):
        assert shown("secret_key", "abcdef") == "<hidden, 6 characters>"

    def test_an_absent_secret_says_so(self):
        assert shown("token", None) == "<absent>"
        assert hidden(None) == "<absent>"

    def test_a_secret_never_appears_in_the_line(self, caplog):
        note("Session opened", session_token="hunter2-and-then-some")

        assert "hunter2" not in caplog.text
        assert "<hidden, 21 characters>" in caplog.text

    def test_a_secret_buried_in_a_body_is_hidden_too(self):
        cleaned = sanitised({"page": "22", "auth": {"token": "abc123"}})

        assert cleaned["auth"]["token"] == "<hidden, 6 characters>"
        assert cleaned["page"] == "22"

    def test_a_secret_inside_a_list_is_hidden(self):
        cleaned = sanitised([{"password": "abc"}, {"page": "1"}])

        assert cleaned[0]["password"] == "<hidden, 3 characters>"

    def test_a_tuple_comes_back_a_list(self):
        assert sanitised(("a", "b")) == ["a", "b"]

    def test_a_plain_value_is_left_alone(self):
        assert sanitised(3) == 3

    def test_a_body_logged_whole_is_scrubbed(self, caplog):
        note("Answer", body={"cookie": "session=abc", "status": 200})

        assert "session=abc" not in caplog.text
        assert "200" in caplog.text


class TestPlayThroughIds:
    """A game id is a credential here, and is written in part only."""

    def test_a_game_field_is_an_identifier(self):
        assert is_an_identifier("game")
        assert is_an_identifier("game_id")
        assert not is_an_identifier("page")

    def test_only_its_head_is_written(self):
        written = shortened("4f2a91c0deadbeefcafe")

        assert written == repr("4f2a91c0" + "…")
        assert "deadbeef" not in written

    def test_a_short_one_is_written_whole(self):
        assert shortened("abc") == repr("abc")

    def test_an_absent_one_says_so(self):
        assert shortened(None) == "<absent>"

    def test_the_head_is_long_enough_to_tell_two_apart(self):
        assert IDENTIFIER_LENGTH >= 8

    def test_it_never_appears_whole_in_a_line(self, caplog):
        note("Combat armed", game="4f2a91c0deadbeefcafe", page="22")

        assert "deadbeefcafe" not in caplog.text
        assert "4f2a91c0" in caplog.text


class TestReadable:
    """How a value is written so it reads back."""

    def test_a_string_keeps_its_quotes(self):
        assert readable("22") == "'22'"

    def test_an_empty_string_is_visible(self):
        """Without the quotes it would read as nothing at all."""
        assert readable("") == "''"

    def test_a_trailing_space_is_visible(self):
        assert readable("22 ") == "'22 '"

    def test_a_number_is_written_plainly(self):
        assert readable(3) == "3"
        assert readable(3.5) == "3.5"

    def test_a_boolean_is_written_plainly(self):
        assert readable(True) == "True"

    def test_nothing_is_written_as_none(self):
        assert readable(None) == "None"

    def test_a_structure_is_written_as_json(self):
        assert readable({"page": "22"}) == '{"page": "22"}'

    def test_accents_survive(self):
        assert "numéro" in readable({"message": "le numéro"})

    def test_something_json_refuses_falls_back_to_str(self):
        from datetime import date

        assert "2026" in readable({"day": date(2026, 9, 9)})

    def test_something_nothing_can_write_falls_back_to_repr(self):
        """JSON refuses a key that is not a string or a number; repr does not."""

        class Awkward:
            def __repr__(self):
                return "<awkward>"

        assert readable({Awkward(): 1}) == "{<awkward>: 1}"

    def test_a_structure_that_refers_to_itself_falls_back_too(self):
        looping: list[object] = []
        looping.append(looping)

        assert readable(looping) == repr(looping)


class TestCut:
    """What is too long to be worth a whole file."""

    def test_a_short_value_is_left_alone(self):
        assert cut("abc") == "abc"

    def test_a_long_value_says_what_was_cut(self, monkeypatch):
        monkeypatch.setattr(general_log, "VALUE_LIMIT", 10)

        written = cut("x" * 50)

        assert written.startswith("x" * 10)
        assert "50 characters in all" in written

    def test_a_limit_of_zero_writes_everything(self, monkeypatch):
        monkeypatch.setattr(general_log, "VALUE_LIMIT", 0)

        assert cut("x" * 5000) == "x" * 5000


class TestConfiguration:
    """What the environment may say about the log."""

    def test_debug_is_the_default(self, monkeypatch):
        monkeypatch.delenv("LOG_LEVEL", raising=False)

        assert the_level() == DEFAULT_LEVEL == logging.DEBUG

    def test_a_named_level_is_read(self, monkeypatch):
        monkeypatch.setenv("LOG_LEVEL", "warning")

        assert the_level() == logging.WARNING

    def test_an_unreadable_level_falls_back_rather_than_stopping(self, monkeypatch):
        """A mistyped level must not cost a start-up."""
        monkeypatch.setenv("LOG_LEVEL", "chatty")

        assert the_level() == DEFAULT_LEVEL

    def test_the_limit_is_read_from_the_environment(self, monkeypatch):
        monkeypatch.setenv("LOG_VALUE_LIMIT", "50")

        assert the_limit() == 50

    def test_an_unreadable_limit_falls_back(self, monkeypatch):
        monkeypatch.setenv("LOG_VALUE_LIMIT", "lots")

        assert the_limit() == general_log.DEFAULT_VALUE_LIMIT


class TestUrls:
    """A URL as a log may carry it."""

    def test_a_url_without_a_query_is_untouched(self):
        assert without_the_secrets("/book/22") == "/book/22"

    def test_an_ordinary_parameter_is_kept(self):
        assert without_the_secrets("/game?rolled=1") == "/game?rolled=1"

    def test_a_secret_parameter_is_hidden(self):
        written = without_the_secrets("/x?page=22&token=abcdef")

        assert "abcdef" not in written
        assert "page=22" in written


class TestRotatingLog:
    """The file the log is written into."""

    def test_the_directory_is_made_if_it_is_missing(self, tmp_path):
        """`logs/` is not versioned, so a fresh clone has none."""
        path = tmp_path / "nowhere" / "general.log"

        handler = open_the_log(path)

        try:
            assert path.parent.is_dir()
        finally:
            handler.close()

    def test_the_line_carries_its_time(self, tmp_path):
        handler = open_the_log(tmp_path / "general.log")

        try:
            written = handler.format(
                logging.LogRecord("x", logging.INFO, "f", 1, "a line", None, None)
            )
        finally:
            handler.close()

        assert written.endswith("  a line")

    def test_it_rotates_by_size(self, tmp_path):
        path = tmp_path / "general.log"
        handler = open_the_log(path, max_bytes=200, files_kept=2)
        logger = logging.getLogger("haute_tension.test.rotation")
        logger.addHandler(handler)
        logger.setLevel(logging.DEBUG)

        try:
            for _ in range(40):
                logger.info("x" * 50)
        finally:
            logger.removeHandler(handler)
            handler.close()

        assert path.exists()
        assert (tmp_path / "general.log.1").exists()
        assert not (tmp_path / "general.log.3").exists()

    def test_the_default_size_is_generous_enough_for_a_run(self):
        assert MAX_BYTES >= 100 * 1024

    def test_a_directory_that_disappears_is_remade(self, tmp_path):
        """`logs/` going under a running server must not cost every later line.

        The handler reopens the file at every rotation, so making the directory
        once, before handing it over, is not enough.
        """
        import shutil

        directory = tmp_path / "logs"
        path = directory / "general.log"
        handler = open_the_log(path, max_bytes=200, files_kept=2)
        logger = logging.getLogger("haute_tension.test.vanishing")
        logger.addHandler(handler)
        logger.setLevel(logging.DEBUG)
        logger.info("before")
        shutil.rmtree(directory)

        try:
            for _ in range(20):
                logger.info("x" * 50)
        finally:
            logger.removeHandler(handler)
            handler.close()

        assert directory.is_dir()
        assert path.exists()
        assert "x" in path.read_text(encoding="utf-8")

    def test_nothing_is_written_to_stderr_when_it_disappears(
        self, tmp_path, capsys
    ):
        """A lost log line used to print a traceback per request."""
        import shutil

        directory = tmp_path / "logs"
        handler = open_the_log(directory / "general.log", max_bytes=200)
        logger = logging.getLogger("haute_tension.test.quiet")
        logger.addHandler(handler)
        logger.setLevel(logging.DEBUG)
        logger.info("before")
        shutil.rmtree(directory)

        try:
            for _ in range(20):
                logger.info("x" * 50)
        finally:
            logger.removeHandler(handler)
            handler.close()

        assert "FileNotFoundError" not in capsys.readouterr().err

    def test_the_directory_is_made_on_the_first_open_too(self, tmp_path):
        handler = open_the_log(tmp_path / "deeper" / "still" / "general.log")

        try:
            assert (tmp_path / "deeper" / "still").is_dir()
        finally:
            handler.close()


class TestSetUp:
    """Giving the log its file and its level."""

    def test_the_log_has_a_file(self):
        assert general_log.GENERAL_LOG.handlers

    def test_setting_up_again_adds_no_second_handler(self):
        """A module imported twice would otherwise write every line twice."""
        before = list(general_log.GENERAL_LOG.handlers)

        general_log.set_up_the_log()

        assert general_log.GENERAL_LOG.handlers == before

    def test_it_writes_into_the_logs_directory(self):
        assert general_log.GENERAL_LOG_PATH.parent.name == "logs"
        assert general_log.GENERAL_LOG_PATH.name == "general.log"
