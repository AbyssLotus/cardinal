# Sweep: What the Engine Has, What's Broken, What's Missing

*Sweep date: 2026-10-02. Measured against the project's goal — a playable, text-based world
that responds to its own actions and has agency — on branch `feat/overlapping-regions` (with
audit Phases 1–4 uncommitted). Findings marked **verified** were reproduced by running the
engine (a probe harness in the session scratchpad, not committed); the rest come from reading
the code and the specification.*

---

## 1. Bottom line

**The engine is now a good model of *where things are*, and almost nothing else.**

Physical Reality is substantial: about 4,600 lines, 10 systems, and 46 fact types covering
space, terrain, weather, sight, and travel. One Living Systems system tracks body heat.
Everything a player would touch is absent:
- no way to type a command
- nothing that describes the world
- no names for anything
- no way to pick anything up
- no saving
- no one with a mind of their own

The other eight domains, three of the four services, and all three front ends are empty stubs of
36 lines or fewer.

The archived prototype (v0.1) was **playable**: it had a parser, narrators, NPCs with schedules
and memories, combat, markets, quests, and saves. The rebuild has the better foundation, but on
features it is behind the thing it replaced.

There are also **ten defects in what exists**, all verified. The serious ones:
- world files are never validated
- the movement ownership rule (Ruling 13) is not enforced
- terrain broke two height queries
- indoors gets full sun and outdoor weather

---

## 2. What exists today

| Layer | What runs | State |
|---|---|---|
| Kernel (3,030 lines) | Tick pipeline, fact store, determinism hash, seeded RNG, simulated-time clock, integer math (sqrt, sin/cos, angles), spatial index | Solid. Missing: entity creation, a dependency-ordered scheduler, parallelism |
| Physical (4,644 lines, 10 systems) | Day/night temperature, weather (temperature, humidity, pressure), sunlight, wind, portal danger, closing finished motion legs, gravity, travel | Rich, with the defects in §3 |
| Living (369 lines, 1 system) | Body heat drifting between a set point and the air | A single number with no consequence |
| Resources, Economy, Society, Culture, Knowledge, Conflict, Institutions, Ecology | — | 36-line stubs |
| Packages (1,957 lines) | World-file parser and loader | Works; **no validation** |
| Persistence, Replay, Observe | — | Stubs of 8 lines or fewer |
| CLI, Narrator, Tools | — | Stubs: the CLI prints one sentence and exits |

The queries a consumer can ask today:
- **Where:** `position_in`, `relative_position`, `distance`, `height_above_ground`
- **Which way:** `heading_in`, `bearing_to`, `relative_bearing`
- **What's around:** `within`, `nearest`, `contents`, `in_box`
- **Regions:** `regions_of`, `is_within`, `members_of`, `overlaps`
- **Routes:** `route`, `can_reach`
- **Motion:** `velocity`, `speed`, `course`, `arrival`
- **Sight:** `line_of_sight`, `visible`
- **Ground:** `ground`, `slope_percent`, `support`

That's a strong spatial vocabulary. Nothing yet turns it into an experience.

---

## 3. Defects in what exists (all verified)

| # | Defect | Observed | Severity |
|---|---|---|---|
| D1 | **World files are never validated** (spec Vol. IV Ch. 7) | A duplicated `[positions]` line gave entity 10 *two* positions. A portal into nonexistent region 999 and an entity inside nonexistent container 777 both loaded. A containment cycle (city in bedroom in house in city) loaded. | High: every later query silently answers from corrupt data |
| D2 | **Ruling 13 is not enforced** | A non-physical system set a body's container and position directly: the tick committed and the body teleported. Nothing checks who proposes a position, containment, or door state. | High: the law that makes walls real is honour-system only |
| D3 | **Heights ignore terrain** | The cottage's ground-level front door is rated danger 4500 (a 3 m fall) because it stands on a 3 m hill. `height_above_ground` reports height above the world datum, not above the ground beneath. | Medium |
| D4 | **Indoors is outdoors** | At noon the walled kitchen gets full sunlight (10000), same as the hillside. Every room draws its own independent outdoor weather. No heat or light passes through doors or windows, and "enclosed" doesn't imply shelter. | Medium-high: a text world would describe sunshine and storms indoors |
| D5 | **A travel intent with no speed is silently ignored** | The body never moves, and it is never marked blocked or rejected, so the decider can't tell. | Low-medium |
| D6 | **Body heat reads only the immediate container** | A rider in a cart crossing a −10 °C region stays at 37 °C all day; so does anything carried or in a bag. | Medium |
| D7 | **Body heat has no limits or consequences** | Someone walking beside the cart cools to a 5.7 °C *body temperature* and nothing happens. There is no clothing, no shivering, no harm, no death. | Medium (really a gap: see M10) |
| D8 | **Planning paths is slow in crowds** | 1,000 walkers planning at once took 1.2 s for that tick, 0.6 ms per tick while walking. Each plan rebuilds the room's obstacle grid and rescans every object in the room. | Medium |
| D9 | **Proximity cost grows with the number of rooms** | "Who is within 3 m" took 1 µs in a 10-room world and 121 µs in a 1,000-room town: it visits every room in the hierarchy. | Medium for a town-sized world |
| D10 | **Walkers float or sink slightly mid-leg on slopes** | 7 cm off the hillside mid-stride: legs are straight lines between waypoints and the ground is curved. | Low |

One check passed: the cottage world replays bit-identically from the same seed over 200 ticks.
Determinism holds across the new systems.

### Status after the fixes (2026-10)

Every fix is held by the Ashford reference suite (`tests/reference/`), which runs the whole
integration suite in one fictional city instead of a world per feature.

| # | Status | Held by |
|---|---|---|
| D1 | Fixed. The loader validates first and refuses with every problem named: duplicates, dangling references, undeclared materials, cycles, `closed` on a non-portal (Vol. IV Ch. 7, layers 2–4). | `engine.rs` |
| D2 | Fixed (Amendment A-5). Only Physical's own systems may write position, containment, motion, facing, door state, and their reports; the kernel refuses anyone else's proposal and duplicate system ids. | `movement.rs` (the teleporter is refused) |
| D3 | Fixed (A-5). Heights are measured from the ground beneath; an opening's danger is the drop where one lands. | `ground.rs` |
| D4 | Fixed (A-5). Walled rooms are sheltered: no sky weather, daylight through openings by area, air that follows the outside with a lag. | `climate.rs` |
| D5 | Fixed. A speedless or immobile traveller is reported blocked. | `movement.rs` |
| D6 | Fixed. Body heat reads the nearest enclosing place with air, through carts and bags. | `living.rs` (the rider) |
| D7 | Open, by design: it needs the health model (M10). | — |
| D8 | Improved. 1,000 walkers in an open yard: planning tick 7 ms (was 1.2 s), walking 0.6 ms/tick. With 200 crates in the yard: planning tick 164 ms, walking 18 ms/tick — each leg around an obstacle costs about 0.2 ms to start. Shared, incrementally updated grids (M17) remain the full fix. | measured, release build |
| D9 | Improved. "Who is within 3 m": 2 µs among 10 rooms, 27 µs among 1,000 (was 121 µs). Far rooms' contents are skipped, but the search still checks each sibling room's bounds, so cost still grows with room count. Room bounds in a spatial index of their own (M18) remain the full fix. | measured, release build |
| D10 | Fixed. Legs over terrain are kept to half a sample spacing, short enough that a walker never sinks more than 2 cm into the hillside mid-stride. | `movement.rs` (the villager) |

M13 (decision systems) and M14 (the information layer) have a first slice: see
`docs/audits/agents.md`. Perception (M3) has sight by daylight; hearing and the rest are to come.

Ashford found three more defects as it was built, all fixed:

- **A slope was mistaken for a ledge.** Walking down the Hill, a walker "fell" the height the
  slope dropped over one leg. A ledge is now only a drop off something solid.
- **Stairs up rated as a fall.** Portal danger was measured from the outdoor ground, so the
  top of a staircase was 3 m above it. Danger is now the drop at the landing spot.
- **Timber held heat better than stone** (Amendment A-6). Thermal mass was specific heat
  per kilogram; it is now heat stored per cubic metre, so the granite hall lags the afternoon
  more than the timber kitchen.

---

## 4. What's missing, by what it blocks

### Tier 1 — Can't play without it

| # | Missing | Why it matters | Spec status |
|---|---|---|---|
| M1 | **A play loop and input pipeline.** Command → parse to actions (INTERPRET) → check them against the world (VALIDATE) → spend their time (COST, ADVANCE) → tick → resolve → narrate | It *is* the game. Today nothing turns "go to the loft" into a travel intent or decides how far the world advances per command | **Spec gap**: Vol. V cites "Volume II's INTERPRET/VALIDATE" twice, but no current Vol. II chapter defines them. Only the archived prototype spec does. Needs an amendment first |
| M2 | **A player** — an entity bound to a perspective | Vol. I Law 10 says the player is an ordinary entity; nothing yet says *which* entity you are | Vol. IV Ch. 5 (scenario binding): specified, unbuilt |
| M3 | **Perception**: what someone notices — sight with light and darkness, hearing, smell, distance | "You see…", "you hear footsteps above" | Sight is geometric only. Vol. II Ch. 4 Observation is unbuilt |
| M4 | **A narrator**: a deterministic text renderer of what the player perceives and what just happened | Without it the world is numbers | Vol. V Ch. 9: specified, unbuilt (the prototype had one) |
| M5 | **Content with names and kinds.** Declarations ("a timber door", "a red deer") with namespaced ids, display names, descriptions, and a vocabulary; entities created from kinds | Every entity today is a bare number; the narrator would have nothing to call anything | Vol. IV Ch. 1 and Ch. 3: specified, unbuilt |
| M6 | **Interacting with things**: take, drop, put, give, open, close, lock, unlock — intents validated for reach, with containers that shut, lock, and hide their contents, plus capacity and weight | The core verbs of a text game. Today an item can only be moved by teleporting it (D2) | Needs a ruling like Ruling 13 for manipulation |
| M7 | **Creating and destroying things at runtime**: a deterministic id allocator, and retiring an entity as a whole | A dropped coin, a lit fire, a birth, a broken pot | Vol. II identity: specified, unbuilt (audit P8) |
| M8 | **Saving and loading** | A game you can't keep | Vol. V Ch. 7: specified, unbuilt |

### Tier 2 — A world that responds to its own actions

| # | Missing | Why it matters |
|---|---|---|
| M9 | **Events with content.** The chronicle records *that* a fact changed and why (a cause name), never *to what* or *who else was involved* | Narration, memory, and "what happened here" all need "the cat fell 30 cm through the window", not "fact changed: fall_height" |
| M10 | **Living consequences**: health and injury, needs (hunger, thirst, fatigue, warmth), death, capabilities (top speed, carrying capacity, senses), clothing and insulation | Falls record a height that nothing reads; cold has no effect; a decider can set any travel speed up to 10 km/s |
| M11 | **Material consequences**: fire (flammability is stored, never used), breakage (hardness unused), weight (density unused) | Materials exist but do nothing except damp temperature swings |
| M12 | **A fuller environment**: light sources (a lamp, a hearth), shelter and heat flow between indoors and outdoors (D4), seasons and a calendar, real precipitation, cloud, and fog (the "Precipitation" system only moves humidity), water (rivers, wetness, swimming), sound | The texture of a place: "rain drums on the roof; the hearth keeps the cold out" |

### Tier 3 — Agency

| # | Missing | Why it matters |
|---|---|---|
| M13 | **Decision systems for NPCs**: goals, needs-driven choices, daily schedules, reacting to what they perceive | "Has agency." The prototype had 15 NPCs with schedules or goals; the rebuild has none |
| M14 | **The information layer**: knowledge, memory, and belief per mind, and communication (talk, rumour) | The spec's rule that minds act on beliefs, not on truth (Vol. II Ch. 4). Without it NPCs are omniscient or inert |
| M15 | **The other eight domains**: resources, economy, society, culture, knowledge, conflict, institutions, ecology | Trade, families, factions, fights, laws, wildlife: everything that makes a world more than a map |

### Tier 4 — Engine work that comes due as worlds grow

| # | Missing |
|---|---|
| M16 | **Proposer authority in the kernel**: a way for an owning domain to accept some writes only from its own systems (fixes D2 at the root) |
| M17 | **Shared navigation grids per region**, cached and updated as obstacles move (fixes D8), plus collision between moving bodies, and routing that weighs danger as well as distance |
| M18 | **Room bounds in the proximity search**, so far-off rooms are skipped (fixes D9) |
| M19 | **The rest of the scale path** (the earlier audit's Phase 5): persistence, the hybrid hot store, residency for distant regions, parallel evaluation, a dependency-ordered scheduler |
| M20 | **Observability**: an inspection CLI or tools ("why is the courier blocked?") — Vol. I requires the CLI to support inspection without narration |

---

## 5. Parity with the archived prototype

What v0.1 could do that the rebuild can't yet:

| Prototype had | Rebuild |
|---|---|
| Natural-language and command parser | — (M1) |
| Plain narrator, plus an LLM narrator with guaranteed fallback | — (M4) |
| Player perception (perception-only narration context) | Geometric sight only (M3) |
| 15 named NPCs with schedules, goals, and memories | — (M13, M14) |
| Combat with timing, parries, damage, death | — (M10, M15) |
| Skills and proficiency growth | — (M15) |
| Items, inventory, vehicles | Positions only (M6) |
| Markets with daily repricing; currency conservation | — (M15) |
| Quests | — |
| Factions (data only) | — |
| Saves with format migrations | — (M8) |
| Telemetry reports; a 365-day stability run | Determinism hash only (M20) |
| **Did not have:** real 3D space, terrain, line of sight, rotating frames, overlapping regions, simulated-time units, a spatial index, deterministic travel through openings | **Has all of these** |

---

## 6. Recommended order

Each step starts with its spec amendment, as before.

1. **Fix the defects** (D1–D6, D10). All small, and two are integrity issues:
   - a validation pass in the loader: duplicates, dangling references, cycles
   - a manipulation-and-movement authority check, so only Physical's own systems may move a body
   - terrain-aware heights
   - shelter implied by enclosure
   - speedless intents reported as blocked
   - body heat reading the nearest container that has a temperature
2. **Content with names and kinds (M5).** Kinds in data packs, namespaced ids, display names and
   descriptions, a vocabulary. Without this the narrator has nothing to say.
3. **Entity lifecycle and interaction (M7, M6).** An id allocator; take, drop, put, open, close,
   lock and unlock as intents validated for reach; containers that shut and hide their contents.
4. **Player, perception, and events (M2, M3, M9).** A bound perspective; sight that respects
   light; hearing; events that record what happened and to whom.
5. **The play loop (M1, M4, M8).** A CLI with a command parser, validation, time advance per
   command, a deterministic narrator, then save and load.

At that point the world is playable. Agency follows:
- Living needs and health (M10)
- NPC decisions with schedules and goals (M13)
- the information layer and talk (M14)
- then the domains a given world needs (M15)
