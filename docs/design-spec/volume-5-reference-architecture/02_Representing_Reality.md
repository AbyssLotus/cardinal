# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 2
# Representing Reality

> *The store may be clever. The truth may not.*

---

# Chapter Overview

Volume II declared what reality *is*: entities, atomic facts with provenance, relationships, events. It deliberately said nothing about bytes.

This chapter says the byte things — as tradeoffs. The reality store is the kernel's largest component and the engine's most consequential performance surface, and there are at least four defensible ways to build it. The architecture's job is not to pick one forever; it is to define the *interface all of them must satisfy*, so the choice can be revisited in year five without touching a single domain.

That interface — the Reality Store contract — is this chapter's fixed point. Everything after it is engineering judgment, argued honestly.

---

# 2.1 The Reality Store Contract

Whatever its internals, the store presents one logical model:

```text
Store operations (kernel-internal):
  resolve(id) → entity | tombstone
  read(entity, fact_type) → value + provenance
  query(pattern) → matching facts          # by type, owner, region, relation
  relate/unrelate via fact writes          # relationships are facts
  apply(commit_batch)                      # the ONLY mutation path
  snapshot(tick) → consistent view
  history(entity | fact | event pattern) → chronicle slice
```

Six clauses give it teeth:

1. **One mutation path.** All change enters through `apply()`, which accepts only resolved commit batches from the tick machinery (Chapter 3). No other write API exists — not for tools, not for tests, not for migrations (which run as their own recorded batches).
2. **Provenance is not optional.** Every fact answers *who, when, why* (source system, tick, causing event). A store layout may compress provenance cleverly; it may never drop it.
3. **Reads are of committed state.** During a tick, systems read the last committed world — the store never exposes in-flight proposals as truth.
4. **Identity is permanent.** Ids are never reused; deletion is tombstoning; tombstones resolve (Volume II's identity invariant, and Volume IV Chapter 6's migration dependency).
5. **Queries are the product.** Volume III Chapter 1's read-mostly principle: the store is optimized for *asking*, and the query surface (spatial, relational, historical) is part of the contract, not a bolt-on.
6. **Snapshots are consistent.** Any snapshot is a whole committed tick — persistence, replay, and observers never see a torn world.

---

# 2.2 The Design Space

Four families, honestly compared:

## Entity-Component Storage (ECS)

Facts grouped into typed components, stored in contiguous arrays; systems iterate archetypes.

*For:* iteration speed is unmatched — cache-friendly sweeps over exactly the data a system declared; read/write sets map naturally onto component types; the game industry has proven it at scale.

*Against:* classic ECS is built for *frame simulation*, and three of its instincts must be overridden: components rarely carry provenance (clause 2 must be added), mutation is typically in-place (clause 1 must be imposed — proposals, not pokes), and history is foreign to it (the chronicle must be first-class, not an afterthought). ECS relationships (entity-to-entity graphs) are also its weakest suit, and Cardinal is graph-heavy.

*Verdict:* strong candidate for the hot layer — with the caveat that what Cardinal needs is ECS *storage discipline*, not ECS *architecture*: systems still go through propose/commit, never direct component writes.

## Relational Storage

Facts as rows; SQL or SQL-like engine; the current implementation's home ground (SQLite with atomic per-turn transactions).

*For:* transactions are native (clause 1 and 6 nearly free); ad-hoc query power is enormous; durability and tooling are mature; referential integrity is enforceable in the store itself. As proof of adequacy: the entire current engine, six milestones of it, runs on this profile.

*Against:* per-tick performance ceilings arrive early — row round-trips in hot loops, ORM temptations, and the impedance mismatch of graph traversals in joins. Fine at village scale; strained at Meridian's five million.

*Verdict:* excellent durable layer and small-world profile; unlikely to be the hot layer at reference-world scale.

## Fact/Triple Store

The literal Volume II model: `(entity, fact_type, value, provenance)` as the atom, indexed several ways; kin to RDF and Datomic-style designs.

*For:* perfect fidelity to the spec — provenance, history, and queryability are the *native* shape, not adaptations; schema evolution is gentle (new fact types are just new rows); time-travel reads fall out of the design.

*Against:* the least cache-friendly layout of the four; naive implementations pay a pointer-chase per fact; write amplification on busy entities is real.

*Verdict:* the best *semantic* reference — and the recommended mental model for the contract itself — with performance demanding a hybrid beneath it at scale.

## Hybrid (Recommended Reference Design)

Tiered representation behind the single contract:

```text
HOT   — current committed tick: ECS-style typed columns for
        system iteration; graph adjacency for relations
WARM  — recent ticks + provenance detail: append-only fact log
COLD  — chronicle + snapshots: persistence formats (Ch. 7)
```

Systems iterate the hot tier at array speed; `apply()` writes hot and appends warm; snapshots and history serve from warm/cold. The fact model remains the *logical truth*; the tiers are how it goes fast. All clause obligations (provenance, consistency, single path) are enforced at the contract, so tiers can be rebalanced — or a tier's technology replaced — invisibly.

---

# 2.3 Memory Management

Principles, since mechanisms are language-dependent:

**Arena by tick.** Proposal and scratch allocations live in per-tick arenas, freed wholesale at commit — no simulation garbage survives a tick boundary, and allocation cost stops being a per-fact tax.

**Stable storage for identity.** Entities and facts live in structures whose *logical* addresses never move (handles/indices, not raw pointers), because half the engine — chronicle, saves, tools — holds long-lived references.

**Aggregates are the memory model.** Volume III Chapter 10's individuation and Volume IV Chapter 4's lazy actualization are the real memory strategy: most of the world, most of the time, is aggregate facts, and the store must make region-level residency explicit and cheap rather than heroically caching individual entities.

**Budget observability.** Memory per region, per domain, per tier is a first-class meter (Chapter 8) — a world that cannot report where its bytes went cannot be scaled honestly.

---

# Designer Note
## The Store Is a Servant With One Master

Every storage technology arrives with an architecture it would prefer you had. ECS wants systems to poke components. SQL wants logic in queries. Document stores want denormalized truth. Graph databases want traversal to be the program.

The discipline of this chapter is that Cardinal's architecture was fixed three volumes ago, and the store *serves* it: one mutation path, committed reads, permanent identity, provenance always. The store's master is the contract — its internals may be as clever, tiered, and technology-specific as performance demands, precisely *because* no cleverness can leak past `apply()` and `query()`.

The current engine proves the contract's portability in one direction (a relational profile carried six milestones). The hybrid reference design is the same contract facing the other direction — toward Meridian. Neither is the truth. The contract is.

---

# 2.4 Common Queries

- What is this fact's value, provenance, and history?
- Which entities match this pattern in this region? (spatial index)
- What relates to this entity, by which relationship facts? (graph adjacency)
- What did this snapshot look like at tick T? (time travel)
- Which tier holds this data, and what did promotion/demotion cost this tick?
- Where is memory going, by region, domain, and tier?

---

# 2.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. All mutation flows through `apply()` on resolved commit batches; no second write path exists.
2. Every fact carries provenance; no layout may shed it.
3. Systems read only committed state.
4. Identity is permanent; ids are never reused; tombstones resolve.
5. Snapshots are whole-tick consistent, always.
6. The logical model is the fact model; storage layouts are private to the store.
7. Store internals may change without any domain, service, or frontend changing.
8. Relationship queries and spatial queries are contract surface, not add-ons.
9. Aggregate representation and residency are explicit store states, not cache accidents.
10. No store optimization may weaken determinism (Chapter 4 reads this clause twice).

---

# 2.6 Anti-Patterns

### Direct Component Writes

The ECS reflex: systems mutating storage in iteration. One poke bypasses proposals, and the tick's atomicity — and everything Volume II built on it — is fiction.

### Provenance on a Diet

"We dropped the provenance columns in the hot tier for speed." Chapter 11 of Volume III dies first (no causal audit), migration honesty (Vol. IV Ch. 6) dies second. Compress it; never shed it.

### The ORM Simulation

Domain logic drifting into store queries ("the market system is mostly SQL now"). Systems evaluate rules; stores answer reads. Logic in the store is logic outside the scheduler, the read/write sets, and the determinism regime.

### Pointer Truth

Raw in-memory references as identity. The first defragmentation, reload, or tier migration corrupts every long-lived reference the engine holds.

### The Cache With Opinions

Derived-value caches inside the store that "help" by persisting (Volume II's derived-data rule inverted). Caches are rebuildable, invisible, and owned by the store — the moment one is observable as truth, there are two truths.

---

# 2.7 Future Evolution

Future versions may explore:

- columnar chronicle compression with intact referents
- immutable/persistent data structures for zero-copy snapshots
- NUMA-aware hot-tier partitioning (feeding Chapter 5)
- store-level differential snapshots (feeding Chapter 7)
- formal store conformance suites, so alternate stores certify against the contract

All of it behind `apply()` and `query()`, where cleverness belongs.

---

## Preparing for the Next Chapter

Reality can now be held. The next chapter makes it move:

**The Simulation Kernel** — the tick pipeline, the scheduler that turns declared read/write sets into execution order, proposal resolution, and the machinery that makes Volume II's promises executable.

---

# END OF CHAPTER 2
