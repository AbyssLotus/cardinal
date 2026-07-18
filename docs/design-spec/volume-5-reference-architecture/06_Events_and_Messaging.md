# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 6
# Events and Messaging

> *Inside the simulation there is no mail. There is only the world.*

---

# Chapter Overview

Most engines have a message bus, and most simulation bugs of a certain flavor live on it: the event fired twice, the listener that ran before its data existed, the cascade nobody can reconstruct.

Cardinal's position, held since Volume II and made executable here, is that *within the simulation there is no messaging at all.* Systems do not signal systems. Domains do not notify domains. There is exactly one way meaning moves inside the tick — committed reality — and exactly one record of what happened — the chronicle.

Messaging *does* exist in the architecture, but only at the boundary: services and frontends subscribing to a world they can never touch. This chapter builds both halves: the chronicle machinery (the event system that is actually a history), and the observation plane (the messaging that is actually a broadcast).

Keeping the two halves apart is the chapter's entire discipline.

---

# 6.1 Events Are History, Not Control Flow

The distinction that governs everything here:

In a conventional engine, an event is *imperative* — `on_death → drop_loot()` — a hidden function call wearing a costume. Firing order matters, listeners matter, and the call graph is invisible.

In Cardinal, an event is *declarative history*: a committed record that reality changed, assembled from proposal causes (Ch. 3) at stage 6:

```text
Event #88121
├── type: settlement.seizure
├── tick: Y203.spring.d14.t332
├── participants: [clan_vess, milbrook, meadow.east]
├── deltas: [the committed facts this event explains]
└── causes: [event #87455, …]              # upstream chain
```

Nothing *listens* to this event. Nothing fires because of it. Systems that care about seizures (Conflict's grievance accrual, Society's reputation) will read committed reality — including committed event records — on their own cadence, next tick, through their declared read sets, and propose accordingly.

The consequences run deep:

- **No ordering problem exists.** There are no listeners to order. Effects happen when their systems run, against a consistent committed world.
- **No cascade problem exists.** A "chain reaction" is just successive ticks doing ordinary work — each link committed, chronicled, and inspectable (Vol. IV Ch. 12's effect chains).
- **The call graph is the chronicle.** *Why did this happen* is a data query, not a debugging session. Volume III Chapter 11's causal audit is not tooling bolted on; it is what events *are*.

## The Chronicle Store

The event half of the reality store (Ch. 2's warm/cold tiers), with its own contract:

- **Append-only, always.** Events are immutable; correction is a new event referencing the old (Vol. II Ch. 5).
- **Causally linked.** The `causes` field makes the chronicle a DAG; chain queries (ancestry, descendants, cross-domain traces) are first-class store operations.
- **Indexed for the historian.** By tick range, type, participant, and region — because Volume III made grievances, reputation, lore, and legitimacy *query* this store in production, not just in postmortems.
- **Compactable, never amputable.** Era summarization with retained referents (Vol. IV Ch. 6): a millennium-old event may be summarized, but anything still *referenced* — by a standing grievance, a lore entry, a save's migration record — remains resolvable forever.

---

# 6.2 The Observation Plane

Outside the tick, messaging is legitimate — and strictly one-directional.

```text
                    ┌──────────────┐
   commit(N) ──────►│ OBSERVATION  │──► persistence   (Ch. 7)
   + events(N)      │    PLANE     │──► telemetry     (Ch. 8)
   (read-only       │  (fan-out)   │──► narrator feed (Ch. 9)
    views)          └──────────────┘──► network clients (Ch. 5)
                                    ──► tools, replays, dashboards
```

The plane's rules:

1. **Sources are commits.** The plane fans out exactly two things: the committed batch and its events, as read-only views. There is no other publication and no pre-commit peeking.
2. **Subscribers are powerless.** No subscriber can block, delay, or influence a tick. Slow consumers get buffering, snapshots-and-catch-up, or disconnection — never backpressure into the simulation. (The one nuance: *durability* subscribers like persistence may gate *acknowledged* progress for operational reasons — Chapter 7 — but even they cannot alter a committed tick, only pause consumption of the next.)
3. **Entitlement is enforced at the plane.** Perspective-bound consumers (clients, narrator) receive their observation streams (Vol. II Ch. 4; Ch. 5's wire discipline) — the plane is where "what may this consumer see" is computed, once, server-side.
4. **Delivery is deterministic in content.** Timing of delivery may vary; *what* a subscriber with given entitlements receives for tick N is a pure function of the commit — so a replay can regenerate any consumer's exact feed.

## Commands Come In the Front Door

The reverse direction — the CLI's `attack`, a tool's query, a client's intent — is not messaging into the simulation either. It is *input*: interpreted, validated, and queued as actions for the next tick's INTERPRET stage (Vol. II's pipeline; Ch. 5's server authority). There is one front door, it opens once per tick, and everything that enters becomes proposals like everything else.

---

# Designer Note
## The Bus You Don't Build Is the Bug You Don't Have

The event-bus instinct dies hard, so it is worth rehearsing what the bus would cost against what it buys.

It buys immediacy: loot drops the instant death fires. Cardinal's answer costs one tick of latency — death commits at N; scavenging, inheritance, and grief propose at N+1 — and buys, in exchange: no listener-ordering semantics to specify, no reentrancy, no cascade storms inside a tick, no hidden coupling between domains (the bus is where Volume III Chapter 12's "no domain knows another exists" would quietly die), and a complete causal record *as a side effect of the architecture*.

One tick is also the honest physics of the specification: Volume II defined the tick as the quantum of change. An effect "in the same instant" as its cause is a claim the chronicle cannot represent — and anything the chronicle cannot represent, Volume III Chapter 11 cannot audit.

When the latency genuinely matters — death and its loot inside one combat round — the answer is composition *within* the owning domain (Conflict's engagement resolution handles both, one system, one proposal set), never a cross-system signal. If two domains need same-tick coupling, the boundary is drawn wrong, and Appendix A has a procedure for that.

---

# 6.3 Common Queries

- What events explain this fact's current value? (chain ancestry)
- What did this event cause, transitively, across domains? (chain descent)
- What events touched this entity/region in this era, by type?
- What exact feed did this subscriber receive for ticks N..M? (replay regeneration)
- What entitlements produced this client's stream, and who granted them?
- What is the chronicle's compaction state, and does every live reference still resolve?

---

# 6.4 Engineering Invariants

Every implementation SHALL preserve these rules.

1. No system, domain, or in-simulation component sends or receives messages.
2. Events are committed history assembled from proposal causes — never control flow.
3. Nothing executes *because* an event exists; systems read, on cadence, and propose.
4. The chronicle is append-only, causally linked, and indexed for production queries.
5. Compaction preserves resolvability of every referenced event, forever.
6. The observation plane fans out committed state only; no pre-commit visibility exists.
7. No subscriber can influence simulation content; durability gating pauses progress, never alters it.
8. Entitlement filtering is computed at the plane, server-side, per perspective.
9. Any subscriber's feed is deterministically regenerable from the save.
10. All external influence enters as validated input through the tick's front door.

---

# 6.5 Anti-Patterns

### The Costumed Callback

An "event" whose handlers run synchronously at emission. That is a function call with extra steps and hidden ordering; the chronicle gains nothing and the tick's atomicity leaks.

### Listener Archaeology

Any design where understanding behavior requires enumerating subscribers. In Cardinal the question "what reacts to a seizure?" has a static answer: whichever systems declare event-reads of that type — in their manifests, not in runtime registration order.

### The Backchannel

A service that "just this once" writes a flag the simulation reads. The observation plane's one-way arrow is the whole security model of Chapters 5, 8, and 9; a single backchannel voids all three.

### Chronicle as Log File

Treating events as diagnostics — unindexed, uncaused, trimmed at will. The chronicle is production data with production queries (grievances! legitimacy! lore!); its loss is world corruption, not disk savings.

### Same-Tick Heroics

Contorting the resolver to make cross-domain effects land in their cause's tick. The quantum is the tick; effects follow causes. Where that is intolerable, the two effects belong to one domain — fix the boundary, not the physics.

---

# 6.6 Future Evolution

Future versions may add:

- chain-query language for historians and tools (Vol. III Ch. 11's ask)
- event-type schemas with per-domain registries and versioning
- tiered chronicle storage with transparent summary/detail resolution
- standing subscriptions with durable cursors for offline tools

The two halves stay two: history inside, broadcast outside, and never a wire between systems.

---

## Preparing for the Next Chapter

The observation plane's most important subscriber has been waiting three chapters:

**Persistence Machinery** — snapshots, the chronicle store's durable form, serialization discipline, and the migration engineering that Volume IV's promises rest on.

---

# END OF CHAPTER 6
