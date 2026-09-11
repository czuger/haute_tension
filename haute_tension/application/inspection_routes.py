"""Flagging a page as having a problem, and reviewing the pages flagged.

The header's **Signaler** opens a dialog on every page; the form in it posts here
with the page's own path and title filled in. Sent by the browser's script, the
answer is JSON and the reader never leaves the page; sent as a plain form, it is
a flash message and a redirect back to the page, so the feature works without
JavaScript.

There are no users, so nothing here is signed and nothing is restricted:
`/flagged-pages` is a tool for whoever maintains the book.
"""

from flask import (
    Blueprint,
    abort,
    flash,
    jsonify,
    redirect,
    render_template,
    request,
    url_for,
)
from werkzeug.wrappers.response import Response

from haute_tension.core.db import (
    find_inspection,
    flag_page,
    flagged_pages,
    set_inspection_status,
)
from haute_tension.core.models.page_inspection import OPEN, RESOLVED, STATUSES

BAD_REQUEST = 400

FLAGGED = "Merci, la page est signalée pour inspection."
NOTHING_SAID = "Dites en quelques mots ce qui ne va pas."


def create_inspection_blueprint(
    book: str, book_series: str, book_title: str
) -> Blueprint:
    """Create the routes that flag pages and review the flagged ones.

    Args:
        book: The book being served, as `"<series>/<book>"`.
        book_series: Display name of the book series.
        book_title: Display title of the current book.

    Returns:
        A configured Flask blueprint.
    """
    blueprint = Blueprint("inspection", __name__)

    @blueprint.post("/flag-page")
    def report_page() -> Response | tuple[Response, int]:
        """File a report on a page: open the file on it, or add to it.

        A blank remark is refused with a 400 rather than filed: a flag with
        nothing said tells the inspector nothing.
        """
        path = _reported_path()
        text = request.form.get("comment", "").strip()
        if not text:
            return _refused(NOTHING_SAID, path)

        inspection = flag_page(
            book, path, request.form.get("title", "").strip() or None, text
        )
        if _wants_json():
            return jsonify(
                {
                    "success": True,
                    "message": FLAGGED,
                    "comments": len(inspection["comments"]),
                }
            )
        flash(FLAGGED, "success")
        return redirect(path)

    @blueprint.get("/flagged-pages")
    def list_flagged() -> str:
        """The pages flagged for inspection, most recently commented first.

        `?status=open` or `?status=resolved` narrows the list; anything else
        shows everything.
        """
        status = request.args.get("status")
        if status not in STATUSES:
            status = None
        return render_template(
            "flagged_pages.html",
            book_series=book_series,
            book_title=book_title,
            inspections=flagged_pages(book, status),
            status=status,
        )

    @blueprint.get("/flagged-pages/<int:inspection_id>")
    def show_flagged(inspection_id: int) -> str:
        """Everything said about one flagged page."""
        inspection = find_inspection(inspection_id)
        if inspection is None:
            abort(404)
        return render_template(
            "flagged_page.html",
            book_series=book_series,
            book_title=book_title,
            inspection=inspection,
        )

    @blueprint.post("/flagged-pages/<int:inspection_id>/resolve")
    def resolve_flagged(inspection_id: int) -> Response:
        """Close the file on a page."""
        if set_inspection_status(inspection_id, RESOLVED) is None:
            abort(404)
        return redirect(url_for("inspection.show_flagged", inspection_id=inspection_id))

    @blueprint.post("/flagged-pages/<int:inspection_id>/reopen")
    def reopen_flagged(inspection_id: int) -> Response:
        """Open the file on a page again."""
        if set_inspection_status(inspection_id, OPEN) is None:
            abort(404)
        return redirect(url_for("inspection.show_flagged", inspection_id=inspection_id))

    return blueprint


def _reported_path() -> str:
    """The path the form says it was sent from, kept on this site.

    Only a local path is filed or redirected to: an absolute URL in the field
    would make the redirect an open one, so anything else counts as the root.
    """
    path = request.form.get("path", "").strip()
    if path.startswith("/") and not path.startswith("//"):
        return path
    return "/"


def _wants_json() -> bool:
    """Whether the browser's script sent the form, rather than the browser."""
    return request.accept_mimetypes.best == "application/json"


def _refused(message: str, path: str) -> Response | tuple[Response, int]:
    """Send a bad report back with the reason: JSON for a script, a flash otherwise."""
    if _wants_json():
        return jsonify({"success": False, "message": message}), BAD_REQUEST
    flash(message, "error")
    return redirect(path)
