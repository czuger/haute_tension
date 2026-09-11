"""One SQLAlchemy model per table — the shape of what is stored.

The models describe the tables and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands plain dicts back, so only this package and `core.db` ever see
a row object.

    page_views       -> PageView, one row per page asked for
    games            -> Game, one play-through: the hero and the fight he is in
    page_inspections -> PageInspection, one flagged page and its comments

Every table follows the hybrid pattern of `HybridDocument`: a real column for
what a query filters or sorts on, one JSON blob for everything else. Import a
model from its own module; nothing is re-exported from here.

The book is in none of them: it is static and read into memory at startup.
"""
