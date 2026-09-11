"""One SQLAlchemy model per table — the shape of what is stored.

The models describe the tables and nothing else: no module here opens a
connection or runs a query. `core.db` holds the connection and every read and
write, and hands plain dicts back, so only this package and `core.db` ever see
a row object.

    games            -> Game, one play-through: the hero and the fight he is in
    items            -> Item, one line of a hero's bag
    page_views       -> PageView, one row per page asked for
    page_inspections -> PageInspection, one flagged page and its comments

Every table follows the hybrid pattern of `HybridDocument`: a real column for
what a query filters, sorts or constrains, one JSON blob for everything else.
`Timestamped` adds `created_at` and `updated_at` to the tables whose rows
change. Import a model from its own module; nothing is re-exported from here.

The book is in none of them: it is static and read into memory at startup.
"""
