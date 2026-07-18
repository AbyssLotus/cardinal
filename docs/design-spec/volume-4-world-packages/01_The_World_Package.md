# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 1
# The World Package

> *The engine knows how worlds work. The package knows which world this is.*

---

# Chapter Overview

Volume III ended with a promise: an architecture of everything, waiting to become a simulation of *somewhere*.

The world package is how somewhere happens.

A world package is the complete, self-contained definition of a universe: its physical rules, its selected domains, its species and materials and cultures, its generation recipe, and its starting condition. The engine, by design, contains none of this. Volume I's fourth prime directive — the engine contains zero world-specific content — means that every noun in a running simulation arrived through a package.

This volume explains how radically different simulations — a medieval kingdom, a modern city, an ocean with no people in it, a colony ship, a world where gravity is a suggestion — are all *the same engine* running different packages.

This first chapter defines what a package is, what it contains, and the contract that binds it to the engine.

---

# 1.1 Purpose

The world package exists to hold a boundary absolutely:

**The engine executes rules. The package defines them.**

Everything on the package's side of the boundary is data. Everything on the engine's side is mechanism. The test, stated once and applied everywhere:

> If deleting a world would delete it, it belongs in the package.

Species, items, techniques, cultures, laws, gods, currencies, place names — deleted with the world. Package.

Ticks, commits, fact storage, the scheduler, the information pipeline, determinism — survive any world's deletion. Engine.

This boundary is what makes Cardinal an engine rather than a game. It is also what makes every chapter of Volume III honest: when Chapter 3 said "resource definitions are world-package data," this volume is where that debt is paid.

---

# 1.2 What a Package Is

A world package is a versioned, validated bundle of declarations:

```text
worlds/thornwall/
├── manifest          — identity, version, engine requirements, vocabulary
├── domains           — which of Volume III's domains run, and their composition rules
├── rules             — every tunable number and law, per domain
├── content/          — data packs: species, materials, resources, techniques,
│                       cultures, institutions, item and structure definitions
├── generation/       — how initial reality is produced (authored, procedural, hybrid)
├── scenarios/        — named starting conditions atop the generated world
└── validation/       — package-specific invariants and reference-world expectations
```

The layout is conceptual, not prescriptive — a package may be one file or ten thousand. What is prescriptive is the *separation of concerns*: identity, rules, content, generation, and scenario are distinct kinds of declaration with distinct lifecycles, and later chapters treat each in turn.

## The Manifest

The package's identity card:

```text
id: world.thornwall
version: 2.4.0
engine: ">=0.3, <0.5"
vocabulary:
  currency: "shilling"
  region_label: "March"
domains: [physical, living, resources, economy, society,
          culture, knowledge, conflict, institutions, ecology]
```

The manifest also declares the world's *vocabulary* — the human labels the narrator and tooling use for engine concepts. The engine has regions; Thornwall has Marches. The engine has money; Thornwall has shillings. Vocabulary is presentation, never semantics: renaming a currency changes no rule.

## Identity Discipline

Every declared thing carries a namespaced, stable identifier:

```text
species.red_deer
mat.limestone
res.iron_ore
tech.drystone_walling
culture.marchfolk
law.kings_road_toll
```

Identifiers are the package's contribution to Volume II's identity invariant: they are permanent, never reused, and survive renames (display names are vocabulary; ids are identity). Every cross-reference in a package resolves by id, and Chapter 7's validation refuses any package containing a dangling one.

---

# 1.3 The Package Contract

Four obligations bind every package; four bind the engine.

## The Package Promises

1. **Completeness.** Everything the world needs is declared in the package or its declared dependencies. No rule, species, or number lives anywhere else.
2. **Validity.** The package passes every validation layer (Chapter 7) before any simulation begins.
3. **Determinism.** All declared rules, rates, and generators are deterministic given the seed. A package cannot introduce nondeterminism, because it has no mechanism to — it is data.
4. **Honest declaration.** The package declares its engine version range, its dependencies, and its domain selection, and does not reach outside them.

## The Engine Promises

1. **Neutrality.** No package is privileged. The engine ships with no default world, no default species, no default anything.
2. **Sufficiency.** The declared architecture of Volumes II and III is fully available to any package — a package never needs engine changes to define a new world *within the existing domains*.
3. **Stability.** A valid package remains valid across engine patch versions; breaking engine changes are versioned and migratable (Chapter 6).
4. **Indifference to scale.** One room or one continent, the same contracts hold.

## The Extension Seam

One question decides where new work goes: **is this a new world, or a new kind of reality?**

A new species, currency, magic tradition, or legal system is a new world — package work, no engine changes, by promise 2.

A new *domain* (Volume III, Chapter 12's checklist) or a new fact-type semantics is a new kind of reality — engine work, after which every package may select it.

Packages declare *which* domains run and *how* their facts compose, but never define new fact semantics themselves. When a package's ambition exceeds the domains that exist, the answer is a domain proposal against Chapter 12, not a clever encoding.

---

# Designer Note
## The Engine Has Never Heard of a Sword

It is worth sitting with how far the neutrality promise goes.

The engine does not know what a sword is. It knows items exist, that items have material properties (Physical Reality), owners (Economy), and combat characteristics consumed by engagement resolution (Conflict). `item.arming_sword` — its mass, its edge, its reach, its name — is Thornwall's declaration.

The engine does not know what a king is: offices are Institutions' mechanism; `office.king_of_thornwall` is a declaration. It does not know what bread, prayer, winter, or wolves are.

This is uncomfortable at first — surely *some* things are universal? But every "surely" fails somewhere interesting. The ocean world has no items because nothing there has hands. The colony ship's winter is a reactor schedule. The fantasy world's wolves speak.

The discipline holds because the alternative is a thousand small defaults, each one a world some future package cannot be.

---

# 1.4 Common Queries

Asked of packages by the engine, tooling, and authors:

- What engine versions can run this package?
- Which domains does it select, and with what composition rules?
- What does this id resolve to, and what references it?
- What vocabulary does this world use for this engine concept?
- What are this package's declared dependencies, and do they resolve?
- Is this package valid, and against which validation layers?
- What changed between package versions 2.3 and 2.4, and does it break saves? (Chapter 6)

---

# 1.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. The engine contains zero world-specific content, without exception.
2. Every simulated world is produced by exactly one root package and its declared dependencies.
3. Package identifiers are namespaced, stable, and never reused.
4. Vocabulary affects presentation only, never semantics.
5. Every tunable number lives in a package, never in engine code.
6. Packages are data: they declare, and cannot execute.
7. A package declaring only existing domains requires zero engine changes to run.
8. No package is architecturally privileged.
9. Package validity is established before simulation, never during.
10. The manifest's engine-version declaration is enforced, not advisory.

---

# 1.6 Anti-Patterns

### The Convenient Default

"Every world probably has humans, so ship a human template in the engine." Every default is a colonization of package space; the ocean world now has vestigial humans to suppress.

### The Escape Hatch

Letting packages ship executable code to cover gaps in declaration. The moment a package can execute, determinism, validation, and portability are all voided. Gaps in declarative power are engine feature requests, not scripting opportunities.

### The Hardcoded Guest

World content that sneaks into engine code "temporarily" during development. The repo's oldest hard rule — `engine/` never imports from `worlds/` — is this anti-pattern's tombstone. Content reaches the engine through the package loader or not at all.

### Id Reuse

Retiring `species.wolf` and later reusing the id for something else. Identity is forever; saves, chronicles, and other packages may reference the old meaning. Retire ids to a tombstone list; never recycle.

### The Monolith Package

One undifferentiated blob mixing rules, content, generation, and scenario. The lifecycles differ — rules change with balancing, content grows with authorship, scenarios multiply with use — and Chapter 3's modularity depends on the separation.

---

# 1.7 Future Evolution

Future versions of Cardinal may introduce:

- package registries and distribution
- cryptographic signing of packages and provenance chains
- capability negotiation for optional engine features
- multi-package worlds beyond dependencies (federation)
- authoring toolchains that round-trip validated packages

Each changes how packages are made and moved.

None changes what a package is: the complete, inert, versioned definition of a world.

---

## Preparing for the Next Chapter

A package's first substantive declaration is which laws its universe obeys.

The next chapter covers **Rules and Domain Selection** — how a package chooses its domains, sets every tunable number, and defines the physics, death, and composition laws of its reality.

---

# END OF CHAPTER 1
