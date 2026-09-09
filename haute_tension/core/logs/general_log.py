"""The general log: the server's own trace, in `logs/general.log`.

What is read when something has gone wrong and the page itself has nothing to say
about it — a fight that opened on the wrong page, a hero the session lost, a
database that stopped answering. The requests that come in and the answers that
go out in full (`request_trace.py`), the games rolled up, the fights begun and
decided.

**DEBUG, and on.** The level is DEBUG unless `LOG_LEVEL` says otherwise in the
environment: a trace one must first go and turn on is a trace one does not have
on the day it is needed.

Two ways in, and one rule for both: **name every variable and write out its
content.**

    note("Combat armed", page="22", enemies=1)          # a step, DEBUG
    event("Hero rolled up", force=15, vie=24)           # something happened, INFO

The rule that is not negotiable: **a secret is never written out.** A name that
says token, secret, password, authorization, cookie, key or session — at the top
level or deep inside a body being logged — is replaced by its length.

`game_id` is the exception this application adds. It is not a secret by name, but
it is the only credential here: whoever holds one can pick up that play-through.
It is written to its first few characters, which is enough to follow one reader
through a run and not enough to replay them.
"""

import json
import logging
import os
from collections.abc import Mapping
from urllib.parse import parse_qsl, urlsplit, urlunsplit

from haute_tension.core.config import ROOT
from haute_tension.core.logs.rotating_log import open_the_log

GENERAL_LOG_PATH = ROOT / "logs" / "general.log"

# The level, unless `LOG_LEVEL` says otherwise. Anything unreadable there falls
# back here rather than stopping the application: a mistyped level must not cost
# a start-up.
DEFAULT_LEVEL = logging.DEBUG

# Beyond that many characters a value is cut — and says it was, with its full
# length. A story page is a couple of kilobytes and `/data/<n>` hands one over
# whole, so a trace that wrote every one of them would hold an afternoon's
# reading and nothing else. `LOG_VALUE_LIMIT` raises it; 0 there writes every
# answer whole, however long.
DEFAULT_VALUE_LIMIT = 2000

# A field whose name carries one of these is never written out. Broad on purpose:
# a new secret passing through a name nobody thought of is hidden by default
# rather than published by default.
SECRET_NAMES = (
    "secret",
    "token",
    "password",
    "authorization",
    "cookie",
    "session",
    "key",
)

# How much of a play-through id is written. Enough to tell two readers apart in a
# log, short enough to be useless to anyone who reads the file.
IDENTIFIER_NAMES = ("game_id", "game")
IDENTIFIER_LENGTH = 8

GENERAL_LOG = logging.getLogger("haute_tension.general")


def the_level() -> int:
    """Read the level from `LOG_LEVEL`, DEBUG failing that.

    Returns:
        A `logging` level; DEBUG for an absent or unreadable one.
    """
    asked = os.environ.get("LOG_LEVEL", "").strip().upper()
    return logging.getLevelNamesMapping().get(asked, DEFAULT_LEVEL)


def set_up_the_log() -> None:
    """Give the log its file and its level, once.

    Called at import, and idempotent because it is: a module imported twice
    under two names — which a test run does — would otherwise write every line
    twice.
    """
    if GENERAL_LOG.handlers:
        return
    GENERAL_LOG.addHandler(open_the_log(GENERAL_LOG_PATH))
    GENERAL_LOG.setLevel(the_level())


set_up_the_log()


def the_limit() -> int:
    """Read the cut from `LOG_VALUE_LIMIT`, `DEFAULT_VALUE_LIMIT` failing that.

    Returns:
        The number of characters a value is written out to; 0 for no cut at all.
    """
    asked = os.environ.get("LOG_VALUE_LIMIT", "").strip()
    return int(asked) if asked.isdigit() else DEFAULT_VALUE_LIMIT


VALUE_LIMIT = the_limit()


def note(message: str, **variables: object) -> None:
    """Write one step of what the server is doing, at DEBUG.

    Args:
        message: What is happening, in English and in plain words.
        **variables: Everything worth reading afterwards, each named; contents
            are written out, secrets excepted.
    """
    GENERAL_LOG.debug("%s", spell_out(message, variables))


def event(message: str, **variables: object) -> None:
    """Write something that happened rather than a step towards it, at INFO.

    A hero rolled up, a fight decided, a game abandoned: what is still worth
    having when the level has been raised to leave the steps out.

    Args:
        message: What happened.
        **variables: Its variables, named; contents are written out, secrets
            excepted.
    """
    GENERAL_LOG.info("%s", spell_out(message, variables))


def failure(
    message: str,
    trouble: BaseException | None = None,
    **variables: object,
) -> None:
    """Write something that went wrong, at ERROR, with the traceback if any.

    Args:
        message: What failed.
        trouble: The exception, whose traceback is then written under the line;
            `None` for a failure that raised nothing. A variable of one's own may
            not be called `trouble`.
        **variables: Its variables, named; contents are written out, secrets
            excepted.
    """
    GENERAL_LOG.error("%s", spell_out(message, variables), exc_info=trouble)


def spell_out(message: str, variables: Mapping[str, object]) -> str:
    """Put a message and its variables into the one line the log carries.

        Combat armed — page='22', enemies=1, game='4f2a91c0…'

    Args:
        message: The message.
        variables: Its variables, in the order they were given.

    Returns:
        The message alone when there is no variable; the message, an em dash and
        the variables otherwise.
    """
    if not variables:
        return message
    written = ", ".join(
        f"{name}={shown(name, value)}" for name, value in variables.items()
    )
    return f"{message} — {written}"


def shown(name: str, value: object) -> str:
    """Write out one variable's content, unless its name forbids it.

    Args:
        name: The variable's name, which is what decides.
        value: Its content.

    Returns:
        The content, cut at `VALUE_LIMIT`; a description of its length for a
        secret; the first characters for a play-through id.
    """
    if is_a_secret(name):
        return hidden(value)
    if is_an_identifier(name):
        return shortened(value)
    return cut(readable(sanitised(value)))


def is_a_secret(name: str) -> bool:
    """Say whether a field's name forbids writing its content.

    Args:
        name: The field's name.

    Returns:
        True if it carries one of `SECRET_NAMES`.
    """
    lowered = name.lower()
    return any(word in lowered for word in SECRET_NAMES)


def is_an_identifier(name: str) -> bool:
    """Say whether a field names a play-through, which is written in part only.

    Args:
        name: The field's name.

    Returns:
        True if it is one of `IDENTIFIER_NAMES`.
    """
    return name.lower() in IDENTIFIER_NAMES


def hidden(value: object) -> str:
    """Describe a secret without writing it: its length, and nothing else.

    Args:
        value: The secret.

    Returns:
        `<absent>` for nothing at all, `<hidden, n characters>` otherwise.
    """
    if value is None:
        return "<absent>"
    return f"<hidden, {len(str(value))} characters>"


def shortened(value: object) -> str:
    """Write the head of a play-through id, enough to follow it through a run.

    Args:
        value: The id.

    Returns:
        `<absent>` for nothing at all, the first characters followed by an
        ellipsis otherwise.
    """
    if value is None:
        return "<absent>"
    written = str(value)
    if len(written) <= IDENTIFIER_LENGTH:
        return repr(written)
    return repr(written[:IDENTIFIER_LENGTH] + "…")


def sanitised(value: object) -> object:
    """Walk a value and replace every secret-named field inside it.

    A body is logged whole, and a body is where a secret travels.

    Args:
        value: Anything about to be written out.

    Returns:
        The same value, with the fields whose name is a secret's replaced by
        their description. Mappings come back as plain dicts, sequences as lists.
    """
    if isinstance(value, Mapping):
        return {
            str(name): hidden(content)
            if is_a_secret(str(name))
            else sanitised(content)
            for name, content in value.items()
        }
    if isinstance(value, (list, tuple)):
        return [sanitised(item) for item in value]
    return value


def readable(value: object) -> str:
    """Write a value the way it is read back: JSON for a structure, `repr` for a
    string.

    `repr` for strings on purpose — an empty one, a trailing space or a stray
    newline are read in the quotes and invisible without them. Anything JSON
    refuses (a date, an object) goes through `str`, as the log wants what it
    looks like, not what it can be reloaded from.

    Args:
        value: The value.

    Returns:
        Its written form.
    """
    if value is None or isinstance(value, (bool, int, float)):
        return str(value)
    if isinstance(value, str):
        return repr(value)
    try:
        return json.dumps(value, ensure_ascii=False, default=str)
    except (TypeError, ValueError):
        return repr(value)


def without_the_secrets(url: str) -> str:
    """A URL as a log may carry it: every parameter but the credentials in it.

    Used on the `Location` of every redirect, which is where this application
    sends a reader after an assault.

    The query is rebuilt unescaped, as it reads rather than as it would be
    replayed: this is a log.

    Args:
        url: The URL.

    Returns:
        The same URL, every secret-named parameter replaced by a description of
        its length.
    """
    parts = urlsplit(url)
    if not parts.query:
        return url
    parameters = [
        f"{name}={hidden(value) if is_a_secret(name) else value}"
        for name, value in parse_qsl(parts.query)
    ]
    return urlunsplit(parts._replace(query="&".join(parameters)))


def cut(text: str) -> str:
    """Shorten what is too long to be worth a whole file, and say what was cut.

    Args:
        text: The written value.

    Returns:
        The text as it is, or its beginning followed by its full length.
    """
    if VALUE_LIMIT <= 0 or len(text) <= VALUE_LIMIT:
        return text
    return f"{text[:VALUE_LIMIT]}… (cut, {len(text)} characters in all)"
