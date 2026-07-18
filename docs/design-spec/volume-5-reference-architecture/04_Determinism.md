# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 4
# Determinism

> *Determinism is not a property you have. It is a property you defend.*

---

# Chapter Overview

Every volume has leaned on the same promise: identical inputs, identical world. Replay depends on it (Vol. II), emergence auditing depends on it (Vol. III Ch. 11), generation and testing depend on it (Vol. IV), and multiplayer will depend on it absolutely (Ch. 5).

This chapter is the engineering of that promise — and its central claim is unglamorous:

**Determinism is not a feature to implement. It is a class of bugs to prevent, forever.**

Nondeterminism enters engines through a small number of well-known doors. This chapter names each door, specifies the lock, and then — because locks fail — specifies the alarm system: the testing regime that catches drift within one commit of its introduction rather than one year.

---

# 4.1 The Doors

## Door 1 — Randomness

The classic, and the most fully solved. The kernel owns a counter-based PRNG forest:

```text
world_seed
└── domain streams (weather, combat, economy, …)     # Vol. II Ch. 2
    └── issued substreams: (system, tick, scope-key)  # Ch. 3
```

Rules: systems receive issued substreams and cannot construct their own; no call ever touches OS entropy; stream state is part of the save (Vol. II Ch. 5); and *scope keys are content-derived, not order-derived* — the deer's flight roll is keyed to the deer, not to "the third roll this tick," so unrelated changes cannot shift its draw. Counter-based generators (rather than sequential state-march) make keyed access natural and parallel-safe.

## Door 2 — Iteration Order

The subtlest door, and the one that ships most drift. Any unordered container iterated during evaluation — hash maps above all — injects platform- and history-dependent order into proposals, tie-breaks, and event sequences.

The lock is total: **no unordered iteration anywhere in the simulation path.** Entities iterate by id order; proposals sort by declared key (system, basis, sequence); regions by stable index; ties break by explicit comparator, never by memory address or insertion accident. Languages whose maps randomize iteration (deliberately or not) get wrapper types in the kernel that refuse raw iteration at compile time where the language allows it.

## Door 3 — Floating Point

Cross-platform float reproducibility is the door with genuine tradeoffs, so the architecture specifies a policy ladder rather than pretending one answer fits:

```text
Level 0 — same binary, same platform:    IEEE754 + fixed compiler flags
          (no fast-math, no FMA contraction variance) — cheap, fragile across ports
Level 1 — same source, all platforms:    strict IEEE754 semantics, ordered
          reductions, no platform intrinsics in simulation math
Level 2 — the reference recommendation:  fixed-point / integer arithmetic for
          all *accumulated* simulation quantities (needs, stocks, currency,
          populations); floats only for derived, non-persisted computation
```

The recommendation is Level 2 for a reason Volume III makes obvious: Cardinal's load-bearing numbers are *ledgers* — conserved quantities audited over centuries (Kepler's air, Thornwall's shillings). Ledgers deserve integers. Where continuous math is genuinely needed (fields, distances), it is either derived-only (recomputed, never accumulated) or wrapped in deterministic fixed-point routines. This single choice removes the worst cross-platform door entirely and makes Chapter 5's networking tractable.

## Door 4 — Time and Environment

No wall-clock, no locale, no environment variables, no filesystem ordering, no thread timing inside evaluation — Chapter 3's hermetic seal already forbids these; this chapter adds the audit: the kernel's evaluation context simply *does not expose* ambient anything, so violations require visible smuggling rather than accident.

## Door 5 — Parallelism

Scheduling nondeterminism from concurrent execution. Chapter 5 owns the machinery; the law is stated here: parallel execution must be *observationally equivalent to the published serial order* — same committed batch, same chronicle, bit-identical. Parallelism that wins a race is not faster; it is wrong.

---

# 4.2 The Alarm System

Locks rust. The regime that finds drift:

**Twin runs.** CI runs every reference scenario twice from the same seed and diffs committed state hashes per tick. Divergence names the first differing tick — and the diff of that tick's proposals names the system. This is the cheapest, highest-yield determinism test that exists; it runs on every commit.

**Golden transcripts.** Byte-exact recorded runs (the current engine's golden combat tests are the validated pattern) for focused subsystems, catching drift that twin runs on short horizons might miss.

**Save-splice runs.** Run 1000 ticks; save at 500; load and continue; diff against the unbroken run. Catches state that lives outside the save (Vol. II Ch. 5's deterministic-reload clause, mechanized).

**Cross-platform hashes.** At whatever policy level the project has adopted, reference runs on each supported platform must produce identical per-tick hashes. This test *is* the definition of the level being honored.

**Chaos ordering.** Where parallelism exists: run the same tick with adversarial scheduling (random worker counts, forced preemption) and require identical commits. Races that survive chaos runs are races the design actually excluded.

Per-tick state hashing — a canonical serialization of committed reality, hashed incrementally — is the shared infrastructure under all five. It is worth building early and keeping fast, because every determinism test reduces to comparing it.

---

# Designer Note
## The Bug You Find in a Year

Nondeterminism has a signature cruelty: it is almost free to prevent and almost unpayable to retrofit.

The hash-map iteration introduced in month three surfaces in month fifteen as a replay that diverges at tick 2,144,006 — in the *economy*, though the bug is in *pathfinding*, because a route tie broke differently and a caravan arrived a day late and prices moved. There is no stack trace from a price to an iteration order. The debugging session is archaeological, and the fix invalidates every recorded transcript and reference signature the project owns.

This is why the chapter treats determinism as *defense*: wrapper types over raw maps, streams issued rather than constructed, ledgers in integers, twin runs on every commit. Each measure looks like paranoia the week it is added. Each is the memory of a fifteen-month bug someone else already paid for.

Defend the property while it is cheap. It is only cheap once.

---

# 4.3 Common Queries

- What is the state hash at tick N, and where do two runs first diverge?
- Which system produced the first differing proposal at the divergence tick?
- What float policy level is this build certified for, and by which passing runs?
- Which RNG substream served this outcome, under what scope key?
- Did save-splice runs pass for this release across all reference worlds?

---

# 4.4 Engineering Invariants

Every implementation SHALL preserve these rules.

1. All randomness flows from the seeded forest through kernel-issued, content-keyed substreams.
2. No OS entropy, wall time, or ambient environment is reachable from evaluation.
3. No unordered iteration exists on any simulation path.
4. All tie-breaks are explicit, total, and content-keyed.
5. Accumulated simulation quantities use deterministic arithmetic (Level 2: fixed-point/integer).
6. The float policy level is declared per build and enforced by cross-platform hash tests.
7. Parallel execution is bit-equivalent to the published serial order.
8. Per-tick state hashes are cheap, canonical, and part of the kernel's contract.
9. Twin runs and save-splice runs gate every release; golden transcripts guard hot subsystems.
10. A determinism failure is a release blocker, never a known issue.

---

# 4.5 Anti-Patterns

### The Debug Print That Rolls

Logging or telemetry that consumes from a simulation RNG stream, shifting every subsequent draw. Observation must be stream-free; issue observers no streams.

### Map Order Roulette

"It's deterministic on my machine." Hash iteration order is a per-process, per-platform, per-version accident. The wrapper types exist so this sentence is never typed again.

### Fast-Math Optimism

Enabling aggressive float optimization "because the speedup is free." The payment arrives as Level 0 collapse: same source, different binaries, different worlds.

### Retrofit Season

"We'll tighten determinism before multiplayer." See the designer note; the retrofit costs a rewrite of every accumulated quantity plus the invalidation of all recorded history. Determinism is a founding decision or a second engine.

### The Tolerated Flake

A determinism test that "sometimes fails, rerun it." A flaking twin run *is the bug report*. Quarantining it converts the cheapest possible detection into the fifteen-month archaeology dig.

---

# 4.6 Future Evolution

Future versions may add:

- deterministic SIMD math libraries raising Level 2 throughput
- formal audits of the no-unordered-iteration rule via static analysis
- divergence bisection tooling (auto-localizing the first differing proposal)
- reproducibility certificates shipped with saves (hash chains per era)

Each strengthens the defense.

None retires it — the doors never close permanently; they are only guarded.

---

## Preparing for the Next Chapter

Determinism defended at one scale must now survive three more:

**Scale** — parallelism across cores, streaming across space, and networking across machines, all under the same bit-identical law.

---

# END OF CHAPTER 4
