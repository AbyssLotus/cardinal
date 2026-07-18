# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 3
# The Simulation Kernel

> *The tick is a machine for keeping promises.*

---

# Chapter Overview

Volume II, Chapter 2 promised: reality changes only at tick boundaries, systems read committed state and propose, conflicts resolve deterministically, commits are atomic.

This chapter builds the machine that keeps those promises: the tick pipeline, the scheduler, proposal resolution, and cadence management. It also settles — as an architecture decision, not a doctrinal fudge — the relationship between the full read→propose→resolve→commit model and simpler execution profiles.

The settlement up front, because everything else depends on it:

**The canonical model is read→propose→resolve→commit.** It is the only model that scales to parallelism (Chapter 5), makes conflicts explicit, and gives the chronicle clean per-tick causality. Simpler profiles (notably serialized incremental application, the current engine's model) are *valid executions* of the canonical model exactly insofar as they are observationally equivalent to it — same inputs, same committed reality. Where the reference design and a shortcut disagree, this volume specifies the reference design.

---

# 3.1 The Tick Pipeline

One tick, end to end:

```text
 TICK N (world committed at N-1 is immutable input)
 │
 ├─ 1. SCHEDULE   — which systems run this tick (cadence + dirty hints)
 ├─ 2. EVALUATE   — systems read committed state, emit proposal sets
 │                  (parallelizable per read/write analysis — Ch. 5)
 ├─ 3. RESOLVE    — proposals grouped per fact; owner-declared composition
 │                  rules (Vol. IV Ch. 2) applied; conflicts settled
 ├─ 4. VALIDATE   — owning domains' coherence checks on the resolved batch
 ├─ 5. COMMIT     — store.apply(batch): atomic; reality is now N
 ├─ 6. CHRONICLE  — events appended with provenance (Ch. 6)
 └─ 7. OBSERVE    — services notified: persistence, telemetry, narrator feed
                    (read-only, off the critical path where possible)
```

Stage discipline:

**Evaluation is hermetic.** A system receives: a committed-state view scoped to its declared read set, its RNG substream, the clock, and its rules. It returns proposals. It touches nothing else — no store handles, no globals, no other systems. Hermeticity is what makes stage 2 parallelizable and every system testable in isolation.

**Resolution is mechanical.** Stage 3 has no judgment: it sorts proposals into per-fact groups (deterministic order — Chapter 4), applies the fact owner's declared composition function, and produces one resulting value per touched fact. A conflict without a declared composition rule is a *validation failure at package load* (Vol. IV Ch. 7, layer 3), never a runtime improvisation.

**Failure aborts the tick.** Any stage failing means tick N never happened: the store still says N-1, and the failure is reported with full context. Volume II's "consistency over progress," executably.

## Proposals

The pipeline's currency, worth specifying precisely:

```text
Proposal
├── system         — proposing system id
├── basis          — (entity, fact_type) read epoch          # for conflict detection
├── change         — set | delta | create | tombstone | relate
├── magnitude      — typed payload
└── cause          — event reference(s) this proposal asserts
```

The `cause` field is where chronicle honesty is manufactured: a proposal must name the event it participates in (the attack, the meal, the decree), and stage 6 assembles committed events from exactly these assertions. No proposal, no cause; no cause, no commit.

---

# 3.2 The Scheduler

The scheduler owns two questions: *who runs this tick*, and *in what order do their effects compose*.

## Cadence

Each system registers its cadence (every tick, hourly, daily, monthly — simulation time, per Volume II). The scheduler maintains the cadence calendar and, at stage 1, selects the due set. Two refinements earn their complexity:

**Dirty-region hints.** Systems may declare interest predicates ("run for regions where weather changed"). Hints *narrow* work; they may never *add* work outside cadence, and a hintless run must produce identical results (hints are pure optimization — Chapter 4's observational-equivalence clause applies).

**Alignment ticks.** When multiple cadences coincide (the daily set and the monthly set on day 30), all due systems run in the same tick under the same ordering rules — never as stacked mini-ticks, which would create observable intermediate states.

## Ordering

Execution order derives from declarations, not code position:

```text
inputs:  each system's read set, write set, declared happens-before edges
process: build DAG → detect cycles (load-time failure) →
         topological order with deterministic tie-breaking (Ch. 4)
output:  the tick's execution order — stable for a given
         (engine version, package, enabled domains)
```

The order is *published*: it appears in engine diagnostics and in the save's provenance, because Volume II said changing the order changes reality — so the order is part of what a save pins.

---

# 3.3 Determinism Duties (Preview)

The kernel carries most of Chapter 4's weight, so the duties are named here where they live:

- stable iteration everywhere (entities, proposals, regions — sorted or index-ordered, never hash-ordered)
- RNG substreams issued by the kernel per (system, tick, scope) — systems cannot construct streams
- no wall-clock, no ambient environment, no I/O inside evaluation
- floating-point policy applied uniformly (Chapter 4 owns the policy choice)

---

# Designer Note
## Why Not Just Apply Deltas as You Go?

The serialized shortcut — run systems in order, apply each one's deltas immediately, wrap the whole turn in one transaction — is genuinely attractive. It is simpler, it needs no proposal machinery, and the current engine shipped six milestones on it.

Its costs are the reasons this volume specifies the full model anyway:

*Implicit ordering semantics.* Under incremental application, system B sees system A's writes mid-tick — so the execution order becomes invisible simulation input, baked into every outcome, discoverable only by reading scheduler internals. The canonical model makes ordering explicit exactly once, at resolution, under declared composition rules.

*No parallelism path.* Immediate application serializes by construction. The proposal model parallelizes stage 2 for free wherever read/write sets are disjoint (Chapter 5's entire foundation).

*Conflicts vanish instead of resolving.* Last-writer-wins is a composition rule nobody declared. When healing and damage land on the same tick, Volume IV Chapter 2 says the package decides the order — a decision incremental application silently takes away.

The shortcut remains a legitimate *profile* for small worlds: run stage 2 serially, treat each system's output as instantly-resolved proposals, commit at the end — and prove observational equivalence against the reference model in the conformance suite. That last clause is not decoration. It is what keeps the shortcut a profile instead of a fork.

---

# 3.4 Common Queries

Kernel diagnostics every implementation should answer:

- What ran this tick, in what order, and why (cadence, hints, dependencies)?
- What did each system propose, and how did each contested fact resolve?
- What is the published execution order for this configuration, and when did it last change?
- Where did a failed tick fail — stage, system, fact, rule?
- What is the per-stage time and allocation profile of tick N? (feeds Ch. 8)

---

# 3.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. The tick pipeline's stages are fixed and ordered; no stage is skippable.
2. Systems evaluate hermetically: scoped reads, issued streams, proposals out, nothing else.
3. All writes are proposals; all proposals carry basis and cause.
4. Resolution applies owner-declared composition rules mechanically; undeclared conflicts fail at load, not at runtime.
5. Commit is atomic; a failed tick leaves reality at N-1 exactly.
6. Every committed change appears in the chronicle via its proposals' causes.
7. Execution order is derived from declarations, deterministic, and published.
8. Cadence and hints affect performance only; a hintless run is observationally identical.
9. Coinciding cadences run in one tick under one ordering — no intermediate observable states.
10. Alternative execution profiles are valid only under proven observational equivalence to this pipeline.

---

# 3.6 Anti-Patterns

### The Mid-Tick Peek

Any path by which a system observes another's current-tick output. The canonical model's entire value is that this is structurally impossible; guard the hermetic seal accordingly.

### Ordering by Accident

Execution order emerging from registration sequence, file iteration, or hash maps. Order is reality (Vol. II); reality does not depend on the filesystem.

### The Improvised Merge

Runtime "sensible" handling of undeclared conflicts. Sensible is not deterministic, and deterministic is not negotiable. Fail at load.

### Cause-Free Commits

Letting proposals through without event causes "for minor changes." Minor is where chronicle rot starts; Volume III's grievances and Volume IV's migrations both die by a thousand uncaused facts.

### The Skippable Stage

"Validation is expensive; disable it in production." The production world is the one that matters; stage 4 is the last line before a corrupt commit becomes permanent history. Optimize it; never bypass it.

---

# 3.7 Future Evolution

Future versions may add:

- speculative evaluation with rollback (behind equivalence proofs)
- incremental evaluation (systems consuming committed change-sets since their last run)
- per-region tick sharding (Chapter 5's streaming marriage)
- formally verified resolution kernels

The pipeline they optimize is the one specified here — promises first, throughput second.

---

## Preparing for the Next Chapter

The machine keeps promises only if nothing in it wobbles.

The next chapter hunts the wobble: **Determinism** — streams, ordering, floating point, and the discipline that makes "same seed, same world" a tested property instead of a hope.

---

# END OF CHAPTER 3
