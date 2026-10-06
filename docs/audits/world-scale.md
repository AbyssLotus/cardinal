# World Scale: Are We Doing It the Best Way?

*2026-10-06, on branch `feat/agents`. The project owner's goal: worlds with cities on the scale of
Rome, New York, or Night City, and a world may hold more than one city, and the land between.
This audit sets the targets, measures how far the engine is from them, judges each of its methods
against them, and proposes the architecture and build order to close the gap. Measurements are
from `tests/reference/examples/scale.rs` (see `hostile-audit-review.md` §6).*

---

## 1. The target

| | Chosen |
|---|---|
| Machine | A mid-range gaming PC: Ryzen 7 (8 cores, 16 threads), Radeon 6800, 32 GB of memory |
| Speed | The player's scene ticks once a second, in real time; the rest of the world keeps pace |
| Detail | Tiered, as the spec prescribes: full detail where attention is, coarser elsewhere, consistent across tiers (Vol. I, Law 14) |
| First target | A generated city of about a million people (Rome), in a world that can hold more cities and countryside |

**Budgets.**
- **CPU:** one second of real time per simulated second, on eight cores.
  - The detailed scene gets up to 300 ms of it.
  - Everything else gets up to 400 ms.
  - The rest is headroom.
- **Memory:** about 16 GB for the simulation, leaving the rest for the system and the client.
  - For a city of a million and the world around it, that is **about 16 KB per resident, on
    average across all tiers.**
  - Each person in a distant city at aggregate detail costs bytes, not kilobytes.

The GPU is not counted on. The authoritative simulation is branchy and must be bit-identical on
every machine. Shader results differ between drivers, so the GPU belongs to the client.

---

## 2. Where we are

Every agent is simulated at full detail, every tick: a mind, geometric sight, beliefs, needs, and
individual items.

| | Today (release, one core) | Needed for a million |
|---|---|---|
| CPU per agent per tick | ~0.06 ms | ~0.0004 ms at one-second ticks |
| Memory per agent | ~160 KB (527 → 2,027 agents: 74 → 315 MB) | ~16 KB average |
| A million at full detail | ~60 s per tick, ~160 GB | — |

The gap is about **150× in CPU and 10× in memory**, before the world holds a second city. Tuning
the current loop narrows it; it cannot close it. The rest of this audit is about why, and what
does.

---

## 3. The laws are right; the defaults are not

**What to keep.** Cardinal's laws are what scale needs, and none needs to change:
- Committed state read by every system.
- Proposals resolved and committed atomically.
- One owner per fact.
- Declared reads and writes.
- Determinism.
- Intents as the only way to act.
- Minds that read beliefs.
- Worlds as packages.

Declared reads and writes are exactly what makes deterministic parallelism safe (Vol. V Ch. 5
§5.1). Committed-state reads are exactly what makes regions computable apart.

**What to change.** How the engine *executes* those laws was chosen for a village, and every
default below scales with *agents × ticks* when it should scale with *events*:

| # | Method today | Why it fails at scale | The better way | Spec already says |
|---|---|---|---|---|
| 1 | **Every system polls every entity every tick.** Hunger, fatigue, and heat for everyone; sight for everyone; every mind thinks every step. | Cost is agents × ticks, though most agents, most seconds, are doing nothing new. | **An agenda.** State that changes smoothly is a function of time, and the kernel keeps a deterministic index of *when* each entity is next due: a need crossing a line, a walk ending, a shift starting. Systems process what is due, and what changed, and nothing else. | "The scheduler should advance directly to the next due boundary" (Vol. I Ch. 4 §7.2) |
| 2 | **Continuous quantities are rewritten every tick.** About 3 of every 7.5 chronicle entries per agent per tick. | Writes, chronicle, and index work for numbers anyone could compute. | **An anchor and a rate**, as motion already is (A-3): hunger *at* tick t is `anchor + rate·(t − t₀)`. A fact is written only when the rate changes: eating, falling asleep, getting cold. | Motion segments (A-3) |
| 3 | **The store is a generic map of facts, a B-tree of B-trees, with provenance on every value.** | ~160 KB per agent; every read walks a tree; poor cache locality. | **A hot tier.** Dense arrays per hot fact type, indexed by a compact entity slot. The generic store stays for the long tail, and as the correctness oracle every faster store must match. Provenance compact (system, tick, cause in 8 bytes) or kept only in the chronicle. | "Hybrid store hot tier" (Vol. V Ch. 2; roadmap) |
| 4 | **Every apple is an entity,** with an id, a size, a material, a position, and beliefs about it in every mind that saw it. | A city's goods would outnumber its people a hundredfold. | **Holdings as quantities** (`kitchen holds apple × 7`), individuated into entities only where a scene needs them, and merged back after. | Holdings are quantities (Vol. III Ch. 4 §4.4); individuation (Ruling 12) |
| 5 | **Perception is geometric line of sight, for every observer, to everything in range, every tick.** Even after today's fixes it is most of every tick. | The single most expensive thing the engine does, done for people nobody is watching. | **Geometry only in the detailed scene,** and recomputed only when something there moved or changed. Coarser tiers perceive by co-presence ("in the same room or street"), or not at all. | Fidelity tiers (Vol. I Ch. 4 §7.3) |
| 6 | **A belief per thing ever seen, never forgotten.** Two copies of what is in view. | Memory grows with everything a person has ever seen. | **Bounded memory:** salience, forgetting, and one copy of the view. Coarse tiers hold *knowledge by kind* ("where bread is sold near home"), not a belief per loaf. | Information layer (Vol. II Ch. 4) |
| 7 | **Minds re-score every option against every belief, every think step.** | Cost per mind grows with what it knows; most thoughts conclude "keep doing this". | **Minds wake on triggers:** a need crossing its line, a step done, something perceived. Indexed beliefs answer "nearest food". Coarse tiers live by a **day plan**: timed segments (home, walk, work, eat, home) re-planned a few times a day. | Utility minds (Vol. V Ch. 9) |
| 8 | **Space is centimetres and a walking grid inside rooms.** No travel between open places without an opening. | A city's travel needs routes across thousands of streets; a world's, between cities. | **A route graph by level:** rooms and doors in the scene; streets, entrances, and floors in the city; roads and sea lanes between cities. Coarse travel is a timed segment along a route, with nothing written while it happens. | Regions and topology (Vol. III Ch. 1) |
| 9 | **Worlds are written by hand** (Ashford: 783 lines). | A million people and their homes cannot be written. | **Generation:** the package declares generators (districts, building types, households, trades), and the world is generated deterministically from the seed, with stable ids. | Generation (Vol. IV Ch. 4) |
| 10 | **One thread.** | Seven of eight cores idle. | **Deterministic shards:** systems, and entities within a system, split by content (id range, region), and merged in key order. Bit-identical at any thread count. | Vol. V Ch. 5 §5.1 |
| 11 | **One region at one level of detail.** | A world of several cities cannot hold them all in detail. | **The residency ladder per region:** DETAILED, AGGREGATE, COLD. A distant city is cohorts, stocks, and flows, always advancing, and actualized consistently when someone arrives. | Vol. V Ch. 5 §5.2; Law 14 |

The sight work just finished is not wasted. It is the cost of the detailed scene, which keeps
geometric sight. What changes is *who* gets the detailed treatment.

---

## 4. The architecture this points to

Three levels of detail, chosen per region by where attention is (the player, a tool, a
phenomenon worth watching). Each level is honest about the others (Law 14).

| Level | Where | Who is in it | What runs | Rough cost |
|---|---|---|---|---|
| **Scene** (Tier 0) | Around the player: a building, a street, a square | ~1,000–5,000 people and the things at hand | Everything built so far: minds, sight, beliefs, individual items, centimetre geometry; one-second ticks; recomputed on change | ~0.05 ms per person per second → ≤ 250 ms on one core; less when sharded |
| **City** (Tier 1) | Every other part of a city with attention nearby | Every resident, as an individual | Compact state; needs as anchors and rates; a day plan of timed segments; holdings as quantities; knowledge by kind; woken only by its agenda | ~10–20 events per person per day; ~1 KB per person |
| **Region** (Tier 2) | Other cities, countryside, the far side of the world | Cohorts (by age, trade, district), stocks, and flows | Aggregate rules on their own cadence (hours, days); travellers on the roads as boundary flows | Bytes per person; negligible CPU |

**Why this fits the machine.**

| | Rome, a million residents | Budget |
|---|---|---|
| City-level events | 1M × 20 a day ≈ 230 a second | Under a core's tenth |
| City-level memory | 1M × ~1 KB ≈ 1 GB | 16 GB |
| The scene | ~5,000 people at today's cost | 250 ms on one core |
| Whole world | Rome at city level, plus four cities of a million at region level | ~2 GB |

**Moving between levels** is where the hard work is, and the spec has already written the rules
(Law 14; Vol. I Ch. 4 §7.4). *Promotion* is the player turning a corner into a street:
- people are placed where their day plan says they are;
- the goods they hold become items;
- what they know by kind becomes beliefs.

*Demotion* folds back:
- items into holdings;
- the mind's current goal into its day plan;
- needs into anchors.

A round trip must change nothing that matters: deterministic, conserving, and tested by running
a district both ways and diffing.

---

## 5. Build order

Each phase starts with its amendment and ends in a gate measured by the bench, on the target
machine's numbers. Behaviour that the phase does not mean to change must not change: identical
state hashes, or a stated and tested reason.

| Phase | Builds | Gate |
|---|---|---|
| 0. **Measure** *(done)* | The scaling bench; exact sight speedups; tick continuity and id limits as errors | §6 of `hostile-audit-review.md` |
| 1. **The agenda** | Kernel: levels (A-20); what changed each tick, by fact type; a deterministic "due at tick" index; per-system time and proposal counts. Living: hunger and fatigue as levels, and the ticks they cross a line. Minds wake on what is due or what changed. | Ashford's week behaves the same. In the village bench, an idle agent costs nothing, and chronicle entries per agent fall by at least half. |
| 2. **The hot store** | Dense columns for hot facts behind the same store trait; compact provenance; per-tick arenas and reused buffers; a conformance suite against the reference store. | Identical hashes. Memory per detailed agent ≤ 20 KB; ticks ≥ 3× faster. |
| 3. **Parallel shards** | Systems, and entities within them, evaluated across cores; merged in key order. | Bit-identical at 1, 2, 4, 8, and 16 threads; ≥ 5× on eight cores. |
| 4. **City-level residents** | Day plans, holdings as quantities, knowledge by kind; promotion to the scene and demotion back. | A district of 100,000 runs in real time on one core in under 200 MB. A round trip through the scene changes nothing. |
| 5. **City geography and generation** | Streets, buildings, entrances, floors, and the route graph; a package-declared generator. | A Rome of a million generated in minutes, in under 4 GB. The player walks across it in real time. |
| 6. **The world** | The residency ladder across regions; cities and countryside at region level; roads and travellers between them. | Rome at city level, four cities at region level, the scene at one second: all on the target PC, in real time, and the same with the same seed. |

**Before phase 1:**
- Commit the work in flight.
- Keep running the bench on each change.
- Put a budget gate in CI as soon as the numbers settle.

---

## 6. Risks, plainly

- **Promotion and demotion are the hard part.** Getting Law 14 right, with books that balance and
  a round trip that changes nothing, is where effort will go. It is also where bugs will hide. It
  needs its own adversarial suite.
- **An agenda makes determinism subtler.** What is due must be ordered by (tick, key), never by
  insertion. Waking on change needs the kernel to say what changed, cheaply.
- **Two kinds of mind,** scene and city, must agree enough that a promoted person does not behave
  like a different person. The day plan should be what the scene mind would have done, on
  average, not a different personality.
- **The chronicle at city scale** must record events, not every number that moves. With anchors and
  rates it mostly will. What the durable log is (`hostile-audit-review.md` §2, finding 13) has to
  be decided before persistence is built.

---

## 7. A second outside review, weighed

*An outside reviewer proposed 28 points and a top five. Each is weighed here against the code
and this plan. Agreeing where it is right is not the same as adopting it whole.*

**Already done.** Their points, and where the code already has them:
- **Motion segments (their #1, and their first task)** are A-3. A body in motion writes at its
  segment's ends, and its position between is derived.
- **Spatial partitioning (#5)** is A-2: a hierarchical grid, the structure they recommend over
  octrees.
- **An incremental digest (#13)** is in `kernel/src/hash.rs`: O(changes), by add and remove.
- **Batch commits and grouped proposals (#9, #10)** are already the tick: proposals are grouped
  by fact, resolved once, and applied as one `CommitBatch`.
- **Per-system frequency (#19)** exists as each system's cadence.
- **Only entities with the needed facts take part (#3)**, because systems iterate
  `entities_with(fact)`.

Their own diagnosis is right that #3 and #19 do not save enough. Every organism has hunger, so
every organism is still visited. The saving is in #4 and #20 below.

**Right, and already this plan:**

| Their point | Here |
|---|---|
| Dirty sets and event-driven activation (#4, #20, #16) | Phase 1, the agenda |
| A data-oriented hot store, keeping the B-tree store as the reference (#7, #8) | Phase 2 |
| Deterministic parallelism (#14) | Phase 3 |
| Simulation LOD and aggregation (#17, #18) | Phases 4 and 6 |
| Hot and archival history (#12) | Persistence, after the durable log is decided |

**Right, and adopted now:**
- **The kernel should say what changed (#4).** It has no such primitive today. `apply` knows
  every key it touched, so it can keep, per fact type, the entities changed in each recent tick.
  Then a system can ask "whose position changed since tick t?" in time proportional to the
  answer. Waking minds, incremental sight, and work that only matters on change (portal danger is
  still *computed* every tick, though now written only on change) all need it. **Added to
  phase 1.**
- **Per-system metrics (#27).** The bench needed an outside profiler to find that sight was 80% of
  a tick. The kernel should time each system, and count its proposals, every tick. This sits
  outside the simulation, so it cannot touch determinism, and feeds the meters Vol. V Ch. 8 asks
  for. **Added to phase 1.**
- **Per-tick buffer reuse and arenas (#22, #23).** Allocation shows in the profile (a few percent),
  and Vol. V Ch. 2 §2.3 already prescribes arenas by tick. **Added to phase 2,** where the store's
  layout changes anyway.

**Right in part:**
- **"Never store what can be derived" (#2).** True of positions, distances, and temperatures,
  which Cardinal already derives. But some derived things are stored here for a reason the review
  does not see: domains never call one another, so a derivation another domain needs crosses as a
  fact. That is why what each body sees is a fact.
  - The better answer is the one the spatial index already uses: the owner registers a derived
    view the kernel hosts, other domains query it, and nothing is stored per tick.
  - Visibility is the case worth doing this way: it is expensive, read by Information, and today
    held twice (`in_view` and `in_sight`). **Carried into the incremental-sight work.**
- **Incremental spatial queries (#6).** Worth it for what is expensive to derive (visibility), not
  for proximity. With the grid, a neighbourhood is cheaper to recompute than to keep, since keeping
  costs memory per entity per neighbour.
- **Execution classes (#15).** Useful, but declared reads and writes already tell the scheduler
  most of it. The one addition worth making is a system declaring that it is *per entity*, so the
  kernel can shard it without being told how. **Carried into phase 3.**
- **Compiling packages (#25, #26).** The loader already resolves ids and seeds facts once. What is
  still recomputed is derived work done every tick whatever changed, which the change-tracking
  above removes. And static lookups are slow only because the store is a B-tree, which phase 2
  fixes.
- **A compact chronicle (#11).** The bigger problem is not its layout but that entries hold no
  values (`hostile-audit-review.md`, finding 13). Decide what the durable log is first, then
  compress it.

**Where we differ:**
- **Order.** Their top five put the hot store second and event-driven scheduling third. Their own
  stated order is "algorithm, then data model, then memory layout", and that puts event-driven
  first. A store ten times faster still does work for a million idle agents; an agenda does none.
  **This plan keeps the agenda first.**
- **SIMD (#24).** They place it last, rightly. But its value here is smaller than they suggest. The
  costs measured are branchy (sight, minds, perception), not wide arithmetic over arrays.
- **What their list leaves out.** They omit the two costs this engine actually has most of:
  - **perception and belief:** sight, mirrors, and memories, roughly 80% of a tick and most of the
    store;
  - **the consistency of moving between levels of detail.** Law 14 is the hard part of
    aggregation, not the aggregation itself. A promoted person must be the person the aggregate
    said they were.

**Their closing principle** is right, and it is this plan's: *simulate as though only what can
affect the outcome exists*. With one Cardinal amendment: what cannot affect the outcome *now* is
still simulated, at the cost its level of detail allows. It is never frozen, so that when it does
matter it is what the world says it is (Vol. V Ch. 5, *The Frozen Duchy*).
