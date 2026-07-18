# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 5
# Scale: Parallelism, Streaming, Networking

> *Scale is the art of doing less, elsewhere, at the same time — without anyone being able to tell.*

---

# Chapter Overview

Cardinal scales in three directions, and this chapter's thesis is that they are one problem wearing three coats:

**Parallelism** — more cores on one tick.
**Streaming** — more world than memory, via locality.
**Networking** — more machines observing and inhabiting one world.

The unifying law is inherited from Chapters 3 and 4 and repeated because everything here answers to it: every scaling mechanism must be *observationally invisible*. Same commits, same chronicle, same hashes. Scale changes throughput and residency; it never changes reality.

The unifying asset is inherited from the whole specification: Cardinal's discipline — declared read/write sets, committed-state reads, locality of rules, aggregate truth — happens to be *exactly* the discipline parallel and distributed systems beg for. The architecture did not add these for scale; scale is where they pay out.

---

# 5.1 Parallelism

## Where Parallelism Lives

Stage 2 of the tick (evaluation) is the prize: systems reading committed state and emitting proposals are pure functions, and pure functions parallelize.

Two orthogonal axes:

**Across systems.** The scheduler's DAG (Ch. 3) already encodes which systems are independent — disjoint read/write sets and no happens-before edge means concurrent evaluation. This axis is nearly free and should be the first parallelism any implementation ships.

**Within systems.** A system whose evaluation is per-entity or per-region (most are: need decay, movement, market clearing per market) parallelizes internally by *sharding its scope* — each worker takes a deterministic shard (by id range or region), produces proposals independently, and the shards' outputs concatenate in shard order.

## Keeping It Bit-Identical

The mechanisms that make Chapter 4's Door 5 hold:

- **Deterministic decomposition.** Shards are content-defined (id ranges, region ids), never "whatever each worker grabbed." Worker count changes throughput, not shard identity.
- **Order-restoring merge.** Proposals carry their sort keys (Ch. 3); resolution consumes them in key order regardless of arrival order. The race is run, and then its results are alphabetized.
- **Keyed streams.** RNG scope keys (Ch. 4) already make draws position-independent — the deer's roll is the deer's, on any worker.
- **No shared mutable anything.** Workers share the committed snapshot (immutable) and nothing else. Proposal buffers are per-worker, merged at the barrier.

Resolution and commit (stages 3–5) remain logically serial — they are cheap (sorting and applying) and their serialism is what the equivalence proof leans on. Parallelizing them is possible (parallel reduction per fact group) but is an optimization to *prove*, not assume.

---

# 5.2 Streaming

Streaming is the memory dimension: worlds larger than residency, held honest by the aggregate machinery the specification already owns.

The load-bearing insight — Volume III Chapter 10's individuation seam and Volume IV Chapter 4's lazy actualization *are* the streaming design:

```text
Region residency ladder:
  COLD      — persisted, aggregate facts only        (a distant duchy)
  AGGREGATE — in memory, aggregate facts simulated   (populations, stocks,
              settlement summaries — cheap systems run against these)
  DETAILED  — fully actualized entities              (where attention is)
```

Rules of the ladder:

- **Aggregate truth is always resident.** No region is ever "not simulated" — cold regions still advance through aggregate cadences on load (catch-up is deterministic: elapsed ticks × aggregate rules). The world never freezes off-screen; Volume I's second directive holds at every rung.
- **Transitions are the seam contract.** Detail→aggregate summarizes deterministically (and the books balance — deaths, stocks, holdings reconcile). Aggregate→detail actualizes positionally (Vol. IV Ch. 4: keyed to the region, not to when asked). Round-tripping a region through the ladder is observationally inert.
- **Boundaries stay honest.** Cross-region flows (caravans, migrations, rumors, armies) are facts at the boundary, honored on actualization. The caravan that entered a cold duchy arrives — its journey computed at aggregate fidelity, its consequences real.
- **Residency is policy, truth is not.** What is detailed follows attention (perspectives, tools, hot phenomena); *policy* is tunable and observable (Ch. 8 meters), but no policy choice may alter any committed fact — only the cost of computing it.

---

# 5.3 Networking

Multiplayer and remote observation, built on the only foundation that makes them cheap: **determinism makes state transfer almost free.**

## The Lockstep Core

The reference model is deterministic lockstep:

```text
what crosses the wire:  inputs, not world state
  — player actions (Vol. II's INTERPRET output: validated Action objects)
  — tick advancement tokens
  — periodic state hashes (Ch. 4) for divergence detection

each peer/server:       runs the identical engine, identical package,
                        identical seed → identical reality
```

Bandwidth is measured in actions per tick, not facts per tick — a world of millions of facts synchronizes on kilobytes, because the world is *recomputed*, not transmitted. This is the payoff Chapter 4 promised; without Level-2 determinism, this model is impossible and networking regresses to state replication at five orders of magnitude the cost.

## Topology and Authority

The reference topology is **server-authoritative lockstep**: one authoritative simulation; clients submit intents, receive tick confirmations plus the event/observation feed their perspectives are entitled to. Peer-to-peer lockstep is viable for small trusted sessions; the server model is specified because of the next section.

## Information Discipline on the Wire

The subtle obligation, and Cardinal-specific: **a client may receive only what its bound perspective could observe** (Vol. II Ch. 4; Vol. IV Ch. 5's binding). Full-state lockstep on clients would hand every fog-of-war secret to a packet inspector. The reference design therefore splits:

- the *server* runs full reality;
- each *client* receives its perspective's observation stream (plus enough public state for presentation);
- clients needing local prediction run a *belief-state simulation* — openly a simulation of the client's knowledge, never authoritative.

This is Volume II's reality/information split, recapitulated as network architecture — the rare case where the philosophy chapter directly prevents a cheating scandal.

## Latency, Joins, Departures

Standard lockstep craft, constrained by the specification: input delay windows or server tick batching for latency; joins are a snapshot transfer (Ch. 7's format) plus catch-up inputs; departures are events (a perspective unbinds; the world, as always, does not flinch). Hash checks per interval convert any divergence into a named, diagnosable event (Ch. 4's alarm) rather than silent drift.

---

# Designer Note
## The Village Does Not Care Where It Is Computed

The test of this chapter is a thought experiment worth running against every mechanism in it:

A village in Thornwall's north. In run A it sits in a detailed region on a single-threaded engine. In run B it is evaluated by worker 7 of 32. In run C it spends a decade at aggregate fidelity while attention lives elsewhere, then actualizes. In run D it is computed on a server while three players watch from three continents.

Same seed, same package: the village's chronicle — its marriages, harvests, feuds — must be *identical in all four runs*, and a historian diffing the saves must find nothing.

Every mechanism this chapter admits (shards, ladders, lockstep) is admissible precisely because it can pass that test, and every tempting mechanism it rejects (grab-order work queues, freeze-and-fake off-screen regions, state-replication sync) is rejected because it cannot. Scale earns its keep only when the village cannot tell.

---

# 5.4 Common Queries

- What was evaluated in parallel this tick, on how many workers, at what merge cost?
- What is each region's residency rung, and what did transitions cost this session?
- What aggregate catch-up ran for this region on actualization, over how many ticks?
- What is the wire cost per tick, per client, and what entitled each sent observation?
- Where did peers last hash-agree, and if diverged, at which tick and system?

---

# 5.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. All scaling mechanisms are observationally invisible: identical commits, chronicle, and hashes.
2. Parallel decomposition is content-defined; worker count never influences results.
3. Merges are order-restoring; resolution consumes proposals in key order always.
4. Every region advances — aggregate rungs simulate, and catch-up is deterministic.
5. Ladder transitions balance the books both directions and are positionally deterministic.
6. Cross-boundary flows are honored regardless of either side's residency.
7. Networking transmits inputs and entitled observations, never raw full state to clients.
8. Client-visible data is bounded by perspective observation rights, enforced server-side.
9. Divergence detection (hash intervals) is mandatory in every networked session.
10. Residency and scheduling policy are tunable and observable; committed truth is neither.

---

# 5.6 Anti-Patterns

### The Work-Stealing Tick

Dynamic load balancing that lets grab order shape shard contents. Steal *shards*, never *items* — decomposition is content-defined or determinism dies.

### The Frozen Duchy

Off-screen regions paused "for performance," with catch-up faked on arrival. Volume I's second directive has no residency exception; aggregate simulation exists precisely so no region ever stops.

### Sync by State Blast

Replicating fact deltas to all clients because it is easy. It is also a fog-of-war leak, a bandwidth catastrophe, and an admission that determinism was not defended.

### The Trusting Server

Accepting client-computed outcomes ("my attack hit"). Clients submit *intents*; the server's simulation decides — Volume II's VALIDATE step is a security boundary the moment a network exists.

### Prediction Bleed

Client-side belief simulation leaking into authoritative state or UI truth-claims. Prediction is information architecture (possibly wrong, clearly labeled); the commit stream is reality.

---

# 5.7 Future Evolution

Future versions may add:

- distributed simulation (region shards across machines, seam-consistent)
- interest management refinement (per-perspective observation compilation)
- rollback-based latency hiding for interactive combat, under equivalence proofs
- read-replica worlds for analytics and spectating at scale

Three coats, one law: the village must never be able to tell.

---

## Preparing for the Next Chapter

Everything so far moves facts. Something must also carry *meaning* between the engine's parts:

**Events and Messaging** — why reality is the only bus, what the chronicle's machinery owes the historian, and how services subscribe to a world without ever touching it.

---

# END OF CHAPTER 5
