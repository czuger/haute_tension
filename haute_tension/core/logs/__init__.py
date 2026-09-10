"""The log: one file, and the two ways of writing into it.

- `rotating_log.py` opens a log file that rotates — the standard handler, on a
  directory it makes again at every open;
- `general_log.py` is the server's own trace, at DEBUG, in `logs/general.log`.

It sits in `core` and not beside the routes because nothing here imports Flask,
and because `core.db` writes to it: a log the database layer could not reach
would be a log missing everything the database layer does.
`application/logs/request_trace.py` is the Flask half, and it is the only half
that knows what a request is.

This file re-exports nothing, like every `__init__.py` of the project:

    from haute_tension.core.logs.general_log import event, note
"""
