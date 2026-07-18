# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 1
# The Shape of the Engine

> *Structure is the only documentation everyone is forced to read.*

---

# Chapter Overview

Four volumes of philosophy, contracts, domains, and packages now face the only question they cannot answer for themselves:

**What do you actually build?**

Volume V is the construction volume. Its posture, set by the roadmap and held throughout: *discuss tradeoffs rather than dictate implementations.* Where this volume names a specific technique, it is a recommendation with reasons; where it names an invariant, it is restating law from Volumes I–IV that no implementation choice may bend.

One clarification before anything else, because it governs every chapter here: **this volume describes the reference architecture, not the current codebase.** Cardinal has a working engine today — it has earned real scars, and this volume cites them where they validate a pattern. But where a more robust or more modular structure exists than the one currently built, this volume specifies the better structure without apology. Existing code is evidence, never precedent.

This first chapter defines the engine's gross anatomy: its layers, its module law, its bootstrap sequence, and the project layout that makes the architecture visible in a directory listing.

---

# 1.1 The Layer Model

The engine is four layers, with dependencies pointing strictly downward:

```text
┌────────────────────────────────────────────────┐
│  FRONTENDS      CLI, narrator clients, tools,  │
│                 dashboards, network servers    │
├────────────────────────────────────────────────┤
│  SERVICES       persistence, validation,       │
│                 observability, replay,         │
│                 package loading                │
├────────────────────────────────────────────────┤
│  DOMAINS        physical, living, resources,   │
│                 economy, society, culture,     │
│                 knowledge, conflict,           │
│                 institutions, ecology          │
├────────────────────────────────────────────────┤
│  KERNEL         reality store, scheduler,      │
│                 tick machinery, RNG streams,   │
│                 event log, identity            │
└────────────────────────────────────────────────┘
```

**The kernel** is Volume II made executable: facts, ticks, commits, determinism. It knows nothing of deer, prices, or grudges — it knows fact types, proposals, and streams. The kernel is small by intention; every line in it is load-bearing for every world that will ever run.

**Domains** are Volume III made executable: one module per domain, each containing its fact-type schemas, its systems, and its composition validators. Domains depend on the kernel and *never on each other* — Chapter 12 of Volume III is enforced here as a build-level rule, not a code-review hope.

**Services** operate on the simulation from outside the tick: they persist committed reality, validate packages, record telemetry, drive replays. Services may read everything and simulate nothing.

**Frontends** are consumers: the CLI, the narrator, the historian's tooling, a future network server. Frontends touch services and public kernel queries only.

## The Dependency Law

Four rules, mechanically enforceable, non-negotiable:

1. Dependencies point downward only. A kernel that imports a domain is corrupt; a domain that imports a frontend is absurd; both are build failures.
2. Domains never import domains. Cross-domain effect is committed reality, full stop.
3. Nothing imports world content. Volume IV Chapter 1's neutrality promise, at link level: packages arrive through the loader as data, or not at all.
4. Every layer's public surface is explicit. What a layer exports is a declared interface; reaching past it is a build failure, not a convention.

The current engine's oldest hard rule — `engine/` never imports from `worlds/` — is rule 3's ancestor and remains binding. The full four-rule law is where the architecture should stand.

---

# 1.2 The Bootstrap Sequence

Engine startup is a fixed, inspectable pipeline — no lazy side-effect initialization, no import-order magic:

```text
1. KERNEL UP      — reality store, identity service, RNG root: constructed empty
2. DOMAINS REGISTER — each enabled domain registers fact schemas, systems
                      (with read/write sets and cadence), composition validators
3. PACKAGE LOAD   — the sealed package (Vol. IV Ch. 3) is loaded, validated
                     against registered schemas, bound to vocabulary
4. WORLD BIND     — a save is opened (or generation runs, Vol. IV Ch. 4);
                     validation layers 4–5 pass; provenance is checked
5. SCHEDULE BUILD — the scheduler derives execution order from declared
                     dependencies and read/write sets (Ch. 3 of this volume)
6. SERVICES ATTACH — persistence, telemetry, replay observers subscribe
7. RUN            — ticks begin; from here, every change is a proposal
```

Two properties matter more than the step count:

**Registration is data.** Step 2 makes domains *discoverable* rather than hardwired: the domain list is configuration, which is what makes Volume IV's domain selection (Pelagia disabling half the matrix) a load-time decision instead of a build variant.

**Failure is front-loaded.** Steps 3–5 are where every Volume IV validation layer fires. An engine that reaches step 7 has a valid world; anything wrong after that is a bug, not a bad input.

---

# 1.3 Project Layout

Structure should read as the architecture. The reference layout:

```text
cardinal/
├── kernel/          — reality store, tick, scheduler, rng, events, identity
├── domains/
│   ├── physical/    — schema.*, systems/, composition.*
│   ├── living/
│   └── …            — one directory per Volume III domain
├── services/
│   ├── persistence/ — snapshots, chronicle store, migrations
│   ├── packages/    — loader, composition pipeline, validators
│   ├── observe/     — telemetry, meters, causal-chain queries
│   └── replay/
├── frontends/
│   ├── cli/
│   ├── narrator/
│   └── tools/
├── worlds/          — world packages (content, never code)
└── tests/
    ├── kernel/      — determinism, commit, stream tests
    ├── domains/     — per-domain contract tests
    ├── probes/      — minimal exotica worlds (Vol. IV Ch. 7)
    └── reference/   — signature suites for the five worlds (Vol. IV Ch. 8)
```

The layout is a recommendation; the *visibility* is not. Whatever the language and build system, a newcomer must be able to find the kernel, enumerate the domains, and locate the dependency law's enforcement within their first hour. An architecture that requires a guided tour to perceive is already eroding.

---

# Designer Note
## Small Kernel, Heavy Law

The strongest structural decision available to Cardinal is keeping the kernel *small and finished* rather than large and helpful.

Every convenience added to the kernel — a pathfinding helper, a "common" combat utility, a default anything — is a bet that all future domains and worlds want it, and Volume IV Chapter 1 already taught how those bets end (the engine has never heard of a sword). The kernel's job is to make the law of Volumes I–II executable: atomic ticks, owned facts, deterministic streams, append-only history. That is perhaps a few thousand lines in any serious language, and it should asymptotically stop changing.

Growth belongs in domains (new kinds of reality, by Chapter 12's checklist), services (new ways to operate worlds), and frontends (new ways to consume them). A kernel that is still gaining features in year three is a kernel absorbing someone's domain — and the engine's future flexibility is what it is eating.

---

# 1.4 Common Queries

Structural questions the build itself should answer:

- What does each layer export, and who imports it?
- Which domains are registered in this engine build, at what versions?
- Where is the dependency law enforced, and when did it last fire?
- What does the bootstrap sequence log at each step, and where did a failed boot stop?
- Which kernel interfaces are public to frontends versus internal to services?

---

# 1.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. The engine is layered kernel → domains → services → frontends, dependencies downward only.
2. Domains never depend on domains; the prohibition is build-enforced.
3. No engine code imports world content; packages arrive only through the loader.
4. Domain registration is data-driven; domain selection is a load-time decision.
5. The bootstrap sequence is fixed, ordered, and inspectable.
6. All package and world validation completes before the first tick.
7. Layer surfaces are explicit interfaces; bypass is a build failure.
8. The kernel contains no domain knowledge and no world knowledge.
9. Services cannot mutate simulation state; they observe and operate around it.
10. Project structure makes layers, domains, and their law discoverable by inspection.

---

# 1.6 Anti-Patterns

### The Helpful Kernel

Utilities migrating kernel-ward "because everyone needs them." Every one is a future domain's constraint. The kernel executes law; it does not help.

### The God Module

A `core/` or `common/` that everything imports and nothing owns. Shared code either belongs to a layer with a declared surface or it is the dependency law dissolving in real time.

### Convention-Only Boundaries

"We just know not to import across domains." Boundaries that live in culture die in deadlines. If the build cannot refuse it, the architecture does not forbid it.

### Bootstrap Sprawl

Initialization scattered through lazy singletons and import side effects. A boot that cannot be printed as a numbered list cannot be debugged as one either.

### The Embedded Frontend

Narrator or CLI concerns leaking into domains ("format this nicely for display"). Presentation is vocabulary and frontends; a domain that knows about prose is a domain that will someday lie for it.

---

# 1.7 Future Evolution

Future versions of this architecture may add:

- multiple kernel profiles (single-threaded, parallel, distributed — Chapters 3 and 5)
- out-of-process domains behind the same registration contract
- embeddable engine builds (library form for tools and tests)
- formal interface definitions checked across language boundaries

The layer law survives all of them unchanged — which is the point of having it.

---

## Preparing for the Next Chapter

The shape is set. The first thing to build inside it is the thing everything else touches:

**Representing Reality** — the reality store, the honest tradeoffs between entity-component, relational, and fact-oriented designs, and how memory layout serves truth without ever defining it.

---

# END OF CHAPTER 1
