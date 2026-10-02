# Audit: Can Cardinal Simulate a Full 3D World?

*Audit date: 2026-10-01. Scope: the five-volume specification, the roadmap (Vol. V Ch. 10
§10.4 and the root README), and the code on branch `feat/overlapping-regions`. All
measurements were taken on an Apple M5 Max, single-threaded, release build.*

---

## 1. Bottom line

**The architecture can carry a 3D world, but neither the current plan nor the current code
is heading toward one.**

The spec makes the right foundational bets: queries over storage, integer coordinate frames
composed through containment, determinism, a residency ladder for scale, and lockstep
networking. Most game engines get at least two of those wrong.

Where the plan falls short, it's because it never commits to anything concrete. It names the
spatial primitives a 3D world needs ("Intersects", "Above", "Visible", "Occupied", "Reachable")
but defines none of the data or machinery behind them. It defines simulation time with no
physical unit. It assigns travel to Physical Reality and then never specifies travel.

The last several pull requests built out per-region weather and materials: wind, exposure,
thermal mass. That's good work, but none of it moves the engine closer to 3D.

Three findings decide everything else:

1. **The environment breaks at the tick rate a 3D world needs.** Every world runs at 24 ticks
   per simulated day, so one tick is an hour and a walking person covers 5 km per tick. A
   house-scale world needs ticks of about a second or less. At one tick per minute, a
   half-sheltered region loses its day/night swing entirely. At one tick per second, random
   weather drift over a single day grows from about 1.6 °C to about 136 °C (§4.1). Even at
   today's hourly ticks the climate isn't stable: after one simulated year, regions that all
   started at 15 °C range from −55 °C to +74 °C.
2. **There is no spatial index.** "Who is within 10 m of me?" is a full scan. For 1,000 agents
   it takes 375 ms per tick; for 3,000 agents it takes 3.8 s. The cost grows with the square of
   the agent count (§4.2).
3. **Motion is written as per-tick position facts.** Every mover writes two or three facts and
   two or three chronicle entries every tick. At 10 ticks per second with 50,000 movers that's
   about 48 MB of chronicle per second, held in memory with no persistence to drain it (§4.3).

None of these needs the architecture to change. All three need **spec amendments first** (the
project's own law, Vol. V Ch. 10 §10.3), then a re-sequenced roadmap. Section 8 has that plan.

---

## 2. What "a full 3D world" has to mean here

The spec has already ruled on part of this, and the plan should respect it:

- **Cardinal does not render.** Rendering consumes Physical Reality; it never owns it (Vol. III
  Ch. 1 §1.1, Designer Note; Vol. I Ch. 1 §11, Non-Goals). A 3D *picture* means a separate client
  — an off-the-shelf engine such as Bevy or Godot — in the frontends layer, drawing from a
  perspective-filtered stream.
- **Cardinal is not a rigid-body physics engine.** "Physics is a consumer" (§1.1). Stacking
  crates and ragdolls belong on the client, as presentation, never as authoritative state.
- **Reality is discrete; presentation may be continuous** (Vol. II Ch. 2). The client
  interpolates smoothly between authoritative ticks.

Inside those rulings, "a full 3D world" means the **authoritative simulation** can answer the
following, for many actors, at an interactive rate:

| Capability | Example question | Today |
|---|---|---|
| Where, in 3D, at fine time resolution | Where is the courier *right now*, to the centimetre? | Partial: positions exist, but at 1-hour ticks |
| Which way it faces | Is the guard facing the door? | **Missing**: no orientation |
| How big, what shape | Does the wardrobe fit through the door? | **Missing**: no extents |
| What the ground is | How high is the ground here? Is it too steep? | **Missing**: one elevation number per region |
| What's near | Which wolves are within 30 m? | **Missing**: full scan only |
| What's visible | Can the archer see the window? | **Missing** |
| What blocks movement | Is the door shut? Is that wall solid? | **Missing**: §1.11 unbuilt |
| How to get there | Route from the yard to the attic? | Partial: room-to-room portal routing exists; no navigation inside a room |
| Moving and colliding | Two people step into the same doorway | **Missing**: no movement system |
| Falling and support | What happens if you walk off the roof? | **Missing** |
| Conditions at a point | How cold is it 40 m down the shaft? | Partial: one value per region |
| Creating things | An arrow is loosed; a child is born | **Missing**: entity ids are authored only |
| Scale | A city of thousands; a continent beyond it | Linear today; no residency ladder, no parallelism |

### The decision you need to make first

**What is the authoritative tick length?** Every gap above is sized by it.

- *Recommendation:* **100 ms per tick (10 Hz) for the authoritative simulation, with the client
  rendering at 60 fps by interpolating.** This is the usual design for server-authoritative
  lockstep games. It matches Vol. V Ch. 5 §5.3, and it's fine-grained enough for walking,
  climbing, and line of sight.
- Weather, economy, and politics then run on cadences measured in *simulated time* — every
  minute, hour, or day — not every tick (Vol. II Ch. 2, Scheduling).
- If you want a strategy-scale sim instead (ticks of a second or longer, no action-game
  movement), Phases 1–3 still apply. Phase 4 shrinks, and the numbers in §4 become easy.

---

## 3. What the plan already gets right — keep these

1. **Representation independence** (Vol. III Ch. 1 §1.14). Consumers ask questions; storage
   can be a grid, an octree, or voxels. That's exactly the seam a 3D engine needs. Don't break
   it by letting domains read coordinates directly where a question would do.
2. **Integer local frames composed through containment** (`physical::space`). Positions are
   i64 centimetres relative to the immediate container. That avoids the float-precision
   "far lands" problem games hit at large coordinates, and it makes moving containers (ships,
   wagons, lifts) natural, because their contents move with them for free.
3. **Determinism as law** (Vol. V Ch. 4) and **lockstep networking** (Vol. V Ch. 5 §5.3).
   Multiplayer sends inputs, not world state. A 3D world with thousands of movers synchronizes
   in kilobytes.
4. **The residency ladder** (Vol. V Ch. 5 §5.2) and **positional lazy generation** (Vol. IV
   Ch. 4 §4.3). Cold → aggregate → detailed, with every region still advancing. This is the
   correct strategy for a world bigger than memory.
5. **The hybrid store** (Vol. V Ch. 2 §2.2): hot ECS-style columns behind one contract. That's
   the right home for positions and velocities.
6. **Connectivity distinct from adjacency** (portals), **overlapping regions**, and
   **exposure**. These are the topological half of 3D, and they're already built.
7. **Ownership precedent for travel.** Appendix A Ruling 4 ("the travel is Physical Reality")
   and Ruling 9 ("Conflict proposes, owners apply") already give the pattern movement needs:
   a decision proposes, Physical applies under its own constraints.

---

## 4. Measured: what the engine does today

The benchmark source was a throwaway crate in the session's scratchpad, not committed (see
Appendix). All numbers are single-threaded release builds.

### 4.1 The environment depends on tick rate

One simulated day per row. The "swing" columns show the diurnal system alone; the last column
shows weather noise alone.

| Ticks per day | Tick length | Day/night swing, open region | Day/night swing, half-sheltered | Weather drift over one day |
|---:|---|---:|---:|---:|
| 24 | 1 hour | 4.00 °C | 1.96 °C | 1.59 °C |
| 1,440 | 1 minute | 4.00 °C | **0.00 °C** | 7.93 °C |
| 86,400 | 1 second | 4.00 °C | **0.00 °C** | **135.68 °C** |

There are two causes, both in `domains/physical/src/systems.rs`:

- **Integer rounding of attenuated deltas.** `DiurnalCycle` applies the *change* in the wave each
  tick, then scales it by exposure and thermal mass. At fine tick rates the per-tick change is
  ±1 centi-degree, and `1 × 50% = 0` in integer arithmetic. Every sheltered or massive region
  silently stops having a day.
- **Rules expressed per tick, not per unit time.** `weather_max_swing_centi_c`,
  `humidity_swing`, `pressure_weather_swing`, and the settle/drying divisors are all per tick.
  Temperature noise is an unbounded random walk, so its spread grows with the square root of the
  tick count. Raising the tick rate rewrites the climate.

The missing pull back toward normal is a defect at the current tick rate too. One simulated
year at 24 ticks per day, weather noise only, 500 regions all starting at 15.00 °C:

| Spread (standard deviation) | Coldest region | Hottest region |
|---:|---:|---:|
| 21.3 °C | −54.9 °C | +73.6 °C |

That matches the random-walk prediction: a ±0.40 °C uniform step has a standard deviation of
0.23 °C, and 0.23 × √8,760 ticks ≈ 21.6 °C.

The root cause is in the spec. Vol. II Ch. 2 defines time as `Year → Day → Tick` with no
physical unit, so no rule can be written as a rate.

### 4.2 No spatial index

Every agent asks "who is within 10 m?" using the engine's own `space::distance`:

| Agents | One all-agents pass |
|---:|---:|
| 1,000 | 375 ms |
| 3,000 | 3,795 ms |
| 10,000 | ~40 s (extrapolated; quadratic) |

At a 10 Hz tick (100 ms budget), even 1,000 agents is 4× over budget before they do anything
else.

`regions::members_of` builds its reverse index on every call: 0.6 ms at 10k entities, 85 ms at
1M. It's fine as a one-off question, but quadratic if every agent asks every tick.

Vol. V Ch. 2 invariant 8 already says spatial queries are part of the store contract. The
contract (`CommittedView`) currently offers only `read`, `read_all`, and `entities_with`.

### 4.3 Throughput and chronicle volume

| Workload | Cost per tick | Chronicle entries per tick |
|---|---:|---:|
| Weather, 1k regions | 5.8 ms | 6,666 |
| Weather, 10k regions | 37.9 ms | 66,666 |
| Weather, 100k regions | 405 ms | 666,666 |
| Movement (random walk on 2 axes), 1k movers | 0.9 ms | 2,000 |
| Movement, 10k movers | 9.5 ms | 20,000 |
| Movement, 100k movers | 110 ms | 200,000 |

What that means:

- **Scaling is linear and the constants are fine** — about 4 µs per region and 1.1 µs per mover
  per tick. The BTreeMap store isn't the immediate problem.
- **Every region runs every system every tick.** All eight systems use `Cadence::EveryTick`, and
  there's no residency ladder. At 100k regions, weather alone takes 4× a 10 Hz budget.
- **The chronicle is an unbounded in-memory `Vec`.** Each entry is 48 bytes, and the
  persistence crate is an 8-line stub. Per-tick position writes make it the binding constraint:
  50k movers at 10 Hz is about 48 MB/s, or roughly 170 GB per simulated hour.
- **One core of 18 is used.** Parallel evaluation is specified (Vol. V Ch. 5 §5.1) but
  deferred.

---

## 5. Gaps in the plan (specification)

Each gap lists what the spec says, why a 3D world needs more, and the amendment to make
*before* code (Vol. V Ch. 10 §10.3).

### P1. Time has no physical unit — *amend Vol. II Ch. 2 and Vol. V Ch. 3 §3.2*

- **Says:** time is `Year → Day → Tick`; cadences are "every tick, hourly, daily".
- **Needs:** a world rule `sim_seconds_per_tick`. Every rate is expressed per simulated second
  (wind in cm/s, noise as variance per second, relaxation as a time constant). Cadences are
  declared in simulated time and converted to ticks by the scheduler.
- **New invariant:** *tick length is a resolution choice, not a rule.* Changing it within a
  world's declared range must not change any rate. The §4.1 table becomes a conformance test.

### P2. No extent, shape, or orientation — *amend Vol. III Ch. 1 §1.6 and §1.11*

- **Says:** primitives include Intersects, Overlaps, Above, Occupied — but no fact model behind
  them. The code comment in `schema.rs` defers orientation to "a later fact".
- **Needs:**
  - An **extent** fact per physical entity: an axis-aligned half-size box at minimum, plus an
    optional reference to a package collision shape.
  - An **orientation** fact in fixed point — yaw/pitch/roll, or an integer quaternion — and
    frame transforms that apply it. Today frames are axis-aligned only.
  - **Deterministic fixed-point trigonometry** (lookup table or CORDIC) to rotate frames. Floats
    stay out of committed state (Vol. V Ch. 4).

### P3. No terrain contract — *amend Vol. III Ch. 1 §1.4 / §1.10 and Vol. IV Ch. 4 §4.2*

- **Says:** generation layer 1 makes "terrain, elevation, water"; fields may be "grids, voxels,
  graphs". Nothing chooses one.
- **Needs:** a terrain contract with these queries:
  - `ground_height(point)`
  - `surface_normal / slope(point)`
  - `surface_material(point)`
  - `solid(point)` for caves and overhangs

  Reference implementation:
  - **Chunked fixed-point heightfield tiles**, generated positionally from the seed, so they're
    lazy and stable (§4.3).
  - A **sparse volumetric overlay** for caves, overhangs, and Pelagia's water column. Pelagia is
    the spec's own "three-dimensional space in earnest" world (Vol. IV Ch. 8 §8.3).
  - Named **features with stable identity**, as §1.14 requires: the pass, the river.

### P4. Fields are per region, not sampled — *amend §1.10*

- **Says:** fields may be continuous; consumers must be able to query them.
- **Needs:** `sample(field, point)` with defined interpolation between region samples, and
  **vertical profiles** — temperature lapse with height, light and pressure with depth.
  Pressure already falls with elevation. Temperature, light, and humidity don't.

### P5. Spatial queries are named but not specified — *amend Vol. V Ch. 2 §2.1 and §2.4*

- **Says:** invariant 8, "spatial queries are contract surface".
- **Needs:** these queries on the store contract:
  - `within(point, radius)`
  - `in_box(bounds)`
  - `nearest(point, k)`
  - `raycast(from, to) → first hit`
  - `line_of_sight(a, b)`

  Plus a **conformance rule**: the indexed answer must equal the brute-force answer, in the same
  order. That keeps the index a cache, never truth (the "Cache With Opinions" anti-pattern,
  §2.6).

### P6. Movement is assigned but unspecified — *add Appendix A Ruling 13 and amend §1.11*

- **Says:** Ruling 4, "the travel is Physical Reality"; §1.11 lists blocking, closed doors, and
  solid-object exclusion.
- **Needs:**
  - **Ruling 13 — Travel: deciders propose intent, Physical applies motion.** A decision system
    proposes a heading/speed or a waypoint. Physical's composition turns it into motion under
    constraints. This is the Ruling 9 pattern; no other domain writes position.
  - **Motion as kinematic segments, not per-tick positions:** `(anchor position, velocity,
    since_tick)`. Position at any tick is derived exactly. A fact is written only when velocity
    changes. This collapses the chronicle problem in §4.3, gives the client exact interpolation,
    and lets "where will it be in 3 s" be a query.
  - **Constraint composition at Validate:**
    - passability: slope, solid extents, and portal **open/closed** state (a new fact)
    - **deterministic occupancy resolution** when two movers claim one space
    - **support and falling**: no support means falling, which is kinematic gravity, not
      rigid-body physics
  - **Linked portal faces.** An opening is one entity with two faces. The building test caught
    this: a cat that came in the window was placed at the front door until the test matched
    faces by distance.
  - **Navigation hierarchy:** the portal graph (built) for room-to-room, plus an in-region
    navmesh or grid derived from terrain and extents. Routing weighs cost: distance,
    `PORTAL_DANGER`, slope.

### P7. Perception has no geometry — *amend Vol. II Ch. 4 (Observation)*

- **Says:** observation is constrained by vision, lighting, distance, and obstruction.
- **Needs:** observation is built from P5's `line_of_sight` and `within`, P4's light sampling,
  and P2's orientation (field of view). The networking chapter depends on this too: a client
  "may receive only what its bound perspective could observe" (Vol. V Ch. 5 §5.3). You can't
  enforce that without visibility.

### P8. Entities can't be created at runtime — *amend Vol. II (identity) and Vol. V Ch. 2 §2.1*

- **Says:** ids are permanent and never reused.
- **Has:** only `EntityId::from_raw`, used by authored packages.
- **Needs:** a deterministic allocator for arrows, births, construction, and debris. For
  example, ids derived from `(system, tick, scope, n)`, or a committed counter fact allocated
  during resolution. Without it nothing new can enter the world.

### P9. No presentation stream for a 3D client — *amend Vol. V Ch. 5 §5.3 and Ch. 6*

- **Needs:** a perspective-filtered spatial feed per client: `(entity, motion segment,
  orientation, extent, asset ref)`. Asset references are package content (Vol. IV Ch. 3), never
  engine code. Client interpolation is labelled presentation, as in Ch. 5's "Prediction Bleed"
  anti-pattern.

### What to keep out of the plan

- No rigid-body solver in the authoritative tick.
- No floats in committed state.
- No rendering in the engine.
- No spatial index that a system could observe as truth.

Each of these contradicts a ruling the spec already made for good reasons.

---

## 6. Gaps in execution (code vs. the plan as written)

1. **Roadmap drift.** The README status checklist is entirely unchecked, but the kernel, the
   determinism harness, the package loader, and two domains exist. Meanwhile the recent
   feature work (wind, exposure, portals and portal danger, materials, thermal mass, and now
   overlapping regions) deepened Physical's *region-scalar* and topological features. Under Vol. V Ch. 10 §10.4, "Middle" items
   (the hybrid store, the full scheduler) and persistence come before more breadth.
2. **Persistence doesn't exist.** `services/persistence`, `replay`, and `observe` are stubs of
   8 lines or fewer. No world can run long enough to matter: the chronicle grows in memory and
   nothing snapshots.
3. **Scheduler is a stable sort.** There's no DAG, no dirty-region hints, and no parallelism.
   That's acceptable today, but Phase 5 depends on it.
4. **`Value` has no vector type.** Positions are three separate facts, so each move costs three
   proposals, three resolutions, and three chronicle entries. A fixed-point `Vec3` value — or
   the motion-segment fact from P6 — removes that 3× multiplier.
5. **Eight of ten domains are 36-line stubs.** That's expected at this stage, but it means
   nothing yet *uses* Physical's answers under load. The first real consumer (a decision system
   proposing travel) will expose interface gaps no unit test does.

---

## 7. Defects found during this audit

| Defect | Where | Effect |
|---|---|---|
| Sheltered and massive regions lose their day/night cycle at fine tick rates | `DiurnalCycle` in `domains/physical/src/systems.rs` | Silent: temperature goes flat (§4.1) |
| Temperature noise has no pull back toward normal | `WeatherNoise` in `systems.rs` | At today's settings, a year of weather sends 15 °C regions to between −55 °C and +74 °C (§4.1) |
| Weather rules are written per tick | `WeatherNoise`, `Precipitation`, `PressureSystem` in `systems.rs` | Climate depends on tick length (§4.1) |
| `position_in` can loop forever | `domains/physical/src/space.rs` | Called with an ancestor that isn't on the chain, on a cyclic containment chain, it never returns. `relative_position` is safe; direct callers are not |
| Stairs rate as falls | `PortalDanger` | Fall danger is height above ground, so the top of a staircase 3 m up rates like a 3 m drop unless the world pins it |
| Portal faces aren't linked | schema | An opening's inside and outside are two unrelated portals; nothing says which pairs with which |

---

## 8. Revised execution plan

Each phase starts with its spec amendments and ends with a measurable gate. Nothing in a later
phase starts until the earlier gate is green — under the same two release gates the spec
already imposes: the invariants hold, and the reference worlds stay green.

### Phase 0 — Decide and amend (no code)

- Settle the tick length (§2 recommends 100 ms).
- Write amendments P1–P9 into the volumes, plus Appendix A Ruling 13.
- Update `ROADMAP.md` and the README status to this plan.
- **Gate:** amendments merged.

### Phase 1 — Time that has units

- Add the `sim_seconds_per_tick` world rule and cadences in simulated time.
- Make `DiurnalCycle` *set* the temperature from the wave instead of accumulating
  rounded deltas.
- Express noise and relaxation per simulated second, with mean reversion on temperature, so
  the climate stays bounded over years.
- Move weather to a minute cadence.
- **Gate:** the §4.1 table, re-run at 24, 1,440, and 86,400 ticks per day, shows the same
  diurnal swing in every column, and weather drift within a declared tolerance. A simulated
  decade keeps every region within a declared climate band.

### Phase 2 — Bodies in space

- Add orientation, extent, and fixed-point trigonometry and frame rotation.
- Add motion segments: the position derived from them, with writes only on velocity change.
- Add a deterministic entity allocator.
- Fix `position_in`.
- **Gate:** 100k movers where 5% change velocity per tick, under 10 ms per tick and under
  10k chronicle entries per tick, measured against the movement numbers in §4.3.

### Phase 3 — Asking space questions fast

- Add a store-maintained spatial index (uniform grid per frame first; a loose octree if
  needed) behind `within`, `in_box`, `nearest`, and `raycast`.
- Maintain the containment and membership reverse indexes.
- Add a conformance suite comparing every indexed answer to brute force.
- **Gate:** 10k agents' all-pairs 10 m proximity pass under 10 ms, against about 40 s today.

### Phase 4 — Ground, walls, and sight

- **Ground:** chunked heightfield terrain generated positionally; a sparse volumetric overlay
  (caves, water columns); `sample(field, point)` with vertical profiles.
- **Movement:** Ruling 13 movement through Validate, covering passability, open/closed portals,
  deterministic occupancy, and support and falling. Link portal faces.
- **Navigation:** an in-region navmesh or grid, with cost-aware routing.
- **Perception:** `line_of_sight` feeding observation.
- **Gate:** the building scenario from `domains/physical/tests/building.rs` re-run with real
  movement:
  - the cat climbs in the window and lands under it
  - a shut door blocks the courier
  - two people can't share a doorway
  - someone who steps off the roof falls
  - the archer at the window sees the yard but not the cellar

### Phase 5 — Scale

- The hybrid hot tier for positions and motion.
- Persistence: snapshots plus a chronicle tail, with two-road recovery (Vol. V Ch. 7).
- The residency ladder.
- Parallel evaluation sharded by region.
- **Gate:** budgets set from the chosen tick length — for example, 50k detailed movers plus
  1M aggregate regions at 10 Hz on the reference machine — with the twin-run hash identical
  across 1 and 18 threads (Vol. V Ch. 5 §5.5 invariant 1).

### Phase 6 — A window onto it

- The perspective-filtered presentation stream.
- A minimal 3D debug viewer as a frontend (doubling as the Vol. V Ch. 8 causal debugger).
- Asset references in packages.
- **Gate:** a viewer client shows the farmstead and the house moving at 60 fps from a 10 Hz
  simulation, and it shows nothing the bound perspective couldn't observe.

### First three pull requests

1. **Phase 1 entire.** It's small, it fixes real defects, and every later number depends on it.
2. **Spatial index contract plus a uniform grid plus the brute-force conformance suite** — the
   largest single unlock.
3. **Motion segments, orientation, and extents** — the facts every later phase reads.

---

## Appendix: benchmark method

A standalone crate depended on `kernel` and `physical` by path and drove `run_tick` directly.

- **Weather:** N regions in a chain topology; all seven physical systems; mean of 6 ticks.
- **Movement:** N movers in one region, each proposing a random ±10 cm delta on X and Y per
  tick through a test system; mean of 5 ticks.
- **Proximity:** for every agent, `space::distance` to every other agent; count pairs within
  10 m.
- **Tick rate:** one simulated day of `DiurnalCycle` alone over an open region and a
  half-sheltered one (exposure 50%), and of `WeatherNoise` alone over an open region, at each
  rate.
- **Year drift:** 8,760 ticks of `WeatherNoise` alone over 500 regions starting at 1500
  centi-°C, seed 1.

The crate was not committed. Adding it as a permanent benchmark is how Vol. V Ch. 8's
performance budgets would be enforced.
