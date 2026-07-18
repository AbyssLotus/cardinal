# Project Cardinal — v0.1 Proof of Concept (ARCHIVED)

> **This codebase is frozen.** It is the initial proof of concept that carried Cardinal
> through six milestones to v0.1 and validated the ideas that became the
> [Cardinal Architecture Specification](../../docs/design-spec/README.md). It is preserved
> as evidence, not precedent (Vol. V Ch. 10 §10.3): its scars are cited throughout the
> spec — the headless narrator degradation (Vol. V Ch. 9), golden combat transcripts
> (Vol. V Ch. 4), the income-bleed telemetry catch (Vol. V Ch. 8), the `save_format`
> migrations (Vol. IV Ch. 6), and the `engine/` never imports `worlds/` rule, ancestor of
> Vol. V Ch. 1's dependency law. The engine that replaces it is being built at the
> repository root, against the specification. Do not extend this code.

A persistent world simulation engine. Narrative emerges from system interaction —
the narrator only describes state the simulation has already computed and committed.

The original single-file implementation spec this POC was built against is preserved at
[docs/archive/CARDINAL_DESIGN_SPEC.md](../../docs/archive/CARDINAL_DESIGN_SPEC.md).

## Status

**Milestone M6 — Hardening** (done): **all six spec milestones complete — v0.1.**
- [x] Save-format migrations: `meta.yaml` carries `save_format`; opening an
      older save runs append-only migrations in a transaction (migration 0→1
      upgrades pre-versioned dev saves); saves from a newer engine refuse to
      open with a clear message
- [x] Balance telemetry: `cardinal tick <save> --days N --report` prints NPC
      vitals, chronicle/quest/market/population summaries. The first year-long
      Aincrad run caught a real bug — NPC `income` bled to zero because no
      activity restored it; working a scheduled job now earns (`npc.income_per_work_hour`)
- [x] Invariant tests (§17): golden combat (same seed ⇒ byte-identical fight
      transcript and end state), **no level scaling** (a slime hits a level-40
      player with the same numbers as a level-1), **col conservation** (every
      currency movement reconciles against buys/sells/rewards)
- [x] 365-day Aincrad run: completes cleanly, sustainable NPC vitals,
      populations stable, chronicle non-trivial

**Milestone M5 — Content & narrator** (done):
- [x] LLM narrator (`--narrator llm`): Anthropic-backed prose rendering with
      the GM rules encoded in the system prompt (cached), perception-only
      context, refusal handling — and guaranteed headless degradation: missing
      package, missing credentials, or any API failure falls back to the
      deterministic plain narrator, permanently for the session
- [x] Floor 1 at full population: 15 named NPCs across the Town of Beginnings,
      Horunka, Tolbana, and the labyrinth approaches — every one with a
      schedule or an active goal (Kell the wagoneer runs a real daily freight
      route between towns)
- [x] Second quest (Wolves at the Treeline) with `col` rewards, tying the
      dire-wolf ecology to village life
- [x] v0.1 definition-of-done test: 30 playerless days on Floor 1 produce
      expired/resolved quests, a non-trivial chronicle, NPC movement, and
      20+ NPC memories

## Backlog (known gaps, deliberately deferred)

- **Factions as actors**: `factions.py` tick is a stub — relations, treasury,
  and agendas are data with no collective behavior yet (no turf shifts, no
  guild wars, no spending). The Maelstrom design exercise showed this is the
  biggest missing layer.
- **Hostility on sight**: encounters only start when the player hunts; NPCs/
  monsters never initiate from disposition or faction hatred.
- Dual presence (meat body attackable mid-netrun), NPC-piloted vehicles,
  magazine/reload cycles, per-good stock depth, 2D combat positions,
  aggro/threat model, buying NPC knowledge (`talk` shows prices, no purchase
  verb yet).

**Milestone M4 — Economy & quests** (done):
- [x] Markets (§11): daily repricing `base × (demand/supply)^elasticity`,
      mean-reverting indices (the abstract merchant restock), trades as
      supply/demand inputs — dump loot and watch the price fall tomorrow
- [x] `shop / buy / sell` with the world's own currency; vehicles are stocked
      goods (buy a mule, mount it)
- [x] Quest completion: `talk <npc>` reveals needs, `give <item>` fulfills
      requirements — rewards granted, authored success outcome chronicled,
      permanent NPC gratitude memory, reputation with the NPC and settlement
- [x] Rumor propagation (§5.2): daily salience decay; co-located NPCs pass
      strong memories on as rumors with degraded certainty — your deeds
      reach people you never met
- Note: rare goods are purchasable wherever their category is stocked
      (emergent arbitrage); per-good stock depth is an M6 refinement

**Milestone M3 — Player systems** (done):
- [x] Interactive combat (§8): 1-second rounds, delivery-aware (melee closes
      distance, projectile consumes ammo, thrown weapons leave your hand and
      are recovered — or not — after), the post-skill freeze, cooldowns,
      stamina, parry/dodge/block stances with the reaction-time gate, monster
      ai_script flavors (charge, pack flank, spit range, boss phase)
- [x] Skills & XP (§9): proficiency growth per validated use (diminished vs
      low-level prey), technique unlocks derived from skill files, data-driven
      XP curve with level-ups
- [x] Item runtime (§10 partial): durability wear and shattering, equip slots,
      ammo consumption, loot drops with authored chances
- [x] Modifier effects apply in combat (stat_add/mult, action_lock/paralysis)
- [x] Permadeath: HP 0 deletes the character from the story — chronicled,
      and the dead take no actions
- [x] Consequences: kills reduce zone populations; boss kills enter the
      chronicle as `boss_defeat`
- Deviations: encounter positions are 1D distances (2D plane is an M6
      refinement); encounter state persists in the entities table so a fight
      survives a process restart

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

## Genre probes (proof, not promise)

`tests/fixtures/probe_cyberpunk`, `probe_destiny`, and `probe_wow` are minimal
world packages that run three foreign power systems on the unmodified engine,
verified by `tests/test_probes.py`:

- **Night City**: firearms with real ammo (bare `attack` shoots when your
  weapon is ranged), 400 m/s rounds nobody reaction-dodges, permadeath.
- **Cosmodrome**: a `light` pool from rules.yaml fuels a Solar Grenade
  (`delivery: area`, hits the whole dreg pack) and a hitscan Golden Gun
  (`delivery: beam`); `death.permadeath: false` + `respawn_location` means
  Guardians resurrect at the Tower minus a glimmer tithe.
- **Elwynn**: mana-limited Fireball casting (`resource_cost` beats cooldowns
  as the limiter), graveyard respawns, murloc packs that flee at low HP.

**Netrunning** works through the interaction system + net-architecture-as-region
pattern (see `probe_cyberpunk`): a jack-in terminal (skill-checked `hack`,
requires your cyberdeck) teleports your presence into a subnet floor where ICE
are monsters, quickhacks are RAM-costed beam techniques, a state-gated data
vault yields loot and a secret chronicle entry, and an exit node jacks you out.

**Devices & interactions** (`devices/`, `device.*`): any world object with
verbs — doors, terminals, levers, shrines. Skill-vs-difficulty checks, tool
requirements (`requires_item`), state gates (`requires_state`), and authored
outcomes (state change, message, teleport / give_item / npc_state / chronicle
effects). One primitive covers hacking, lockpicking, and quest machinery.

**Vehicles at runtime**: `starting_player.vehicles` seeds owned instances;
`mount/dismount`; riding cuts travel time to the vehicle's per-terrain speed,
moves the vehicle with you, and burns fuel for machines. In combat a mounted
vehicle absorbs hits (armor first, wreck-and-thrown-clear on destruction) and
enables `attack ram` (damage scales with top speed).

Known simplifications: magazine/reload cycles aren't yet simulated (ammo is
per-shot), no aggro/threat model, no NPC-piloted vehicles yet, and dual
presence (your meat body attackable mid-netrun) is deferred until a cyberpunk
package is a real target.

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
