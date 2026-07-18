"""Monster populations, migration pressure, respawn (§6, §4.3, §20.3).

Day-tick per zone, per species:

- **Logistic regrowth** (spec §20.3): ``dP = r · P · (1 − P/K)`` with
  ``r = 4 / reproduction_days`` — the max growth slope (at P = K/2)
  equals the old linear rate ``K / reproduction_days``, so authored
  ``reproduction_days`` values keep their intuitive meaning while
  near-extinction now recovers slowly. Grinding a species to 2% of
  capacity finally costs the world something linear regrowth never
  charged.
- **Extinction & recolonization:** a species at zero stays gone for
  ``ecology.recolonize_days`` (default 90; ≤ 0 makes extinction
  permanent), then reseeds at 2% of capacity from off-zone drift, with
  a chronicle entry. Extinction pressure is real but the world can
  heal — unless the world rules say otherwise.
- **Scarcity/recovery chronicle hooks** for ALL aggression types (the
  Test 1 gap: only predatory overpopulation used to make the story
  layer): crossing below 10% of capacity chronicles scarcity; climbing
  back past 50% chronicles recovery.
- The predatory migration-pressure event is unchanged.
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext

SCARCE_RATIO = 0.1
RECOVERED_RATIO = 0.5
RESEED_RATIO = 0.02


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    if granularity != "day":
        return []
    threshold = ctx.registry.rule("ecology.migration_pressure_threshold", 0.7)
    recolonize_days = ctx.registry.rule("ecology.recolonize_days", 90)
    deltas: list[Delta] = []

    for floor in sorted(ctx.registry.by_kind("floor"), key=lambda f: f.id):
        for zone in floor.zones:
            runtime = ctx.store.get_entity(zone.id)
            state = dict(runtime["state"]) if runtime else {}
            populations = state.get("populations") or {
                pop.species: pop.current for pop in zone.monster_populations
            }
            extinct_since = state.get("extinct_since", {})
            scarce_flags = state.get("scarce", {})
            capacities = {pop.species: pop.carrying_capacity
                          for pop in zone.monster_populations}

            for species, capacity in capacities.items():
                monster = ctx.registry.find(species)
                name = getattr(monster, "name", species)
                reproduction_days = (getattr(monster, "ecology", {}) or {}).get(
                    "reproduction_days", 0)
                current = populations.get(species, capacity)

                # --- extinction & recolonization -------------------------
                if current <= 0:
                    since = extinct_since.get(species)
                    if since is None:
                        extinct_since[species] = day
                    elif recolonize_days > 0 and day - since >= recolonize_days:
                        current = max(1.0, capacity * RESEED_RATIO)
                        extinct_since.pop(species, None)
                        deltas.append(_chronicle(
                            f"{name} are seen again in the wilds of {floor.name}.",
                            f"recolonized {zone.id} after {day - since} empty days",
                            floor.id))
                    populations[species] = round(current, 2)
                    if current <= 0:
                        continue
                else:
                    extinct_since.pop(species, None)

                # --- logistic regrowth ------------------------------------
                if reproduction_days > 0 and 0 < current < capacity:
                    r = 4.0 / reproduction_days
                    grown = current + r * current * (1.0 - current / capacity)
                    grown = min(capacity, grown)

                    was_below = current / capacity < threshold if capacity else False
                    now_above = grown / capacity >= threshold if capacity else False
                    behavior = getattr(monster, "behavior", None)
                    predatory = behavior is not None and behavior.aggression == "predatory"
                    if predatory and was_below and now_above:
                        deltas.append(_chronicle(
                            f"{name} numbers swell in the wilds of {floor.name}.",
                            f"({zone.id})", floor.id))
                    populations[species] = round(grown, 2)
                    current = grown

                # --- scarcity / recovery hooks (all aggression types) -----
                ratio = current / capacity if capacity else 1.0
                if ratio < SCARCE_RATIO and not scarce_flags.get(species):
                    scarce_flags[species] = True
                    deltas.append(_chronicle(
                        f"{name} grow scarce around {floor.name}.",
                        f"population at {ratio:.0%} of capacity in {zone.id}",
                        floor.id))
                elif ratio >= RECOVERED_RATIO and scarce_flags.get(species):
                    scarce_flags.pop(species, None)
                    deltas.append(_chronicle(
                        f"{name} are thriving again around {floor.name}.",
                        f"population back to {ratio:.0%} of capacity in {zone.id}",
                        floor.id))

            new_state = {"populations": populations}
            if extinct_since:
                new_state["extinct_since"] = extinct_since
            if scarce_flags:
                new_state["scarce"] = scarce_flags
            deltas.append(Delta(
                kind="entity_state",
                payload={"id": zone.id, "kind": "zone", "def_id": floor.id,
                         "state": new_state},
                location_id=floor.id,
            ))
            deltas.append(Delta(kind="zone_tick", payload={"zone_id": zone.id}))
    return deltas


def _chronicle(headline: str, detail: str, location_id: str) -> Delta:
    return Delta(kind="chronicle",
                 payload={"category": "disaster", "headline": headline,
                          "detail": detail, "visibility": "regional"},
                 location_id=location_id)
