"""Where a run gets its settings: `.env`, and which database.

The bottom of the package: this module knows about files and environment
variables, and about nothing else here.
"""

import os
import secrets
from pathlib import Path

from dotenv import load_dotenv

ROOT = Path(__file__).resolve().parents[2]

load_dotenv(ROOT / ".env")

# Base database name; the database actually used is this name plus the current
# environment suffix (see `current_db_name`). APP_ENV picks which local Mongo
# database is used, so a dev import never touches prod data, and vice versa.
DB_NAME = "haute_tension"
ENV_NAMES = ("dev", "prod")
DEFAULT_ENV = "dev"


def load_env(required_vars: list[str]) -> dict[str, str]:
    """Read required environment variables, refusing to run without them.

    Args:
        required_vars: Names every caller of this run needs set.

    Returns:
        The variables and their values.

    Raises:
        EnvironmentError: If any of them is missing or empty.
    """
    env = {name: os.environ.get(name, "") for name in required_vars}
    missing = [name for name, value in env.items() if not value]
    if missing:
        raise EnvironmentError(
            f"Missing required environment variables: {', '.join(missing)}. "
            f"Copy .env.example to .env and fill it in."
        )
    return env


def current_env() -> str:
    """Return the environment this run works in.

    Returns:
        Either `"dev"` or `"prod"`.

    Raises:
        EnvironmentError: If APP_ENV is set to anything else.
    """
    env_name = os.environ.get("APP_ENV", DEFAULT_ENV).strip().lower()
    if env_name not in ENV_NAMES:
        raise EnvironmentError(
            f"APP_ENV must be one of {sorted(ENV_NAMES)}, got '{env_name}'"
        )
    return env_name


def current_db_name() -> str:
    """Return the Mongo database this run reads and writes."""
    return f"{DB_NAME}_{current_env()}"


def session_secret() -> str:
    """Return the key the session cookie is signed with.

    The cookie carries a game id and nothing else, so a forged one costs a
    stranger's play-through rather than an account. A generated key is therefore
    good enough for a dev run, and only bad in production — where it would also
    log every reader out at each restart, which is the visible half of the
    problem.

    Returns:
        `FLASK_SECRET_KEY`, or a fresh random key when it is unset.
    """
    return os.environ.get("FLASK_SECRET_KEY") or secrets.token_hex(32)
