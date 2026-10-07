# The Hostile Audit, Checked Against the Code

*2026-10-06, on branch `feat/agents` at `9a3fe11`. An outside "hostile architecture audit" made 19
findings and proposed five experiments. Each finding is checked below against the code as it
stands, with measurements where a number settles it. Verdicts: **Holds**, **Partly** (true in
part, or true but overstated), **Stale** (fixed since, or never true), **Not yet** (assumes
machinery that does not exist).*

---

## 1. Bottom line

Most of the audit holds. Its strongest points are the ones it could not have measured:

- **The store grows without bound.** Ashford goes from 1,307 facts at load to 5,398 after a week,
  and tick time rises with it (§3).
- **The chronicle holds no values and no causal links.** It cannot be replayed onto a snapshot,
  and it cannot explain a state by walking causes backward, though its doc comment says it can.
- **Two architectural laws are still conventions:** tick continuity and the single mutation path.
  Both are small fixes.

Three findings are stale or premature:
- runtime ids (A-15 answers most of the audit's questions);
- motion segments (A-3 already does what it proposes);
- thread torture (nothing runs in parallel yet).

Its general verdict, *more mature as a specification than as an engine*, is fair:
- 13,400 lines of spec;
- five of the twelve domain crates are 36-line stubs;
- persistence, observe, and replay are placeholders.

---

## 2. Finding by finding

| § | Finding (audit severity) | Verdict | Evidence in the code | What to do |
|---|---|---|---|---|
| 3 | Spec ahead of implementation (critical) | **Holds** | Spec is 13,400 lines. Built: kernel 3.4k, physical 5.9k, packages 3.8k, minds 1.7k, living 1.1k, information 1.0k, economy 0.5k, resources 0.3k, society 0.2k. Stubs: culture, knowledge, conflict, institutions, ecology (36 lines each); persistence (8); observe, replay, narrator, tools (5). Against that, 19 amendments already correct the spec from what code taught it. | Mark each spec section *built*, *amended*, or *reserved*. |
| 4 | Chronicle scaling (critical) | **Holds**, misattributed | Motion is already segmented (A-3). The 47.5 ms / 20,094-entry miss is the per-fact cost of segment *starts*. But Ashford writes about 300 entries a tick for 27 minds (§3). Vitals (heat, hunger, fatigue) are rewritten for every organism every tick. Static portal danger is rewritten every tick. The kernel chronicles every proposal, changed or not. | Skip no-op writes. Write portal danger on change. Treat vitals as motion is treated: a rate and an anchor, read between writes. |
| 5 | Runtime entity lifecycle (critical) | **Partly** | A-15 answers allocation: ids are `floor + tick·2²⁴ + slot·2¹⁴ + n`, so they are deterministic, disjoint per system, never reused, and derive from the tick on restore. An aborted tick commits none. Still open: (a) leaving the world tombstones only placement, so a used-up thing keeps its size and material forever (345 of each in a week); (b) the per-system cap of 16,384 ids a tick and the 1,024-system cap are `assert!`s, so they panic rather than abort the tick; (c) no births, splitting, or merging. | Define *gone*: retire a thing's facts, or move them to history. Turn the caps into `TickError`s. |
| 6 | Tick continuity (high) | **Holds** | `run_tick` checks each proposal's basis against the *requested* tick minus one, never against `store.tick()`. Asking for tick 7 on a store at tick 2 runs. | Check `store.tick() == N − 1` up front, with an error variant and tests for skipped, repeated, and reversed ticks. |
| 7 | Mutation boundary is convention (high) | **Holds** | `RealityStore::apply` is a public trait method and `MemoryStore::seed` is public. `living.rs`, `needs.rs`, and `engine.rs` call `apply` directly. The boundary holds only for code that reaches the world through `LoadedWorld`, which hands out `&MemoryStore`. | Seal `apply` behind the tick, for example with a commit token only `run_tick` can make. Make seeding a builder that ends when loading does. |
| 8 | MemoryStore won't scale (high) | **Already explicit** | It is documented as "the reference store". The roadmap lists a hybrid hot tier. The 3D audit traced its own miss to it. | Keep it as the correctness oracle. Give any future store a conformance suite, as the spatial index has. |
| 9 | Package format becoming a language (high) | **Partly** | 68 section and rule kinds, but no expressions or conditionals. Mechanisms are closed sets (needs arise by *bond* or *dependence*; jobs *carry* or *make*). That is the audit's option A, chosen deliberately. Untested: only one world exists, and each new feature this month needed engine code. | Run experiment 4 (a second, unlike world) before adding more sections. |
| 10 | Cross-domain temporal semantics (high) | **Holds** | Vol. V Ch. 3 says tick N reads N − 1, but not what follows: every hop across domains costs a tick. Picking an apple takes four hops (act, yield, arrive, perceive): 4 s at 1-second ticks, 40 min at 10-minute ticks. Minds hard-code `SETTLE_TICKS = 3` and `FEEL_TICKS = 6`. Rates are the same at any tick length (A-1); chains of acts are not. | Write the latency contract into the kernel chapter. Decide whether intents may chain within a tick. |
| 11 | Provenance too weak (medium/high) | **Holds, and is worse** | A composed fact keeps only the first proposal's system and cause. A `Cause` is a static label (`"picked"`) with no link to the fact or event that triggered it, and chronicle entries carry no parent. The doc comment in `kernel/src/events.rs` promises that "any state can be explained by walking its causes backward"; the data cannot do it. | Make causes nodes that can name their parents, shared rather than copied. Keep every contributor to a composed fact. |
| 12 | State hash not cryptographic (medium) | **Mostly answered** | `kernel/src/hash.rs` already says "a determinism check, not a cryptographic commitment". The real gap: provenance is excluded, so twin-run tests never check causes or the chronicle. | Add a chronicle digest to the determinism tests. Renaming is optional. |
| 13 | Persistence unproven (high) | **Holds, and is worse** | The crate is 8 lines. The chronicle is an in-memory `Vec`, and **its entries hold no values**, so "replay the chronicle tail onto a snapshot" cannot work from it. Replay today means re-simulating from seed and inputs. | First decide what the durable log is: inputs plus seed, or value deltas. Then build snapshots. |
| 14 | Adversarial testing (high) | **Partly** | Exists: undeclared reads and writes, stale basis, duplicate system ids, a stranger writing restricted facts, owner validation aborting a commit, a failed tick committing nothing, index vs. scan under random churn, loader refusals and every validation problem named, every fact a system names owned, twin-run determinism. Missing: skipped, repeated, or reversed ticks; property tests (no `proptest`); shuffled system order; id exhaustion; snapshot-and-replay. | Add these to a permanent adversarial suite. |
| 15 | Performance budgets in CI (high) | **Holds** | No benchmark in the repo; the 3D audit's numbers were one-off runs. CI runs fmt, build, test, and doc only. | A release-mode bench binary with budgets, run in CI: start with the Ashford week and 10k/100k movers. |
| 16 | Cognitive tax (medium) | **Partly** | 13,400 spec lines, 19 amendments, 18 rulings. But much is enforced in code: crate dependencies, declared reads and writes, restricted facts, owner validation, validation layers, the ownership test. What is left as convention is §6 and §7. | Fix §6 and §7, then prefer an enforcing test over prose for each new rule. |
| 17 | 3D readiness exposed missing contracts (high) | **Partly stale** | A-1 to A-4 closed time units, the spatial index, motion, solids, terrain, sight. Open: bodies don't collide with each other, no presentation stream, no pitch or roll, proximity cost grows with room count. | Leave until a 3D client is in scope. |
| 18 | Continuous motion first-class (high) | **Stale** for motion | A-3: a segment stores start, target, start tick, end tick; positions between are derived. | Apply the same idea to vitals (§4). |
| 19 | Observe stage is mostly architectural (medium) | **Holds** | Stage 7 of `run_tick` is a comment: "nothing on the critical path here yet". The observe service is 5 lines. | Mark it reserved in the spec until built. |
| 20 | World authoring (unscored) | **Holds** | One world, 783 hand-written lines, hand-assigned ids. Generation (Vol. IV Ch. 4) is unbuilt. | Not yet pressing; becomes pressing with the second world. |

---

## 3. Measured: a week in Ashford

Release build, 10-minute ticks, 27 organisms, all with minds, nobody directed.

| Day | ms per tick | Chronicle entries per tick | Facts in the store |
|---|---|---|---|
| load | — | — | 1,307 |
| 1 | 2.9 | 282 | 2,779 |
| 3 | 2.8 | 293 | 3,641 |
| 5 | 3.4 | 298 | 4,545 |
| 7 | 5.4 | 293 | 5,398 |

Worst single tick: 759 entries. Whole week: 296,052 entries in 3.4 s.

**Who writes the chronicle**, per tick on average:
- body heat 27, hunger 27, fatigue 26: every organism, every tick;
- portal danger 23: every opening, every tick, unchanged;
- region temperature 22, illumination 17;
- felt fatigue 13.

**What grows**, from day 1 to day 7:

| Fact | Growth | Why |
|---|---|---|
| `info.belief.made_of` | +1,139 | Minds never forget what eaten things were made of. |
| `info.belief.place_of` | +702 | Minds never forget where they last saw things that are gone. |
| `physical.body.size` | +345 | A used-up thing keeps its size after leaving the world. |
| `physical.material.made_of` | +345 | …and its material. |
| `mind.gave_up_on` | +26 | Kept even after the thing is gone. |

At this rate a simulated year holds about 165,000 facts, almost all about things that no longer
exist. Minds scan their beliefs every thought, so tick time grows with it: 2.9 ms on day 1, 5.4 ms
by day 7. At 10,000 agents the same per-agent rates come to about 110,000 chronicle entries a tick.

---

## 4. The five experiments, as of now

| Experiment | Can it run today? | Nearest thing that exists |
|---|---|---|
| 1. A million entities | No: no bench harness, and the reference store would not cope | The 3D audit's one-off runs (100k movers: 47.5 ms) |
| 2. Break the constitution | Mostly | The kernel contract tests and `engine.rs`; tick continuity and the mutation path would fail it today |
| 3. Ten-year replay | No: no snapshots, and the chronicle holds no values | Twin runs from one seed (a day, a week); a decade of climate held in its band (3D audit, phase 1) |
| 4. World independence | Yes | Only Ashford exists |
| 5. Thread torture | Not yet: evaluation is sequential | Shuffling the system order would test the same property today |

---

## 5. Recommended order

Smallest and surest first. Each step is done when a test that tries to break it fails to.

1. **Tick continuity.** Refuse a tick that is not `store.tick() + 1`. Turn the id caps from panics
   into tick errors.
2. **Seal the mutation path.** Only `run_tick` can apply. Seeding ends with loading.
3. **Stop chronicling nothing.** Skip writes that change nothing. Portal danger only on change.
4. **Bounded growth.** A thing that leaves the world takes its facts with it, or into history.
   Minds forget things they know to be gone.
5. **Adversarial suite.** Skipped, repeated, and reversed ticks; shuffled system order; id
   exhaustion; a chronicle digest in every twin-run test.
6. **Benchmark gate.** A release bench with budgets in CI: the Ashford week, then 10k and 100k
   movers.
7. **Contracts before machinery.**
   - Write the cross-domain latency contract.
   - Give causes parents.
   - Decide what the durable log is, before any snapshot code.
8. **A second world**, unlike Ashford, to test whether worlds really are data.

---

## 6. Scaling bench, and first fixes (2026-10-06)

`tests/reference/examples/scale.rs` adds N villagers to Ashford's yard, 10 m apart, each with a
mind, sight, and hunger, with a fruit tree for every ten. It reports ms per tick, chronicle
entries per tick, and facts in the store. Run it with:

```text
TICKS=48 cargo run --release -p reference --example scale -- 127 527 1000 2000
```

`CENSUS=1` lists the largest fact types. Results over eight simulated hours at ten-minute ticks:

| Agents | ms/tick before | ms/tick now | Worst tick before | Worst tick now | Entries/tick | Facts at end |
|---|---|---|---|---|---|---|
| 154 | 9.8 | 5.6 | 34 | 16 | 1,143 | 17,224 |
| 554 | 55.1 | 23.7 | 245 | 60 | 3,997 | 63,709 |
| 1,027 | — | 53.2 | — | 139 | 7,681 | 133,142 |
| 2,027 | 161 | 124 | 534 | 300 | 15,141 | 264,733 |

**Where the time went.** Sampled at 500 agents, sight was 80% of every tick. Every observer
tested lines to everything in range, and each line:
- rebuilt its list of candidate blockers;
- walked the containment chain;
- tested every place in the yard that held anything, including each villager carrying an apple.

That made sight O(population) per line.

**Fixed, all exact.** The store ends with the same facts, and the sight, perception, engine, and
index-equivalence suites pass unchanged.
- Fact types compare by a code computed when they are declared, not by their names
  (`kernel/src/fact.rs`). An entity's facts are walked from `FactType::MIN`.
- Sight looks inside a place only when the line passes through it, and passes over anything that
  cannot block without working out its shape.
- Sight asks only about places within reach of the line, and works out each frame's bodiless
  places once per evaluation (`Bare`), not once per line.
- Two bodies standing in one place skip the containment walk.
- Portal danger is written only when it changes.
- **Stability:**
  - `run_tick` refuses any tick but the next (`TickError::NotNext`).
  - A system asking for too many new ids fails the tick (`IdsExhausted`) instead of panicking.
  - Too many systems, or a tick past what ids can number, is refused (`BeyondIds`).
  - `kernel/tests/continuity.rs` and `creation.rs` try each of these and find them refused, with
    nothing committed.

**What remains.** Cost still grows faster than the population: 0.04 ms per agent at 554, 0.06 ms
at 2,027. Sight is still most of a tick, and the store's working set is about 130 facts per
agent:
- what each sees, held twice (`physical.sense.in_view` and `info.in_sight`);
- where each believes those things are.

In order of payoff:
1. **Sight that skips unchanged views.** A still observer among still things need not retest a
   line. This needs a stored fact for clear lines, with light applied each tick, so it needs an
   amendment.
2. **One copy of what is in view,** not two.
3. **Vitals as a rate and an anchor,** as motion already is: about 3 of every 7.5 entries per
   agent per tick are hunger, fatigue, and body heat rewritten.
4. **Bounded growth** (§5 of the recommended order) for long runs.
5. **A hot-tier store:** single-valued facts out of nested B-tree maps.
