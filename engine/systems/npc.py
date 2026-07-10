"""NPC agents: goals, schedules, decisions (§7). M2 milestone.

M1 stub: keeps runtime NPC positions synced to authored starting locations
so perception works; the utility-scoring agent loop lands in M2.
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []


def seed_runtime_state(ctx: SystemContext, day: int) -> list[Delta]:
    """Called once at save creation: materialize every named NPC as a runtime entity."""
    deltas = []
    for npc in ctx.registry.by_kind("npc"):
        deltas.append(
            Delta(
                kind="entity_state",
                payload={
                    "id": npc.id,
                    "kind": "npc",
                    "def_id": npc.id,
                    "state": {"needs": dict(getattr(npc, "needs", {}))},
                },
                location_id=getattr(npc, "location", None),
            )
        )
    return deltas
