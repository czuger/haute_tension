"""Everything the Flask application layer sits on.

Nothing here knows about Flask: `application/` imports from this package, never
the other way round. The modules are layered so the imports only ever go one
way:

    config      files and environment variables
    story       the book, read off disk into memory
    models      the shape of what is stored, and nothing else
    db          the database, on top of config and models

Only the reading history, the play-throughs and the flagged pages are stored,
in one SQLite file per environment. The book is static, so it is read once at
startup rather than kept in a database that would have to be loaded first.
"""
