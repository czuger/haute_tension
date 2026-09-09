"""Everything the Flask app and the import scripts share.

Nothing here knows about Flask: `application/` and `scripts/` import from this
package, never the other way round. The modules are layered so the imports only
ever go one way:

    config      files and environment variables
    models      the shape of what is stored, and nothing else
    db          the database, on top of both
"""
