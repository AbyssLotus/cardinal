"""Positional combat resolution (§8), generalized across genres. M3 milestone — stub.

The 1-second-round, continuous-position model is delivery-agnostic. Each
attack resolves through its technique's `delivery` (see schemas.Technique):

  melee      — connects within range_m/arc_deg; defender in `ready` state may
               parry/dodge/block if reaction_ms beats activation+execution.
  thrown     — the weapon instance IS the projectile: it leaves the attacker's
               inventory, lands in the encounter space, and can be recovered
               (or looted). Dodgeable if reaction beats flight time.
  projectile — consumes the weapon's RangedSpec.ammo per shot; magazine and
               reload_s are real action costs. Dodgeable only when
               reaction_ms < distance / projectile_speed_mps * 1000 — bullets
               effectively aren't, arrows at range are.
  beam       — hitscan. Defenders dodge the tell (activation_ms), never the
               shot. Cover is the counterplay.
  area       — resolves against every combatant within range_m of the impact
               point; cover and prone-state reduce damage, reactions don't.

Range bands from RangedSpec: full accuracy inside optimal_range_m, linear
falloff to max_range_m, no connection beyond. spread_deg widens with movement.

Cover & sightlines come from zone data (Zone.cover_density, Zone.visibility_m)
plus per-encounter terrain features; a target in cover is only hittable by
delivery paths with line of sight or `area`.

Invariants (unchanged from §8): no level scaling ever; permadeath per world
rules applies to everyone; all tunables live in rules.yaml `combat:` —
nothing genre-specific is hardcoded here.
"""

from __future__ import annotations

from engine.persistence.store import Delta
from engine.systems import SystemContext


def tick(ctx: SystemContext, granularity: str, day: int, hour: int) -> list[Delta]:
    return []
