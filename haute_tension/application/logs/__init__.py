"""The Flask half of the log: every request written into the general log.

The log itself is `core/logs/`; only `request_trace.py` lives here, because only
it knows what a request is.

    from haute_tension.application.logs.request_trace import wire_the_request_trace
"""
