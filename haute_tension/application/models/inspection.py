"""The shape of a flagged page, as the routes and templates read it.

The dict side of `core.models.page_inspection`, converted by `core.db` on the
way out, like `GameDict` for a play-through.
"""

from typing import TypedDict


class CommentDict(TypedDict):
    """One remark on a flagged page."""

    text: str
    created_at: str


class InspectionDict(TypedDict):
    """A flagged page and everything said about it, newest remark last."""

    id: str
    book: str
    path: str
    page_title: str | None
    status: str
    comments: list[CommentDict]
    created_at: str
    updated_at: str
