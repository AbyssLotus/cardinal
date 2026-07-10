"""Monster populations, migration pressure, respawn (§6, §4.3).

Day-tick per zone: populations regrow toward carrying capacity at
capacity/reproduction_days per day (data from each species' ecology block).
When a predatory species crosses the migration-pressure threshold from
below, it enters the chronicle — the first emergent world event.

Population reduction (hunting, player kills) arrives with combat in M3;
zone_ticks bookkeeping supports §6 catch-up.
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    threshold = ctx.registry.rule("ecology.migration_pressure_threshold", 0.7)
    deltas: list[Delta] = []

    for floor in sorted(ctx.registry.by_kind("floor"), key=lambda f: f.id):
        for zone in floor.zones:
            runtime = ctx.store.get_entity(zone.id)
            populations = (runtime["state"].get("populations") if runtime else None) or {
                pop.species: pop.current for pop in zone.monster_populations
            }
            capacities = {pop.species: pop.carrying_capacity for pop in zone.monster_populations}

            for species, capacity in capacities.items():
                monster = ctx.registry.find(species)
                reproduction_days = (getattr(monster, "ecology", {}) or {}).get(
                    "reproduction_days", 0)
                current = populations.get(species, capacity)
                if reproduction_days > 0 and current < capacity:
                    grown = min(capacity, current + capacity / reproduction_days)
                    was_below = current / capacity < threshold if capacity else False
                    now_above = grown / capacity >= threshold if capacity else False
                    aggression = getattr(monster, "behavior", None)
                    predatory = aggression is not None and aggression.aggression == "predatory"
                    if predatory and was_below and now_above:
                        deltas.append(Delta(
                            kind="chronicle",
                            payload={
                                "category": "disaster",
                                "headline": f"{getattr(monster, 'name', species)} numbers "
                                            f"swell in the wilds of {floor.name}.",
                                "detail": f"({zone.id})",
                                "visibility": "regional",
                            },
                            location_id=floor.id,
                        ))
                    populations[species] = round(grown, 2)

            deltas.append(Delta(
                kind="entity_state",
                payload={"id": zone.id, "kind": "zone", "def_id": floor.id,
                         "state": {"populations": populations}},
                location_id=floor.id,
            ))
            deltas.append(Delta(kind="zone_tick", payload={"zone_id": zone.id}))
    return deltas
