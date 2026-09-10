"""Opening a log file that rotates: the standard handler, and the one thing it
does not do.

`logging.handlers.RotatingFileHandler` sets the file aside by size and keeps a
few archives behind it, which is all this log needs. What it does not do is
create the directory it writes into, and `logs/` is not versioned: a fresh clone
has none, and a `git clean -fdx` takes it back off a working one.

Making it once, before handing the handler over, is not enough: the handler
opens its file again at every rotation, and a directory that went away under a
running server would then cost a traceback on stderr per line written —
`FileNotFoundError: … logs/general.log` — for as long as the server ran. Hence
`RotatingLog`, which makes the directory every time it opens the file.
"""

import logging
import logging.handlers
from pathlib import Path

# 512 KB per file and five archives behind it: this log carries whole payloads,
# and yesterday's run should still be there today.
MAX_BYTES = 512 * 1024
FILES_KEPT = 5


class RotatingLog(logging.handlers.RotatingFileHandler):
    """A rotating log file that makes its directory before opening it.

    Every open goes through `_open`: the first one, the one after each rotation,
    and the one a closed handler does on its next line. So the directory is
    remade wherever it went, and the log picks up again by itself rather than
    losing every line until a restart.
    """

    def _open(self):
        """Open the file, its directory made first.

        Returns:
            The stream the handler writes into.
        """
        Path(self.baseFilename).parent.mkdir(parents=True, exist_ok=True)
        return super()._open()


def open_the_log(
    path: Path,
    max_bytes: int = MAX_BYTES,
    files_kept: int = FILES_KEPT,
) -> RotatingLog:
    """Open a rotating log file, creating its directory if it is missing.

    Args:
        path: The current file; the archives are `path.1`, `path.2`, …
        max_bytes: The size beyond which the file is set aside.
        files_kept: How many archives are kept behind it.

    Returns:
        The handler, formatted as the log is read: the time, then the line.
    """
    handler = RotatingLog(
        path, maxBytes=max_bytes, backupCount=files_kept, encoding="utf-8"
    )
    handler.setFormatter(
        logging.Formatter("%(asctime)s  %(message)s", "%Y-%m-%d %H:%M:%S")
    )
    return handler
