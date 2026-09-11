-- Migration 001: integer ids, the bag as rows, the hero's stats as columns, and
-- created_at / updated_at on every table whose rows change.
--
-- Moves a database file from schema version 0 (what the code up to commit
-- 5a899b49 wrote: uuid ids, and everything but a few columns in one JSON blob)
-- to version 1, the schema core/models/ describes.
--
-- Back the file up, then apply the script with -bail, so the first error stops
-- it before anything is committed:
--
--     cp data/haute_tension_dev.sqlite3 data/haute_tension_dev.sqlite3.before-001
--     sqlite3 -bail data/haute_tension_dev.sqlite3 < migrations/001_integer_ids_items_and_stats.sql
--
-- The script is one transaction. It refuses to start on anything but a version-0
-- file, and before it commits it checks that every game, bag line, visit and
-- flagged page came across and that every reference resolves. A failed check
-- stops it, and the file is left exactly as it was.
--
-- Table by table:
--
--   games             renumbered 1, 2, 3... in the order they were created.
--                     force, vie_max, vie_actuelle and gold leave the blob for
--                     columns of their own, and so does created_at. updated_at
--                     is the last instant the old schema recorded: died_at, or
--                     else created_at. The old uuid stays in the blob as
--                     legacy_id, so old log lines can still be traced.
--   items             new: one row per line of each game's bag, in bag order,
--                     stamped with the time of the migration, since the old
--                     schema never recorded when a thing was picked up.
--   page_views        keep their ids. game, a uuid, becomes game_id, the new
--                     integer. A visit made without a hero stays without one.
--   page_inspections  renumbered like the games. created_at leaves the blob,
--                     updated_at is kept, and the old uuid stays as legacy_id.
--
-- The old code wrote instants in the blob with isoformat(), which leaves the
-- microseconds out when they are zero. Those are padded here, so every
-- timestamp column holds text of one width and sorts in time order.

PRAGMA foreign_keys = OFF;

BEGIN;

-- 1. Refuse anything but a file the previous schema wrote.

CREATE TEMP TABLE migration_guard (
    ready INTEGER NOT NULL
        CONSTRAINT expects_a_version_0_database_with_uuid_game_ids CHECK (ready = 1)
);
INSERT INTO temp.migration_guard
SELECT (SELECT user_version FROM pragma_user_version) = 0
   AND EXISTS (
       SELECT 1 FROM pragma_table_info('games') WHERE name = 'id' AND type = 'VARCHAR(32)'
   );

-- 2. Set the old tables aside. Their indexes keep their names until the tables
--    are dropped, which is why the new indexes are only created at the end.

ALTER TABLE page_views RENAME TO legacy_page_views;
ALTER TABLE page_inspections RENAME TO legacy_page_inspections;
ALTER TABLE games RENAME TO legacy_games;

-- 3. The new tables, as SQLAlchemy creates them from core/models/.

CREATE TABLE games (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    book VARCHAR NOT NULL,
    force SMALLINT NOT NULL,
    vie_max SMALLINT NOT NULL,
    vie_actuelle SMALLINT NOT NULL,
    gold SMALLINT NOT NULL,
    died_at VARCHAR(32),
    created_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    updated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    data JSON NOT NULL,
    CONSTRAINT ck_games_force_not_negative CHECK (force >= 0),
    CONSTRAINT ck_games_vie_max_not_negative CHECK (vie_max >= 0),
    CONSTRAINT ck_games_vie_actuelle_not_negative CHECK (vie_actuelle >= 0),
    CONSTRAINT ck_games_gold_not_negative CHECK (gold >= 0)
);

CREATE TABLE page_inspections (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    book VARCHAR NOT NULL,
    path VARCHAR NOT NULL,
    status VARCHAR(16) NOT NULL,
    created_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    updated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    data JSON NOT NULL,
    CONSTRAINT uq_page_inspections_book_path UNIQUE (book, path)
);

CREATE TABLE items (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL,
    created_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    updated_at VARCHAR(32) DEFAULT (strftime('%Y-%m-%dT%H:%M:%f000+00:00', 'now')) NOT NULL,
    data JSON NOT NULL,
    FOREIGN KEY(game_id) REFERENCES games (id)
);

CREATE TABLE page_views (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    book VARCHAR NOT NULL,
    game_id INTEGER,
    viewed_at VARCHAR(32) NOT NULL,
    data JSON NOT NULL,
    FOREIGN KEY(game_id) REFERENCES games (id)
);

-- 4. Number the games and the flagged pages in the order they were created,
--    padding each created_at to the fixed width on the way.

CREATE TEMP TABLE game_numbers AS
WITH raw AS (
    SELECT id AS legacy_id, json_extract(data, '$.created_at') AS created_at
    FROM legacy_games
), padded AS (
    SELECT
        legacy_id,
        CASE length(created_at)
            WHEN 25 THEN substr(created_at, 1, 19) || '.000000' || substr(created_at, 20)
            ELSE created_at
        END AS created_at
    FROM raw
)
SELECT legacy_id, created_at, row_number() OVER (ORDER BY created_at, legacy_id) AS id
FROM padded;

CREATE TEMP TABLE inspection_numbers AS
WITH raw AS (
    SELECT id AS legacy_id, json_extract(data, '$.created_at') AS created_at
    FROM legacy_page_inspections
), padded AS (
    SELECT
        legacy_id,
        CASE length(created_at)
            WHEN 25 THEN substr(created_at, 1, 19) || '.000000' || substr(created_at, 20)
            ELSE created_at
        END AS created_at
    FROM raw
)
SELECT legacy_id, created_at, row_number() OVER (ORDER BY created_at, legacy_id) AS id
FROM padded;

-- 5. Copy the rows across.

INSERT INTO games (
    id, book, force, vie_max, vie_actuelle, gold, died_at, created_at, updated_at, data
)
SELECT
    numbered.id,
    game.book,
    json_extract(game.data, '$.force'),
    json_extract(game.data, '$.vie_max'),
    json_extract(game.data, '$.vie_actuelle'),
    coalesce(json_extract(game.data, '$.gold'), 0),
    game.died_at,
    numbered.created_at,
    coalesce(game.died_at, numbered.created_at),
    json_set(
        json_remove(
            game.data,
            '$.force', '$.vie_max', '$.vie_actuelle', '$.gold', '$.items', '$.created_at'
        ),
        '$.legacy_id', game.id
    )
FROM legacy_games AS game
JOIN temp.game_numbers AS numbered ON numbered.legacy_id = game.id
ORDER BY numbered.id;

INSERT INTO items (game_id, data)
SELECT numbered.id, line.value
FROM legacy_games AS game
JOIN temp.game_numbers AS numbered ON numbered.legacy_id = game.id
JOIN json_each(game.data, '$.items') AS line
ORDER BY numbered.id, line.key;

INSERT INTO page_views (id, book, game_id, viewed_at, data)
SELECT visit.id, visit.book, numbered.id, visit.viewed_at, visit.data
FROM legacy_page_views AS visit
LEFT JOIN temp.game_numbers AS numbered ON numbered.legacy_id = visit.game
ORDER BY visit.id;

INSERT INTO page_inspections (id, book, path, status, created_at, updated_at, data)
SELECT
    numbered.id,
    inspection.book,
    inspection.path,
    inspection.status,
    numbered.created_at,
    inspection.updated_at,
    json_set(json_remove(inspection.data, '$.created_at'), '$.legacy_id', inspection.id)
FROM legacy_page_inspections AS inspection
JOIN temp.inspection_numbers AS numbered ON numbered.legacy_id = inspection.id
ORDER BY numbered.id;

-- 6. Check that everything came across, before anything old is dropped.

CREATE TEMP TABLE migration_checks (
    games INTEGER NOT NULL
        CONSTRAINT every_game_came_across CHECK (games = 1),
    items INTEGER NOT NULL
        CONSTRAINT every_bag_line_came_across CHECK (items = 1),
    visits INTEGER NOT NULL
        CONSTRAINT every_visit_came_across CHECK (visits = 1),
    heroes INTEGER NOT NULL
        CONSTRAINT every_visit_kept_its_hero CHECK (heroes = 1),
    inspections INTEGER NOT NULL
        CONSTRAINT every_flagged_page_came_across CHECK (inspections = 1),
    links INTEGER NOT NULL
        CONSTRAINT every_reference_resolves CHECK (links = 1)
);
INSERT INTO temp.migration_checks
SELECT
    (SELECT count(*) FROM games) = (SELECT count(*) FROM legacy_games),
    (SELECT count(*) FROM items)
        = (SELECT coalesce(sum(json_array_length(data, '$.items')), 0) FROM legacy_games),
    (SELECT count(*) FROM page_views) = (SELECT count(*) FROM legacy_page_views),
    (SELECT count(*) FROM page_views WHERE game_id IS NULL)
        = (SELECT count(*) FROM legacy_page_views WHERE game IS NULL),
    (SELECT count(*) FROM page_inspections) = (SELECT count(*) FROM legacy_page_inspections),
    NOT EXISTS (SELECT 1 FROM pragma_foreign_key_check('items'))
        AND NOT EXISTS (SELECT 1 FROM pragma_foreign_key_check('page_views'));

-- 7. Drop the old tables and the scaffolding, index the new tables, and mark
--    the file with the version it now carries.

DROP TABLE legacy_page_views;
DROP TABLE legacy_page_inspections;
DROP TABLE legacy_games;

DROP TABLE temp.migration_guard;
DROP TABLE temp.game_numbers;
DROP TABLE temp.inspection_numbers;
DROP TABLE temp.migration_checks;

CREATE INDEX ix_games_book ON games (book);
CREATE INDEX ix_games_book_died_at ON games (book, died_at);
CREATE INDEX ix_page_inspections_book_updated_at ON page_inspections (book, updated_at);
CREATE INDEX ix_page_inspections_status ON page_inspections (status);
CREATE INDEX ix_items_game_id ON items (game_id);
CREATE INDEX ix_page_views_book_viewed_at ON page_views (book, viewed_at);
CREATE INDEX ix_page_views_game_id_viewed_at ON page_views (game_id, viewed_at);

PRAGMA user_version = 1;

COMMIT;

PRAGMA foreign_keys = ON;
