"""Player memory (§5.1). Reads/writes the player_* tables via the store.

M1: thin query layer; reputation math and quest-log integration land in M3/M4.
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Store


def recent_history(store: Store, limit: int = 10) -> list[dict[str, Any]]:
    return [
        dict(row)
        for row in store.conn.execute(
            "SELECT * FROM player_history ORDER BY id DESC LIMIT ?", (limit,)
        )
    ]


def reputation(store: Store, scope_id: str) -> float:
    row = store.conn.execute(
        "SELECT value FROM player_reputation WHERE scope_id=?", (scope_id,)
    ).fetchone()
    return row["value"] if row else 0.0
