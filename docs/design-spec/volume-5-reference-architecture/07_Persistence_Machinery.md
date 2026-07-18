# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 7
# Persistence Machinery

> *The disk is where the world goes to survive you.*

---

# Chapter Overview

The persistence *contract* is finished business: Volume II Chapter 5 defined what must survive; Volume IV Chapter 6 defined versioning, pins, and migration honesty. Nothing in this chapter revises a word of either.

This chapter is the machinery: how a persistence service actually captures committed reality without stalling it, what the durable formats owe the decade-old save, how serialization stays honest across engine generations, and how migrations execute as engineering rather than aspiration.

The service sits where Chapter 6 put it — a subscriber on the observation plane, powerful in exactly one dimension (durability) and powerless in every other (content).

---

# 7.1 The Write Path

```text
commit(N) ──► observation plane ──► persistence service
                                        │
                                        ├── event append   (every tick: the
                                        │   chronicle's durable tail — cheap,
                                        │   append-only, sequential)
                                        │
                                        └── snapshot       (periodic: full or
                                            differential state capture)
```

**Events every tick, snapshots on policy.** The durable chronicle *is* the write-ahead log: with snapshot S and events S..N, reality at N is reconstructable two independent ways — replay (re-simulate from S; Ch. 4 guarantees identity) or restoration (apply the recorded deltas). That redundancy is not waste; it is the recovery model *and* the standing determinism audit (a restoration that disagrees with a replay has found a bug somewhere precious).

**Capture without stalls.** Snapshots read a consistent tick view (Ch. 2's snapshot clause). The reference technique is copy-on-write capture: mark the committed tick, let simulation proceed, stream the marked view out behind it. Implementations without CoW stores fall back to brief pause-and-copy at snapshot cadence — a policy tradeoff (Ch. 8 meters it), never a correctness one.

**Durability acknowledgment.** The service tracks the *durable frontier* (last tick safely on disk). Operational policy may gate how far simulation runs ahead of it (Ch. 6's one nuance). Crash recovery is then mechanical: load last snapshot, apply/replay events to the frontier, resume — and recovery is *tested*, not assumed (kill-and-recover runs belong in CI beside the twin runs).

---

# 7.2 Formats

The format family serves three masters — write speed, archival longevity, migration tractability — and no single encoding serves all three. The reference design separates them:

```text
save/
├── manifest        — provenance (Vol. IV Ch. 6): pins, versions, seeds,
│                     migration record; small, human-inspectable
├── snapshots/      — state captures: schema-tagged, compressed,
│                     internally chunked by region and domain
├── chronicle/      — append-only event segments + indexes
│                     (tick, type, participant, region; Ch. 6)
└── integrity       — per-segment checksums + per-tick state hashes (Ch. 4)
```

Format law, mostly inherited and here made operational:

- **Self-describing.** Every segment carries its schema version. A reader confronted with any file from any era knows *exactly* what it holds or refuses with names and numbers (Vol. IV Ch. 6's articulate refusal).
- **Decoupled from memory.** The wire/disk schema is its own declaration — never a dump of in-memory structs (Vol. II Ch. 5's engine-coupled-formats mistake). The store's tiers (Ch. 2) may be rebuilt wholesale; the save format survives.
- **Chunked for partiality.** Region- and domain-chunked snapshots serve streaming (Ch. 5's cold rungs load per region), migration (transform chunk-wise, bounded memory), and repair (one corrupt chunk is one lost chunk, not a lost world — checksums localize it).
- **Compressed, losslessly, semantically inertly.** (Vol. II Ch. 5.) Compression choices are per-segment and recorded; archival re-compression is legitimate maintenance.
- **State, never derivation.** What is absent is as specified as what is present: no caches, no indexes-as-truth, no derived aggregates (rebuilt on load), no presentation state.

## Serialization Discipline

The schema registry is the keystone: every persisted fact type and event type registers its serialization schema *in the domain that owns it* (Ch. 1's registration step), versioned independently of engine releases. Serialization code is generated or derived from the registry — hand-rolled encoders drift; registries do not. The registry's history is itself versioned, because it is precisely what a format migration (next section) consults to read the past.

---

# 7.3 Migration Machinery

Volume IV Chapter 6 defined two migration kinds and their debts. Their engines:

**Format migrations** are chunk-streaming transforms: read segments at schema N, emit at N+1, sequentially composable (1→2→3→4), each transactional per Volume IV (complete or untouched, then recorded in the manifest). Because segments are chunked, migration memory is bounded and interruptible-with-resume is cheap. Every format migration ships with round-trip tests against archived fixture saves from each supported era — the fixture library *is* the compatibility promise, in executable form.

**Package migrations** run *through the engine*, not beside it: a migration is a special boot mode (Ch. 1's sequence with a migration stage after WORLD BIND) in which the package's declared transformation executes as recorded, chronicled batches through `apply()` — the same single write path as everything else (Ch. 2). No side-tool ever rewrites save files around the engine's law; the migration *is a world event* (Vol. IV Ch. 6's chronicled debt) and uses the machinery that makes events honest.

**Dry runs are the difference between surgery and gambling.** Both migration kinds run in report mode against a copy: what would change, what references resolve, what the impact profile looks like (Vol. IV Ch. 6's upgrade-preview query). The report is a reviewable artifact; the keeper of the decade-old save reads it before consenting.

---

# Designer Note
## Two Roads to the Same Tick

The write path buried a gift worth unwrapping: a Cardinal save can always reach tick N two ways — *replay* (recompute from snapshot S under determinism) and *restore* (apply recorded deltas).

Lean on it deliberately. Restoration is the fast path for loading. Replay is the audit: run it in CI against every reference save, and any disagreement between the two roads is an alarm — either determinism drifted (Ch. 4's doors), or serialization lies (this chapter's problem), or the chronicle is incomplete (Ch. 6's problem). One cheap comparison patrols three chapters' worth of promises.

The gift compounds with time: the archived fixture library plus replay means the project can *prove*, on every release, that a save from year one still opens, still restores, still replays identically. The decade-old save stops being a design metaphor and becomes a green checkmark — which is the only place metaphors are safe.

---

# 7.4 Common Queries

- Where is the durable frontier, and how far ahead is simulation running?
- What snapshot cadence and capture cost is this world paying? (Ch. 8 meters)
- Which schema versions does this save carry, per segment, and which readers accept them?
- What would this migration change? (dry-run report)
- Do restore and replay agree for this save, at which audited ticks?
- Which fixture-era saves does this engine build certify against?

---

# 7.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Persistence subscribes to committed reality; it never writes simulation content.
2. The chronicle's durable tail is appended every tick; snapshots follow declared policy.
3. Any persisted tick is reconstructable by restore and by replay, and the two agree.
4. Capture reads whole-tick-consistent views; capture cost never alters commits.
5. Crash recovery (snapshot + tail) is deterministic and CI-tested by kill-and-recover.
6. All formats are self-describing, chunked, and decoupled from in-memory layout.
7. Serialization derives from the versioned schema registry, owned domain-by-domain.
8. Format migrations are sequential, transactional, recorded, and fixture-tested per era.
9. Package migrations execute through the engine's single write path as chronicled batches.
10. Every migration offers a dry-run report before touching an original.

---

# 7.6 Anti-Patterns

### The Struct Dump

Persisting in-memory layouts because it is one `memcpy` away. It is also one refactor away from unreadable, and the decade-old save dies of a field reorder.

### Sidecar Surgery

External scripts that patch save files directly "because booting the engine is heavy." Around the single write path lies the land of unchronicled change (Vol. IV Ch. 6's silent fixup), and nothing that lives there is trustworthy afterward.

### The Untested Fixture

Keeping old-era saves but never opening them in CI. An unexercised compatibility promise is a compatibility hope; eras rot silently until a keeper arrives with the one save that matters.

### Snapshot Worship

Snapshotting so often the chronicle tail is vestigial — then trimming events "since snapshots cover it." The chronicle is production data (Ch. 6) and the replay audit's substrate; snapshots are an *optimization* of load time, not a replacement for history.

### The Ambitious Loader

Load-time "repairs," format coercion, silent field defaults. Loading validates and refuses articulately (Vol. IV Ch. 7 layer 4); every repair is a migration wearing street clothes, and migrations have paperwork for a reason.

---

# 7.7 Future Evolution

Future versions may add:

- differential snapshot chains with periodic full anchors
- content-addressed chunk stores (dedup across saves and branches)
- branching-save ancestry sharing (Vol. IV Ch. 6's futures, made cheap)
- archival tiers with integrity-preserving recompression
- streaming restore (playable before fully loaded, cold rungs arriving behind)

The two roads remain the law: whatever the format does, restore and replay agree.

---

## Preparing for the Next Chapter

A world that survives is a world someone must be able to *see into*:

**Observability, Debugging, and Testing** — meters, time travel, causal tooling, and the test pyramid that keeps five volumes of promises measurable.

---

# END OF CHAPTER 7
