# Project Cardinal

A persistent world simulation engine. Narrative emerges from system interaction —
the narrator only describes state the simulation has already computed and committed.

See [CARDINAL_DESIGN_SPEC.md](CARDINAL_DESIGN_SPEC.md) for the authoritative design.

## Status

**Milestone M2 — Living world** (done):
- [x] NPC utility agents (§7): hourly need decay, schedule-following with
      location resolution (`dist.x` → its city), eat/sleep pressure, goal
      effort accrual, socializing that writes NPC memories
- [x] Quest lifecycle (§13): auto-availability, `npc_fallback` daily rolls,
      expiry firing authored `failure.world_effects` — with or without a player
- [x] Ecology day-tick: populations regrow toward carrying capacity; predator
      pressure crossing the migration threshold is chronicled
- [x] Weather day-tick: seasonal patterns from rules.yaml, storms and season
      turnover enter the chronicle
- [x] Incremental delta application: each tick boundary sees the world state
      the previous one produced, still one atomic transaction per turn

**Milestone M1 — Skeleton** (done):
- [x] Repo layout
- [x] Pydantic schemas for all world-package formats (§4)
- [x] Registry: loads, validates, and cross-reference-checks world packages
- [x] SQLite persistence layer with atomic delta commits (§14)
- [x] World clock with hierarchical tick boundaries (§6)
- [x] Seeded RNG substreams (reproducible saves)
- [x] Minimal 8-step simulation loop (§1.2) with plain narrator
- [x] `cardinal new | play | tick | inspect | validate` CLI
- [x] `worlds/aincrad` starter package (Floor 1 slice)
- [x] `tests/fixtures/testworld` synthetic package + headless tests

## Quick start

```
pip install -e ".[dev]"
cardinal validate worlds/aincrad
cardinal new mysave --world aincrad --seed 42
cardinal play mysave
cardinal tick mysave --days 30
cardinal inspect mysave clock
cardinal inspect mysave chronicle
pytest
```

## Hard rules

- `engine/` never imports from `worlds/` — content reaches the engine only via the registry.
- The engine runs a full simulation tick with the narrator disabled; LLM output is never
  a dependency of state computation.
- Every tunable number lives in the world package's `rules.yaml`, never in engine code.

## Deviations from the spec (documented)

- Added a `resources/` category to world packages (zone resource nodes like
  `res.medicinal_herb` need definitions for reference checking).
- Shops are defined inline inside location districts as `{id: shop.x, ...}` mappings
  (any mapping with an `id:` field self-registers), rather than as bare id strings.
- Starting item lists accept an `_xN` suffix (`item.bread_x2`) meaning quantity, per
  the manifest example in §4.1.

## Genre uplift (beyond the spec's SAO framing)

The combat data model is delivery-agnostic so world packages aren't limited to
melee settings (guns, bows, beams, grenades all express in data):

- `SwordSkill` generalized to **`Technique`** (`tech.*` ids; `swordskill.*` still
  works as a legacy alias) with `delivery: melee | thrown | projectile | beam | area`.
- Items take an optional **`ranged:`** spec — ammo item, magazine, reload time,
  optimal/max range, projectile speed (fast rounds can't be reaction-dodged), spread.
- Zones carry `cover_density` / `visibility_m` for line-of-sight play.
- The manifest names its own `currency:` (col, eddies, berries) and `region_label:`
  (Floor, Island, District).
- Resolution rules per delivery are specified in `engine/systems/combat.py`'s
  docstring and land with M3.
- **Modifiers** (`modifiers/`, `mod.*` ids): one schema for curses, blessings,
  cyberware, tattoos, mutations, diseases, brands. Typed effects (stat, skill
  grant, need rate, disposition-on-sight…), body slots, visibility
  (visible/concealable/hidden), durations, acquisition (surgery/ritual/infliction)
  and removal terms (`removal: null` = a true curse), plus a generic
  `humanity_cost` knob whose meaning each world's rules define. Runtime state in
  the `modifiers` table via `modifier_add`/`modifier_remove` deltas
  (deactivated, never deleted — history survives).
- **Vehicles** (`vehicles/`, `vehicle.*` ids): mounts, wagons, cars, boats,
  ships. Per-terrain `speed_kmh` (absent terrain = impassable), seats/cargo,
  `living: true` for creatures (needs, can die) vs `fuel:` for machines
  (consumption, tank). Runtime instances live in the `entities` table.
  Travel-cost and combat integration land with M3.
