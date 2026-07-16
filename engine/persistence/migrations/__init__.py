"""Save-format migrations (§14, M6).

`meta.yaml` carries `save_format`. When `open_save` finds an older format,
each pending migration runs in order inside a transaction, then the meta file
is rewritten. Migrations are append-only: never edit an existing one — add
the next number.

Migration 0 -> 1 upgrades saves created before the format was versioned
(early M1-M3 saves): it adds the columns and tables that arrived during
development, idempotently.
"""

from __future__ import annotations

from typing import Callable

from engine.persistence.store import Store

CURRENT_SAVE_FORMAT = 3


def _migrate_0_to_1(store: Store) -> None:
    """Bring pre-versioned saves up to the M5 schema."""
    columns = {row["name"] for row in
               store.conn.execute("PRAGMA table_info(item_instances)")}
    if "equipped" not in columns:
        store.conn.execute(
            "ALTER TABLE item_instances ADD COLUMN equipped INTEGER NOT NULL DEFAULT 0")
    if "qty" not in columns:
        store.conn.execute(
            "ALTER TABLE item_instances ADD COLUMN qty INTEGER NOT NULL DEFAULT 1")
    store.conn.execute("""
        CREATE TABLE IF NOT EXISTS modifiers (
            id           INTEGER PRIMARY KEY AUTOINCREMENT,
            owner_id     TEXT NOT NULL,
            def_id       TEXT NOT NULL,
            acquired_day INTEGER NOT NULL DEFAULT 0,
            expires_day  REAL,
            active       INTEGER NOT NULL DEFAULT 1,
            state_json   TEXT NOT NULL DEFAULT '{}'
        )""")
    store.conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_modifiers_owner ON modifiers(owner_id)")


def _migrate_1_to_2(store: Store) -> None:
    """Quest assignment & competition (§23): quests gain an assignee."""
    columns = {row["name"] for row in
               store.conn.execute("PRAGMA table_info(quests)")}
    if "assignee" not in columns:
        store.conn.execute("ALTER TABLE quests ADD COLUMN assignee TEXT")


def _migrate_2_to_3(store: Store) -> None:
    """Dynamic entities (§24.5): runtime-minted definitions."""
    store.conn.execute("""
        CREATE TABLE IF NOT EXISTS dynamic_entities (
            id          TEXT PRIMARY KEY,
            kind        TEXT NOT NULL,
            def_json    TEXT NOT NULL,
            created_day INTEGER NOT NULL DEFAULT 0
        )""")


MIGRATIONS: dict[int, Callable[[Store], None]] = {
    0: _migrate_0_to_1,
    1: _migrate_1_to_2,
    2: _migrate_2_to_3,
}


def migrate(store: Store, from_format: int) -> int:
    """Run all pending migrations; returns the resulting format version.
    Raises if the save is from a NEWER engine than this one."""
    if from_format > CURRENT_SAVE_FORMAT:
        raise RuntimeError(
            f"save format {from_format} is newer than this engine's "
            f"{CURRENT_SAVE_FORMAT} — upgrade the engine, don't downgrade the save")
    version = from_format
    while version < CURRENT_SAVE_FORMAT:
        step = MIGRATIONS.get(version)
        if step is None:
            raise RuntimeError(f"no migration registered from save format {version}")
        with store.transaction():
            step(store)
        version += 1
    return version
