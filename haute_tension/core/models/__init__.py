"""One mongoengine document per collection — the shape of what is stored.

The models describe the collections and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands dicts back to the rest of the app, so only this package and
`core.db` ever see a document object.

    story_pages -> StoryPage (+ its embedded StoryChoice and ElementChange)
    page_views  -> PageView, one row per page asked for
"""

from .page_view import PageView
from .story_page import ElementChange, StoryChoice, StoryPage

__all__ = [
    "ElementChange",
    "PageView",
    "StoryChoice",
    "StoryPage",
]
