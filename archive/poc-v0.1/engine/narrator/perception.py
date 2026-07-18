"""Computes what the player can actually perceive (§15).

Hidden information — NPC internal goals, other regions' events, secret
chronicle entries — is excluded from narrator context here, structurally,
not by prompt discipline.
"""

from __future__ import annotations

from typing import Any

from engine.core.registry import Registry
from engine.narrator.base import PerceptionContext
from engine.persistence.store import Delta, Store


def filter_deltas(deltas: list[Delta], player_location: str) -> list[Delta]:
    """A delta is perceivable if it is unscoped (affects the player directly)
    or happened at the player's location and isn't secret."""
    visible: list[Delta] = []
    for delta in deltas:
        if delta.payload.get("visibility") == "secret":
            continue
        if delta.location_id is None or delta.location_id == player_location:
            visible.append(delta)
    return visible


def build_context(store: Store, registry: Registry, clock_label: str,
                  include_status: bool = False) -> PerceptionContext:
    player = store.get_player() or {}
    location_id = player.get("location_id", "")
    location = registry.find(location_id)

    snapshot: dict[str, Any] = {}
    if location is not None:
        snapshot = {
            "id": location.id,
            "name": getattr(location, "name", location.id),
            "safe_zone": getattr(location, "safe_zone", False),
            "services": list(getattr(location, "services", [])),
        }

    # NPCs the player can see: same location, from runtime entity state when
    # present, falling back to authored starting position.
    present_npcs: list[dict[str, Any]] = []
    for npc in registry.by_kind("npc"):
        runtime = store.get_entity(npc.id)
        npc_location = runtime["location_id"] if runtime else getattr(npc, "location", None)
        if npc_location == location_id:
            present_npcs.append({"id": npc.id, "name": getattr(npc, "name", npc.id)})
    present_npcs.sort(key=lambda n: n["id"])

    status: dict[str, Any] = {}
    if include_status and player:
        status = {k: player[k] for k in ("hp", "hp_max", "level", "col") if k in player}

    return PerceptionContext(
        clock_label=clock_label,
        location_snapshot=snapshot,
        present_npcs=present_npcs,
        player_status=status,
    )
