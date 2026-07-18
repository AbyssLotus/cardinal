"""NPC memory (§5.2). M2 milestone.

Rules to implement in M2:
- promises create rows with due conditions; breach generates consequences
- memories decay in salience unless kind ∈ {promise, betrayal} or |valence| ≥ 0.8
- rumor propagation between co-located socializing NPCs, mutating certainty
"""

from __future__ import annotations

from typing import Any

from engine.persistence.store import Store

def remember(store: Store, npc_id: str, day: int, hour: int, kind: str,
             summary: str, subject_id: str | None = None, valence: float = 0.0,
             rumor_certainty: float | None = None) -> None:
    store.add_npc_memory(npc_id, day, hour, kind, summary, subject_id,
                         valence, rumor_certainty)


def memories_of(store: Store, npc_id: str, subject_id: str | None = None,
                limit: int = 20) -> list[dict[str, Any]]:
    if subject_id is None:
        rows = store.conn.execute(
            "SELECT * FROM npc_memory WHERE npc_id=? ORDER BY id DESC LIMIT ?",
            (npc_id, limit),
        )
    else:
        rows = store.conn.execute(
            "SELECT * FROM npc_memory WHERE npc_id=? AND subject_id=? ORDER BY id DESC LIMIT ?",
            (npc_id, subject_id, limit),
        )
    return [dict(row) for row in rows]
