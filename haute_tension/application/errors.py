"""What a reader is shown when the database cannot answer.

One handler, registered on the application itself so that no route has to think
about it. A page that needs the database and cannot reach it **fails**: it is not
served half-built. The reader gets a page saying so, and the log gets the reason.

`/data/<number>` answers JSON rather than HTML, because that is what its callers
parse; the status is the same.

**503, not 500.** Nothing is wrong with the request or with the application: the
database is not there, and the same request will work once it is.
"""

from flask import Response, jsonify, render_template, request
from werkzeug.wrappers.response import Response as BaseResponse

from haute_tension.core.db import DatabaseFailure
from haute_tension.core.logs.general_log import failure

DATABASE_UNAVAILABLE = 503

# The blueprint whose answers are JSON. Named as Flask names it, so this module
# imports no route.
JSON_BLUEPRINT = "api"


def wire_the_error_pages(application) -> None:
    """Hook the database handler onto every route of the application.

    Args:
        application: The application being built.
    """
    for trouble in DatabaseFailure:
        application.register_error_handler(trouble, database_unavailable)


def database_unavailable(trouble: BaseException) -> tuple[BaseResponse, int]:
    """Report a database that could not answer, and say so in the log.

    Args:
        trouble: What the driver raised, or the missing configuration.

    Returns:
        The page — or the JSON — and a 503.
    """
    failure(
        "The database could not answer, refusing the request",
        trouble=trouble,
        method=request.method,
        path=request.path,
        endpoint=request.endpoint,
    )
    if request.blueprint == JSON_BLUEPRINT:
        return _json_answer(), DATABASE_UNAVAILABLE
    return _page_answer(), DATABASE_UNAVAILABLE


def _json_answer() -> BaseResponse:
    """The refusal as `/data/<number>`'s callers read it."""
    return jsonify(
        {
            "success": False,
            "message": "la base de données est injoignable",
        }
    )


def _page_answer() -> BaseResponse:
    """The refusal as a reader reads it."""
    return Response(
        render_template("database_error.html"),
        content_type="text/html; charset=utf-8",
    )
