"""World memory — the chronicle (§5.3).

Append-only source of truth for "what has happened in this world."
Writes always go through Delta(kind="chronicle") in a committed turn;
this module provides reads and the save-creation seeding pass.
"""

from __future__ import annotations

from typing import Any

from engine.core.registry import Registry
from engine.persistence.store import Delta, Store


def seed_from_world(registry: Registry) -> list[Delta]:
    """Location `history:` blocks become the chronicle's opening entries."""
    deltas: list[Delta] = []
    for location in registry.by_kind("loc"):
        for seed in getattr(location, "history", []):
            deltas.append(
                Delta(
                    kind="chronicle",
                    payload={
                        "category": "politics",
                        "headline": seed.event,
                        "detail": f"(founding record of {location.id})",
                        "visibility": "public",
                    },
                    location_id=location.id,
                )
            )
    return deltas


def public_entries(store: Store, limit: int = 50) -> list[dict[str, Any]]:
    return [
        dict(row)
        for row in store.conn.execute(
            "SELECT * FROM chronicle WHERE visibility='public' ORDER BY id DESC LIMIT ?",
            (limit,),
        )
    ]
