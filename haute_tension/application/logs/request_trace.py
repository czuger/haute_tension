"""Every request written into the general log: what came in, what went out, and
how long it took.

Wired by `create_app` onto the application itself and not onto a blueprint, so
that **nothing is outside it**: a page, a fight, a refusal, an address that
matches no route at all — each one leaves its two lines, `Request` then `Answer`,
and a third when it fails.

    Request — method='GET', path='/combat/22', endpoint='game.show_combat', game='4f2a91c0…'
    Answer  — status=200, type='text/html; charset=utf-8', took='4.2 ms', body=<5264 bytes of text/html>

**A JSON answer is written out in full**, because that is what one comes to this
log for: what the page was actually given, not what it was meant to be. Three
exceptions:

- a **streamed** answer is never read here — reading it would consume it;
- anything **not JSON** is described rather than copied: a page is 12 KB of HTML
  that says nothing a status code does not say better — **unless it is a
  refusal**, from 400 up, where the body carries the sentence explaining it;
- a value longer than `VALUE_LIMIT` is cut and says by how much
  (`general_log.py`).
"""

import time

from flask import Flask, Response, g, request, session
from werkzeug.wrappers.response import Response as BaseResponse

from haute_tension.core.logs.general_log import (
    failure,
    note,
    without_the_secrets,
)
from haute_tension.application.models.game import SESSION_KEY

# When the request started, kept on `g` for the length of that request.
STARTED = "request_opened_at"


def wire_the_request_trace(application: Flask) -> None:
    """Hook the trace onto the application: arrival, answer, and failure.

    Args:
        application: The application being built.
    """
    application.before_request(trace_the_arrival)
    application.after_request(trace_the_answer)
    application.teardown_request(trace_the_failure)


def trace_the_arrival() -> None:
    """Write what came in: the address asked for, who asks, and the payload."""
    setattr(g, STARTED, time.monotonic())
    note(
        "Request",
        method=request.method,
        path=request.path,
        query=request.args.to_dict(),
        endpoint=request.endpoint,
        route=request.view_args,
        game=session.get(SESSION_KEY),
        address=request.remote_addr,
        payload=what_came_in(),
    )


def trace_the_answer(response: BaseResponse) -> BaseResponse:
    """Write what goes out, and hand the answer back untouched.

    Args:
        response: The answer Flask has composed.

    Returns:
        That same answer: an `after_request` handler that returned anything else
        would be serving it.
    """
    note(
        "Answer",
        method=request.method,
        path=request.path,
        status=response.status_code,
        type=response.content_type,
        took=how_long_it_took(),
        body=what_goes_out(response),
        **where_it_sends(response),
    )
    return response


def where_it_sends(response: BaseResponse) -> dict[str, object]:
    """Where a redirect sends, for the lines that are one.

    A redirect's body says nothing and its `Location` says everything: it is what
    a fight is followed on, assault after assault.

    Args:
        response: The answer.

    Returns:
        `{"destination": ...}` for a redirect, nothing at all otherwise, so that
        every other line is not lengthened by a variable that would always be
        empty.
    """
    location = response.headers.get("Location")
    return {"destination": without_the_secrets(location)} if location else {}


def trace_the_failure(trouble: BaseException | None) -> None:
    """Write the exception a request died of, with its traceback.

    Called at the end of every request, with `None` for those that went well — a
    refusal raised by `abort` is one of those: it was handled, and it left as an
    answer with a status of its own.

    Args:
        trouble: What was raised, or `None`.
    """
    if trouble is None:
        return
    failure(
        "Request failed",
        trouble=trouble,
        method=request.method,
        path=request.path,
        endpoint=request.endpoint,
        took=how_long_it_took(),
    )


def how_long_it_took() -> str:
    """How long the request has been in the server's hands.

    Returns:
        The duration in milliseconds; `unknown` where the arrival was not marked
        — an answer composed before the trace was reached, which a
        `before_request` raising would give.
    """
    started = g.get(STARTED)
    if started is None:
        return "unknown"
    return f"{(time.monotonic() - started) * 1000:.1f} ms"


def what_came_in() -> object:
    """The request's payload, in the form it is read back.

    Returns:
        The parsed JSON body, the form as a dict, `None` for a request carrying
        nothing, and a description of anything else.
    """
    if request.is_json:
        return request.get_json(silent=True)
    if request.form:
        return request.form.to_dict()
    if not request.content_length:
        return None
    return f"<{request.content_length} bytes, {request.content_type}>"


def what_goes_out(response: BaseResponse) -> object:
    """The answer's body, in full where reading it costs nothing.

    Args:
        response: The answer.

    Returns:
        The parsed JSON, the text of a refusal, a description of a page, and a
        word for a streamed answer, which is never read here.
    """
    # The refusal first: a 404 raised by the router reaches here as an iterable
    # and would read as streamed, and a refusal is never an endless stream —
    # reading it costs a few hundred bytes.
    if response.status_code >= 400 and not response.direct_passthrough:
        return response.get_json(silent=True) or response.get_data(as_text=True)
    if response.direct_passthrough or response.is_streamed:
        return "<streamed, left untouched>"
    if isinstance(response, Response) and response.is_json:
        return response.get_json(silent=True)
    return f"<{response.content_length} bytes of {response.mimetype}>"
