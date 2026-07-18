"""Long-run tick telemetry (§18 M6): the balance-pass instrument.

`cardinal tick <save> --days N --report` prints this summary. It reads
committed state only — running a report never mutates the world.

What to look for:
- npc_needs.min_food near 0 → someone's life loop can't feed them
- market max_price_drift far from 1.0 → a runaway pricing feedback
- populations at 0 → an ecology that can't recover
- quests all expiring → the world's fallback chances are too low
"""

from __future__ import annotations

import json
from typing import Any

from engine.core.registry import Registry
from engine.persistence.store import Store


def report(store: Store, registry: Registry) -> dict[str, Any]:
    day, minute = store.get_clock()
    result: dict[str, Any] = {"clock": {"day": day, "minute": minute}}

    result["chronicle_by_category"] = {
        row["category"]: row["n"] for row in store.conn.execute(
            "SELECT category, COUNT(*) AS n FROM chronicle GROUP BY category ORDER BY category")
    }

    result["quests_by_state"] = {
        row["state"]: row["n"] for row in store.conn.execute(
            "SELECT state, COUNT(*) AS n FROM quests GROUP BY state ORDER BY state")
    }

    # NPC vitals: are their life loops sustainable?
    needs_seen: dict[str, list[float]] = {}
    npc_count = 0
    for row in store.conn.execute("SELECT * FROM entities WHERE kind='npc'"):
        npc_count += 1
        state = json.loads(row["state_json"])
        for need, value in state.get("needs", {}).items():
            needs_seen.setdefault(need, []).append(value)
    result["npcs"] = {
        "count": npc_count,
        "needs": {
            need: {"min": round(min(values), 3),
                   "avg": round(sum(values) / len(values), 3)}
            for need, values in sorted(needs_seen.items())
        },
    }

    result["npc_memories"] = store.conn.execute(
        "SELECT COUNT(*) AS n FROM npc_memory").fetchone()["n"]

    # market drift: biggest departure from base value
    worst_item, worst_ratio = None, 1.0
    market_rows = 0
    for row in store.conn.execute("SELECT * FROM markets"):
        market_rows += 1
        definition = registry.find(row["item_def"])
        base = getattr(definition, "market", None)
        base_value = getattr(base, "base_value_col", 0) if base else 0
        if base_value > 0 and row["price"] > 0:
            ratio = row["price"] / base_value
            if abs(ratio - 1.0) > abs(worst_ratio - 1.0):
                worst_item, worst_ratio = row["item_def"], round(ratio, 3)
    result["markets"] = {"rows": market_rows,
                         "max_price_drift": {"item": worst_item, "ratio": worst_ratio}}

    # ecology: populations vs authored carrying capacity
    populations: dict[str, dict[str, float]] = {}
    capacities: dict[str, int] = {}
    for floor in registry.by_kind("floor"):
        for zone in floor.zones:
            for pop in zone.monster_populations:
                capacities[pop.species] = capacities.get(pop.species, 0) + pop.carrying_capacity
    for row in store.conn.execute("SELECT * FROM entities WHERE kind='zone'"):
        state = json.loads(row["state_json"])
        for species, current in state.get("populations", {}).items():
            entry = populations.setdefault(species, {"current": 0.0})
            entry["current"] += current
    for species, entry in populations.items():
        capacity = capacities.get(species, 0)
        entry["capacity"] = capacity
        entry["ratio"] = round(entry["current"] / capacity, 3) if capacity else None
    result["populations"] = populations

    player = store.get_player()
    if player:
        result["player"] = {k: player[k] for k in
                            ("level", "hp", "hp_max", "col", "alive", "location_id")}
    return result
