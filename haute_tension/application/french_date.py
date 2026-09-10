"""A date as the templates print it: "10 septembre 2026".

A Jinja filter rather than a field built in `core.db`: the memorial stores when
a hero fell as an instant, and how that instant reads is the page's business.
"""

from datetime import datetime

MONTHS_FR = (
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
)


def french_date(instant: str | None) -> str:
    """Print an ISO 8601 instant as a French date.

    Args:
        instant: The instant as `core.db` hands it out, or `None`.

    Returns:
        The day, month and year in French, or an empty string for `None`.
    """
    if not instant:
        return ""
    moment = datetime.fromisoformat(instant)
    return f"{moment.day} {MONTHS_FR[moment.month - 1]} {moment.year}"
