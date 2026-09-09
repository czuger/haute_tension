"""Opening a log file that rotates: the standard handler, and the one thing it
does not do.

`logging.handlers.RotatingFileHandler` sets the file aside by size and keeps a
few archives behind it, which is all this log needs. What it does not do is
create the directory it writes into, and `logs/` is not versioned: a fresh clone
has none. Hence this one function.
"""

import logging
import logging.handlers
from pathlib import Path

# 512 KB per file and five archives behind it: this log carries whole payloads,
# and yesterday's run should still be there today.
MAX_BYTES = 512 * 1024
FILES_KEPT = 5


def open_the_log(
    path: Path,
    max_bytes: int = MAX_BYTES,
    files_kept: int = FILES_KEPT,
) -> logging.handlers.RotatingFileHandler:
    """Open a rotating log file, creating its directory if it is missing.

    Args:
        path: The current file; the archives are `path.1`, `path.2`, …
        max_bytes: The size beyond which the file is set aside.
        files_kept: How many archives are kept behind it.

    Returns:
        The handler, formatted as the log is read: the time, then the line.
    """
    path.parent.mkdir(parents=True, exist_ok=True)
    handler = logging.handlers.RotatingFileHandler(
        path, maxBytes=max_bytes, backupCount=files_kept, encoding="utf-8"
    )
    handler.setFormatter(
        logging.Formatter("%(asctime)s  %(message)s", "%Y-%m-%d %H:%M:%S")
    )
    return handler
