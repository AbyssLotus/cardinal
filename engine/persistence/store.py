"""SQLite persistence layer (§14).

All step-7 COMMITs happen inside a single transaction via `transaction()`;
a crash mid-turn loses at most the un-narrated turn. Deltas are the unit
of committed change — every system produces them, `apply_deltas` writes them.
"""

from __future__ import annotations

import json
import sqlite3
from contextlib import contextmanager
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Iterator

SCHEMA_PATH = Path(__file__).parent / "schema.sql"


@dataclass(frozen=True)
class Delta:
    """One committed state change. `kind` selects the apply handler; `payload`
    carries the row data; `location_id` scopes perception (§15)."""

    kind: str
    payload: dict[str, Any] = field(default_factory=dict)
    location_id: str | None = None


class Store:
    def __init__(self, db_path: str | Path):
        self.db_path = Path(db_path)
        self.conn = sqlite3.connect(self.db_path)
        self.conn.row_factory = sqlite3.Row
        self.conn.execute("PRAGMA foreign_keys = ON")

    @classmethod
    def create(cls, db_path: str | Path) -> "Store":
        store = cls(db_path)
        store.conn.executescript(SCHEMA_PATH.read_text(encoding="utf-8"))
        store.conn.execute(
            "INSERT OR IGNORE INTO world_clock(id, day, minute) VALUES (1, 0, 0)"
        )
        store.conn.commit()
        return store

    def close(self) -> None:
        self.conn.close()

    @contextmanager
    def transaction(self) -> Iterator[sqlite3.Connection]:
        try:
            yield self.conn
            self.conn.commit()
        except Exception:
            self.conn.rollback()
            raise

    # --- clock -------------------------------------------------------------

    def get_clock(self) -> tuple[int, int]:
        row = self.conn.execute("SELECT day, minute FROM world_clock WHERE id=1").fetchone()
        return (row["day"], row["minute"])

    def set_clock(self, day: int, minute: int) -> None:
        self.conn.execute("UPDATE world_clock SET day=?, minute=? WHERE id=1", (day, minute))

    # --- rng ---------------------------------------------------------------

    def save_rng(self, states: dict[str, str]) -> None:
        self.conn.executemany(
            "INSERT INTO rng_streams(name, state) VALUES(?, ?) "
            "ON CONFLICT(name) DO UPDATE SET state=excluded.state",
            states.items(),
        )

    def load_rng(self) -> dict[str, str]:
        return {
            row["name"]: row["state"]
            for row in self.conn.execute("SELECT name, state FROM rng_streams")
        }

    # --- entities ----------------------------------------------------------

    def upsert_entity(
        self, entity_id: str, kind: str, def_id: str,
        state: dict[str, Any], location_id: str | None, day: int,
    ) -> None:
        self.conn.execute(
            "INSERT INTO entities(id, kind, def_id, state_json, location_id, updated_day) "
            "VALUES(?, ?, ?, ?, ?, ?) "
            "ON CONFLICT(id) DO UPDATE SET state_json=excluded.state_json, "
            "location_id=excluded.location_id, updated_day=excluded.updated_day",
            (entity_id, kind, def_id, json.dumps(state), location_id, day),
        )

    def get_entity(self, entity_id: str) -> dict[str, Any] | None:
        row = self.conn.execute("SELECT * FROM entities WHERE id=?", (entity_id,)).fetchone()
        if row is None:
            return None
        result = dict(row)
        result["state"] = json.loads(result.pop("state_json"))
        return result

    # --- player ------------------------------------------------------------

    def init_player(self, name: str, level: int, col: int, location_id: str, hp_max: int = 100) -> None:
        self.conn.execute(
            "INSERT INTO player(id, name, level, col, hp, hp_max, location_id) "
            "VALUES(1, ?, ?, ?, ?, ?, ?)",
            (name, level, col, hp_max, hp_max, location_id),
        )

    def get_player(self) -> dict[str, Any] | None:
        row = self.conn.execute("SELECT * FROM player WHERE id=1").fetchone()
        return dict(row) if row else None

    def update_player(self, **fields: Any) -> None:
        allowed = {"name", "level", "xp", "hp", "hp_max", "col", "location_id", "alive"}
        unknown = set(fields) - allowed
        if unknown:
            raise ValueError(f"unknown player fields: {unknown}")
        sets = ", ".join(f"{k}=?" for k in fields)
        self.conn.execute(f"UPDATE player SET {sets} WHERE id=1", tuple(fields.values()))

    def add_player_history(self, day: int, hour: int, kind: str, summary: str,
                           refs: list[str] | None = None) -> None:
        self.conn.execute(
            "INSERT INTO player_history(day, hour, kind, summary, refs_json) VALUES(?,?,?,?,?)",
            (day, hour, kind, summary, json.dumps(refs or [])),
        )

    # --- inventory -----------------------------------------------------------

    def add_item_instance(self, instance_id: str, def_id: str, owner_id: str,
                          durability: int | None = None, qty: int = 1) -> None:
        self.conn.execute(
            "INSERT INTO item_instances(id, def_id, owner_id, durability, qty) VALUES(?,?,?,?,?)",
            (instance_id, def_id, owner_id, durability, qty),
        )

    def get_inventory(self, owner_id: str) -> list[dict[str, Any]]:
        return [
            dict(row)
            for row in self.conn.execute(
                "SELECT * FROM item_instances WHERE owner_id=? ORDER BY id", (owner_id,)
            )
        ]

    # --- modifiers ---------------------------------------------------------------

    def add_modifier(self, owner_id: str, def_id: str, day: int,
                     expires_day: float | None = None,
                     state: dict[str, Any] | None = None) -> None:
        self.conn.execute(
            "INSERT INTO modifiers(owner_id, def_id, acquired_day, expires_day, state_json) "
            "VALUES(?,?,?,?,?)",
            (owner_id, def_id, day, expires_day, json.dumps(state or {})),
        )

    def get_modifiers(self, owner_id: str, active_only: bool = True) -> list[dict[str, Any]]:
        query = "SELECT * FROM modifiers WHERE owner_id=?"
        if active_only:
            query += " AND active=1"
        rows = self.conn.execute(query + " ORDER BY id", (owner_id,))
        results = []
        for row in rows:
            record = dict(row)
            record["state"] = json.loads(record.pop("state_json"))
            results.append(record)
        return results

    def deactivate_modifier(self, owner_id: str, def_id: str) -> None:
        self.conn.execute(
            "UPDATE modifiers SET active=0 WHERE owner_id=? AND def_id=? AND active=1",
            (owner_id, def_id),
        )

    # --- chronicle -------------------------------------------------------------

    def add_chronicle(self, day: int, hour: int, category: str, headline: str,
                      detail: str = "", actors: list[str] | None = None,
                      location_id: str | None = None, visibility: str = "public") -> None:
        self.conn.execute(
            "INSERT INTO chronicle(day, hour, category, headline, detail, actors_json, "
            "location_id, visibility) VALUES(?,?,?,?,?,?,?,?)",
            (day, hour, category, headline, detail, json.dumps(actors or []),
             location_id, visibility),
        )

    def get_chronicle(self, limit: int = 50) -> list[dict[str, Any]]:
        return [
            dict(row)
            for row in self.conn.execute(
                "SELECT * FROM chronicle ORDER BY id DESC LIMIT ?", (limit,)
            )
        ]

    # --- deltas -------------------------------------------------------------------

    def apply_deltas(self, deltas: list[Delta], day: int, hour: int) -> None:
        """Write a turn's deltas. Caller wraps this (plus clock/rng writes) in
        one `transaction()` — this method never commits on its own."""
        for delta in deltas:
            handler = _DELTA_HANDLERS.get(delta.kind)
            if handler is None:
                raise ValueError(f"no delta handler for kind {delta.kind!r}")
            handler(self, delta, day, hour)


def _apply_player_update(store: Store, delta: Delta, day: int, hour: int) -> None:
    store.update_player(**delta.payload)


def _apply_player_history(store: Store, delta: Delta, day: int, hour: int) -> None:
    p = delta.payload
    store.add_player_history(day, hour, p["kind"], p["summary"], p.get("refs"))


def _apply_chronicle(store: Store, delta: Delta, day: int, hour: int) -> None:
    p = delta.payload
    store.add_chronicle(day, hour, p["category"], p["headline"], p.get("detail", ""),
                        p.get("actors"), delta.location_id, p.get("visibility", "public"))


def _apply_entity_state(store: Store, delta: Delta, day: int, hour: int) -> None:
    p = delta.payload
    store.upsert_entity(p["id"], p["kind"], p.get("def_id", p["id"]),
                        p.get("state", {}), delta.location_id, day)


def _apply_zone_tick(store: Store, delta: Delta, day: int, hour: int) -> None:
    store.conn.execute(
        "INSERT INTO zone_ticks(zone_id, last_ticked_day) VALUES(?, ?) "
        "ON CONFLICT(zone_id) DO UPDATE SET last_ticked_day=excluded.last_ticked_day",
        (delta.payload["zone_id"], day),
    )


def _apply_modifier_add(store: Store, delta: Delta, day: int, hour: int) -> None:
    p = delta.payload
    store.add_modifier(p["owner_id"], p["def_id"], day,
                       p.get("expires_day"), p.get("state"))


def _apply_modifier_remove(store: Store, delta: Delta, day: int, hour: int) -> None:
    store.deactivate_modifier(delta.payload["owner_id"], delta.payload["def_id"])


_DELTA_HANDLERS = {
    "player_update": _apply_player_update,
    "player_history": _apply_player_history,
    "chronicle": _apply_chronicle,
    "entity_state": _apply_entity_state,
    "zone_tick": _apply_zone_tick,
    "modifier_add": _apply_modifier_add,
    "modifier_remove": _apply_modifier_remove,
}
