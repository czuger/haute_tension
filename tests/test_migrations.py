"""The scripts in `migrations/`, applied to a file the previous schema wrote.

Every test starts from a version-0 database laid out the way the code before
migration 001 wrote one — uuid ids, the hero and his bag inside a JSON blob,
instants written by `isoformat()` — applies every script in order, and reads the
result back: row by row with `sqlite3`, and through `core.db` the way the
application will.
"""

import json
import random
import sqlite3
from contextlib import closing
from pathlib import Path

import pytest

from haute_tension.application.factory import create_app
from haute_tension.application.models.game import SESSION_KEY
from haute_tension.core import db as core_db
from haute_tension.core.config import ROOT

MIGRATIONS = sorted((ROOT / "migrations").glob("*.sql"))
FILE = "haute_tension_dev.sqlite3"
BOOK = "pretre_jean/forteresse_alamuth"
OTHER_BOOK = "pretre_jean/autre_livre"

# What the models at commit 5a899b49 created: the schema of a version-0 file.
LEGACY_SCHEMA = """
CREATE TABLE games (
    id VARCHAR(32) NOT NULL,
    book VARCHAR NOT NULL,
    died_at VARCHAR(32),
    data TEXT NOT NULL,
    PRIMARY KEY (id)
);
CREATE TABLE page_inspections (
    id VARCHAR(32) NOT NULL,
    book VARCHAR NOT NULL,
    path VARCHAR NOT NULL,
    status VARCHAR(16) NOT NULL,
    updated_at VARCHAR(32) NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (id),
    CONSTRAINT uq_page_inspections_book_path UNIQUE (book, path)
);
CREATE TABLE page_views (
    id INTEGER NOT NULL,
    book VARCHAR NOT NULL,
    game VARCHAR(32),
    viewed_at VARCHAR(32) NOT NULL,
    data TEXT NOT NULL,
    PRIMARY KEY (id),
    FOREIGN KEY(game) REFERENCES games (id)
);
CREATE INDEX ix_games_book ON games (book);
CREATE INDEX ix_games_book_died_at ON games (book, died_at);
CREATE INDEX ix_page_inspections_book_updated_at ON page_inspections (book, updated_at);
CREATE INDEX ix_page_inspections_status ON page_inspections (status);
CREATE INDEX ix_page_views_book_viewed_at ON page_views (book, viewed_at);
CREATE INDEX ix_page_views_game_viewed_at ON page_views (game, viewed_at);
"""

SWORD = {"element": "sword", "label": "épée", "count": 1}
BAG = {"element": "bag", "label": "sac", "count": 1}
RATIONS = {"element": "ration", "label": "ration", "count": 4}
KEY = {"element": "key", "label": "clé", "count": 1}

LIVING = "4f2a91c0" + "0" * 24
FALLEN = "0b7e4d2a" + "1" * 24
ELSEWHERE = "9c1f4e3b" + "2" * 24
OPEN_FILE = "a1b2c3d4" + "3" * 24
RESOLVED_FILE = "e5f6a7b8" + "4" * 24

FALLEN_DIED_AT = "2026-09-10T09:30:00.000000+00:00"

# The keys migration 001 takes out of a game's blob.
MOVED_OUT = ("force", "vie_max", "vie_actuelle", "gold", "items", "created_at")

LIVING_BLOB = {
    "mode": "normal",
    "force": 13,
    "vie_max": 25,
    "vie_actuelle": 20,
    "force_dice": [3, 4],
    "vie_dice": [5, 2],
    "gold": 7,
    "gold_dice": [1, 2, 3, 1],
    "items": [SWORD, BAG, KEY],
    "pending": [
        {
            "element": "strength point",
            "label": "point de Force",
            "amount": 1,
            "condition": "pendant tout le temps où vous les porterez",
            "note": None,
            "sign": 1,
            "page": "22",
        }
    ],
    "combat": {
        "page": "22",
        "fight_type": "single",
        "enemies": [
            {"name": "collecteur", "force": 6, "vie_max": 10, "vie_actuelle": 6, "damage_adjustment": 0}
        ],
        "assaults": [
            {
                "number": 1,
                "hero_dice": [4, 3],
                "hero_attack_force": 20,
                "exchanges": [
                    {
                        "enemy_name": "collecteur",
                        "enemy_force": 6,
                        "enemy_dice": [2, 2],
                        "enemy_attack_force": 10,
                        "winner": "hero",
                        "damage": 4,
                        "divine_judgement": False,
                    }
                ],
            }
        ],
        "status": "ongoing",
        "on_victory": "621",
        "on_defeat": "death",
        "on_flee": None,
        "has_special_rules": False,
    },
    "created_at": "2026-09-11T08:00:00.500000+00:00",
}
FALLEN_BLOB = {
    "mode": "easy",
    "force": 14,
    "vie_max": 29,
    "vie_actuelle": 0,
    "force_dice": [1, 1],
    "vie_dice": [1, 1, 1],
    "gold": 0,
    "gold_dice": [1, 1, 1, 1],
    "items": [SWORD, BAG, RATIONS],
    "pending": [],
    "combat": None,
    # isoformat() leaves the microseconds out when they are zero.
    "created_at": "2026-09-10T08:00:00+00:00",
    "died_on_page": "63",
    "died_of": "les épreuves du chemin",
}
ELSEWHERE_BLOB = {
    "mode": "normal",
    "force": 9,
    "vie_max": 22,
    "vie_actuelle": 22,
    "force_dice": [1, 2],
    "vie_dice": [2, 2],
    "gold": 12,
    "gold_dice": [3, 3, 3, 3],
    "items": [],
    "pending": [],
    "combat": None,
    "created_at": "2026-09-11T07:00:00.000001+00:00",
}
OPEN_BLOB = {
    "page_title": "Page 22",
    "comments": [
        {"text": "Le choix mène au 23.", "created_at": "2026-09-10T10:00:00.123456+00:00"},
        {"text": "Encore : é, à, ç.", "created_at": "2026-09-10T12:00:00+00:00"},
    ],
    "created_at": "2026-09-10T10:00:00.123456+00:00",
}
RESOLVED_BLOB = {
    "page_title": "Page 5",
    "comments": [{"text": "Illustration manquante.", "created_at": "2026-09-11T10:00:00+00:00"}],
    "created_at": "2026-09-11T10:00:00+00:00",
}


def blob(values: dict) -> str:
    """A blob the way the old code wrote it."""
    return json.dumps(values, ensure_ascii=False)


def legacy_file(directory: Path) -> Path:
    """Write a version-0 database: three heroes, four visits, two flagged pages."""
    path = directory / FILE
    with closing(sqlite3.connect(path)) as connection:
        connection.executescript(LEGACY_SCHEMA)
        connection.executemany(
            "INSERT INTO games (id, book, died_at, data) VALUES (?, ?, ?, ?)",
            [
                (LIVING, BOOK, None, blob(LIVING_BLOB)),
                (FALLEN, BOOK, FALLEN_DIED_AT, blob(FALLEN_BLOB)),
                (ELSEWHERE, OTHER_BOOK, None, blob(ELSEWHERE_BLOB)),
            ],
        )
        connection.executemany(
            "INSERT INTO page_views (id, book, game, viewed_at, data) VALUES (?, ?, ?, ?, ?)",
            [
                (1, BOOK, FALLEN, "2026-09-10T08:05:00.000000+00:00", blob({"page": "1"})),
                (2, BOOK, None, "2026-09-10T08:06:00.000000+00:00", blob({"page": "5"})),
                (3, BOOK, LIVING, "2026-09-11T08:01:00.000000+00:00", blob({"page": "22"})),
                (5, BOOK, LIVING, "2026-09-11T08:02:00.000000+00:00", blob({"page": "23"})),
            ],
        )
        connection.executemany(
            "INSERT INTO page_inspections (id, book, path, status, updated_at, data) "
            "VALUES (?, ?, ?, ?, ?, ?)",
            [
                (OPEN_FILE, BOOK, "/book/22", "open", "2026-09-10T12:00:00.000000+00:00", blob(OPEN_BLOB)),
                (RESOLVED_FILE, BOOK, "/book/5", "resolved", "2026-09-11T11:00:00.250000+00:00", blob(RESOLVED_BLOB)),
            ],
        )
        connection.commit()
    return path


def migrate(path: Path) -> None:
    """Apply every script in `migrations/`, in order, as `sqlite3 -bail` would."""
    with closing(sqlite3.connect(path)) as connection:
        for script in MIGRATIONS:
            connection.executescript(script.read_text(encoding="utf-8"))


def rows(path: Path, query: str) -> list[dict]:
    """Every row a query reads, as dicts."""
    with closing(sqlite3.connect(path)) as connection:
        connection.row_factory = sqlite3.Row
        return [dict(row) for row in connection.execute(query)]


def schema_of(path: Path) -> list[tuple[str, str, str]]:
    """Every table and index a file holds, its SQL with the whitespace collapsed."""
    return [
        (row["type"], row["name"], " ".join(row["sql"].split()))
        for row in rows(
            path,
            "SELECT type, name, sql FROM sqlite_master "
            "WHERE name NOT LIKE 'sqlite%' ORDER BY type, name",
        )
    ]


def everything_in(path: Path) -> list[str]:
    """The whole file as SQL, and the version it carries."""
    with closing(sqlite3.connect(path)) as connection:
        version = connection.execute("PRAGMA user_version").fetchone()[0]
        return [*connection.iterdump(), f"user_version = {version}"]


def without(values: dict, keys: tuple[str, ...]) -> dict:
    """A copy of a dict with some keys left out."""
    return {key: value for key, value in values.items() if key not in keys}


@pytest.fixture
def migrated(tmp_path):
    """A version-0 file, with every migration applied."""
    path = legacy_file(tmp_path)
    migrate(path)
    return path


@pytest.fixture
def through_the_app(migrated, monkeypatch):
    """`core.db` pointed at the migrated file, the way `.env` would point it."""
    monkeypatch.setenv("DATABASE_DIR", str(migrated.parent))
    yield migrated
    core_db.reset_connection()


class TestTheScripts:
    """What `migrations/` holds."""

    def test_they_are_numbered_from_one_without_a_gap(self):
        assert [script.name[:3] for script in MIGRATIONS] == [
            f"{number:03d}" for number in range(1, len(MIGRATIONS) + 1)
        ]

    def test_the_last_one_reaches_the_version_the_code_reads(self):
        assert len(MIGRATIONS) == core_db.SCHEMA_VERSION


class TestTheSchema:
    """What a migrated file is made of."""

    def test_it_is_the_schema_a_new_file_gets(self, migrated, tmp_path):
        fresh = tmp_path / "fresh" / FILE
        fresh.parent.mkdir()
        engine = core_db._open_engine(f"sqlite:///{fresh}")
        core_db._create_the_schema(engine)
        engine.dispose()

        assert schema_of(migrated) == schema_of(fresh)

    def test_it_carries_the_version_the_code_reads(self, migrated):
        assert everything_in(migrated)[-1] == f"user_version = {core_db.SCHEMA_VERSION}"

    def test_nothing_old_is_left_behind(self, migrated):
        assert [name for _, name, _ in schema_of(migrated)] == [
            "ix_games_book",
            "ix_games_book_died_at",
            "ix_items_game_id",
            "ix_page_inspections_book_updated_at",
            "ix_page_inspections_status",
            "ix_page_views_book_viewed_at",
            "ix_page_views_game_id_viewed_at",
            "games",
            "items",
            "page_inspections",
            "page_views",
        ]


class TestTheGames:
    """Renumbered, their stats in columns, their blob otherwise untouched."""

    def test_they_are_numbered_in_the_order_they_were_created(self, migrated):
        games = rows(migrated, "SELECT id, data FROM games ORDER BY id")

        assert [(game["id"], json.loads(game["data"])["legacy_id"]) for game in games] == [
            (1, FALLEN),
            (2, ELSEWHERE),
            (3, LIVING),
        ]

    def test_the_stats_leave_the_blob_for_their_columns(self, migrated):
        [game] = rows(
            migrated, "SELECT force, vie_max, vie_actuelle, gold FROM games WHERE id = 3"
        )

        assert game == {"force": 13, "vie_max": 25, "vie_actuelle": 20, "gold": 7}

    def test_the_rest_of_the_blob_is_kept_as_it_was(self, migrated):
        blobs = [json.loads(game["data"]) for game in rows(migrated, "SELECT data FROM games ORDER BY id")]

        assert blobs == [
            {**without(FALLEN_BLOB, MOVED_OUT), "legacy_id": FALLEN},
            {**without(ELSEWHERE_BLOB, MOVED_OUT), "legacy_id": ELSEWHERE},
            {**without(LIVING_BLOB, MOVED_OUT), "legacy_id": LIVING},
        ]

    def test_created_at_is_padded_to_one_width(self, migrated):
        assert [game["created_at"] for game in rows(migrated, "SELECT created_at FROM games ORDER BY id")] == [
            "2026-09-10T08:00:00.000000+00:00",
            "2026-09-11T07:00:00.000001+00:00",
            "2026-09-11T08:00:00.500000+00:00",
        ]

    def test_updated_at_is_the_last_instant_the_old_file_knew(self, migrated):
        assert [game["updated_at"] for game in rows(migrated, "SELECT updated_at FROM games ORDER BY id")] == [
            FALLEN_DIED_AT,
            "2026-09-11T07:00:00.000001+00:00",
            "2026-09-11T08:00:00.500000+00:00",
        ]


class TestTheBag:
    """One row per line, in bag order."""

    def test_each_line_is_a_row_of_its_game(self, migrated):
        lines = rows(migrated, "SELECT id, game_id, data FROM items ORDER BY id")

        assert [(line["id"], line["game_id"], json.loads(line["data"])) for line in lines] == [
            (1, 1, SWORD),
            (2, 1, BAG),
            (3, 1, RATIONS),
            (4, 3, SWORD),
            (5, 3, BAG),
            (6, 3, KEY),
        ]

    def test_each_line_is_stamped_when_the_migration_ran(self, migrated):
        for line in rows(migrated, "SELECT created_at, updated_at FROM items"):
            assert line["created_at"] == line["updated_at"]
            assert len(line["created_at"]) == len("2026-09-11T08:00:00.500000+00:00")


class TestTheVisits:
    """Their ids kept, their hero followed to his new number."""

    def test_each_visit_keeps_its_id_and_its_hero(self, migrated):
        visits = rows(migrated, "SELECT id, game_id, data FROM page_views ORDER BY id")

        assert [(visit["id"], visit["game_id"], json.loads(visit["data"])) for visit in visits] == [
            (1, 1, {"page": "1"}),
            (2, None, {"page": "5"}),
            (3, 3, {"page": "22"}),
            (5, 3, {"page": "23"}),
        ]


class TestTheFlaggedPages:
    """Renumbered, their dates in columns, their comments untouched."""

    def test_each_file_keeps_its_dates_and_its_thread(self, migrated):
        files = rows(migrated, "SELECT * FROM page_inspections ORDER BY id")

        assert [without(file, ("data",)) for file in files] == [
            {
                "id": 1,
                "book": BOOK,
                "path": "/book/22",
                "status": "open",
                "created_at": "2026-09-10T10:00:00.123456+00:00",
                "updated_at": "2026-09-10T12:00:00.000000+00:00",
            },
            {
                "id": 2,
                "book": BOOK,
                "path": "/book/5",
                "status": "resolved",
                "created_at": "2026-09-11T10:00:00.000000+00:00",
                "updated_at": "2026-09-11T11:00:00.250000+00:00",
            },
        ]
        assert [json.loads(file["data"]) for file in files] == [
            {**without(OPEN_BLOB, ("created_at",)), "legacy_id": OPEN_FILE},
            {**without(RESOLVED_BLOB, ("created_at",)), "legacy_id": RESOLVED_FILE},
        ]


class TestTheApplicationReadsIt:
    """The migrated file, read and written through `core.db`."""

    def test_the_living_hero_is_whole(self, through_the_app):
        hero = core_db.find_game(3)

        assert (hero["force"], hero["vie_max"], hero["vie_actuelle"], hero["gold"]) == (13, 25, 20, 7)
        assert hero["bag"] == [SWORD, BAG, KEY]
        assert hero["pending"][0]["condition"] == "pendant tout le temps où vous les porterez"
        assert hero["combat"]["assaults"][0]["exchanges"][0]["damage"] == 4
        assert not hero["is_dead"]

    def test_the_fallen_are_remembered(self, through_the_app):
        [fallen] = core_db.fallen_heroes(BOOK)

        assert (fallen["id"], fallen["died_on_page"], fallen["died_at"]) == (
            1,
            "63",
            "2026-09-10T09:30:00+00:00",
        )
        assert fallen["bag"] == [SWORD, BAG, RATIONS]

    def test_the_reading_history_reads_as_before(self, through_the_app):
        assert core_db.last_pages(BOOK) == ["1", "5", "22", "23"]
        assert (core_db.last_page_read(3), core_db.last_page_read(1)) == ("23", "1")

    def test_the_flagged_pages_are_listed_as_before(self, through_the_app):
        assert [(filed["id"], filed["path"]) for filed in core_db.flagged_pages(BOOK)] == [
            (2, "/book/5"),
            (1, "/book/22"),
        ]
        assert core_db.find_inspection(2)["created_at"] == "2026-09-11T10:00:00+00:00"

    def test_play_goes_on_where_it_stopped(self, through_the_app):
        gain_a_key = {
            "goto": "24",
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
            "losses": [],
        }

        after = core_db.follow_choice(3, "23", gain_a_key)

        assert after["bag"] == [SWORD, BAG, {**KEY, "count": 2}]
        assert [line["id"] for line in rows(through_the_app, "SELECT id FROM items WHERE game_id = 3")] == [4, 5, 6]

    def test_a_new_hero_is_numbered_after_the_old_ones(self, through_the_app):
        assert core_db.start_game(BOOK, rng=random.Random(1))["id"] == 4

    def test_a_reader_whose_cookie_predates_it_has_no_hero(self, through_the_app, books_path):
        client = create_app(BOOK, books_path).test_client()
        with client.session_transaction() as session:
            session[SESSION_KEY] = LIVING

        assert client.get("/game").status_code == 200
        with client.session_transaction() as session:
            assert SESSION_KEY not in session


class TestTheGuards:
    """What the script refuses, and what a refusal leaves behind."""

    def test_a_second_run_is_refused_and_changes_nothing(self, migrated):
        before = everything_in(migrated)

        with pytest.raises(sqlite3.IntegrityError, match="expects_a_version_0_database"):
            migrate(migrated)

        assert everything_in(migrated) == before

    def test_a_file_the_current_code_created_is_refused(self, tmp_path):
        path = tmp_path / FILE
        engine = core_db._open_engine(f"sqlite:///{path}")
        core_db._create_the_schema(engine)
        engine.dispose()

        with pytest.raises(sqlite3.IntegrityError, match="expects_a_version_0_database"):
            migrate(path)

    def test_a_row_the_new_schema_refuses_keeps_the_old_file_whole(self, tmp_path):
        path = legacy_file(tmp_path)
        with closing(sqlite3.connect(path)) as connection:
            connection.execute(
                "UPDATE games SET data = json_set(data, '$.gold', -1) WHERE id = ?", (LIVING,)
            )
            connection.commit()
        before = everything_in(path)

        with pytest.raises(sqlite3.IntegrityError, match="ck_games_gold_not_negative"):
            migrate(path)

        assert everything_in(path) == before

    def test_a_visit_whose_hero_is_missing_stops_it(self, tmp_path):
        path = legacy_file(tmp_path)
        with closing(sqlite3.connect(path)) as connection:
            connection.execute(
                "INSERT INTO page_views (id, book, game, viewed_at, data) "
                "VALUES (9, ?, 'ghost', '2026-09-11T12:00:00.000000+00:00', '{}')",
                (BOOK,),
            )
            connection.commit()
        before = everything_in(path)

        with pytest.raises(sqlite3.IntegrityError, match="every_visit_kept_its_hero"):
            migrate(path)

        assert everything_in(path) == before
