# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 4
# World Generation

> *A world is not built. It is grown — quickly, once, before anyone is watching.*

---

# Chapter Overview

Everything so far is potential: rules that would govern, kinds that could exist.

Generation is actualization. It takes the package's declarations and a seed, and produces *tick zero*: a complete, committed, validated initial reality — terrain with geology, forests with populations, villages with families, families with grudges.

This chapter defines generation's place in the architecture, the layered model by which worlds are grown, the three sources of initial fact (authored, procedural, and simulated), and the single test every generated world must pass:

**After tick zero, no one can tell.**

Not the systems, not the chronicle's readers, not the domains — nothing downstream may be able to distinguish a generated world from one that had always been running. Generation is the one process permitted to write reality without simulating it, and that permission ends, totally and forever, at the first committed tick.

---

# 4.1 Generation's Contract

Generation is a pure function:

```text
(sealed package, scenario selection, seed) → initial committed reality
```

Three clauses give the contract its teeth:

**Determinism.** Same package, same scenario, same seed: byte-identical world. Generation draws from the seeded stream architecture of Volume II — substreams per layer, so terrain generation and family generation cannot perturb each other.

**Validity.** The output is a committed reality satisfying every invariant of Volumes II and III: one owner per fact, referential integrity, populations with causes (their cause is the generation event), conservation from tick zero forward. Chapter 7's validators run against generated output exactly as against authored content.

**Termination of privilege.** Generation may place a mountain without erosion, a family without a wedding, a grudge without a crime. It is the only writer with that power, and the power expires at commit. Post-generation, every change flows through systems, proposals, and the scheduler — no tool, no scenario, no later content update may write reality directly again.

The generation event itself enters the chronicle: reality's first recorded cause is its own creation, with package version, scenario, and seed as provenance.

---

# 4.2 The Layer Model

Worlds grow the way their dependencies point. Each layer reads the committed output of the layers before it and writes its own domain's initial facts:

```text
1. Space        — terrain, elevation, water, climate zones, regions   (Physical)
2. Materials    — geology, soils, deposits                            (Physical, Resources)
3. Life         — biomes, populations, food-web instantiation        (Ecology, Living)
4. Habitation   — settlements, households, persons                    (Society, Living)
5. Livelihood   — holdings, markets, infrastructure                   (Economy)
6. Meaning      — cultures, languages, knowledge distribution         (Culture, Knowledge)
7. Order        — institutions, offices, laws, borders               (Institutions)
8. Tension      — grievances, hostilities, standing relations         (Conflict, Society)
9. History      — the backstory that explains all of the above        (chronicle)
```

The ordering is dependency, not ceremony: settlements need terrain and water; markets need settlements; grudges need neighbors. A package configures each layer — or replaces its parameters entirely — but the flow of constraint always runs downhill from space.

Layers matter architecturally because they make *coherence* the default. A village generated after the rivers sits on a river. A culture generated after the mountains differs across them. Generation that ignores layer order produces worlds that are individually plausible and jointly absurd — the desert fishing town, the border no geography explains.

## Three Sources of Fact

Each layer draws initial facts from three sources, freely mixed:

**Authored.** Fixed declarations placed exactly: Thornwall's capital is *here*, the King's Road runs *so*. Authored facts are content-like data in `generation/`, and they win — procedure fills around them.

**Procedural.** Parameterized algorithms expanding the seed into terrain, populations, families, name-bearing persons. All procedure is deterministic; all parameters are package data (Chapter 2's invariant reaches here: a generator's constants are rules).

**Simulated.** The deepest source: run the engine itself, headless and accelerated, from a cruder start — then take the result as tick zero. Let three centuries of ordinary simulation carve the trade routes, accrete the lore, and compound the grudges. Generated history is then not backstory *written* but backstory *lived*, with a chronicle that survives audit because it is real.

---

# Designer Note
## The Backstory Problem

Every world needs a past. The naive solution writes one: "the two duchies have feuded for a century," stamped into a lore field.

The problem: written backstory has no facts under it. *Which* raids? *Whose* deaths? When the simulation starts asking — because reputation, grievances, and lore all trace to events — the feud is a label over a void, and every query into it returns nothing. The world is a stage set: painted doors that open onto plywood.

The layered fix authors the *shape* (two duchies, a contested river) and generates the *substance*: seed the rivalry as standing facts in layer 8, or better, let layer 9's simulated centuries produce the raids that produce the grievances that produce the feud.

The rule of thumb: **author what must be true; generate what makes it true.** The scenario needs the feud; the simulation needs the feud to be made of something.

---

# 4.3 Scale and Laziness

Nothing in the contract requires generating everything at once — it requires that *observation never detects the difference*.

An implementation may generate lazily (regions actualized as attention approaches) provided three disciplines hold:

1. **Determinism is positional, not temporal.** A region generates identically whether actualized at tick 0 or tick 40,000 — its substream is keyed to the region, never to when it was asked for.
2. **Aggregate truth precedes detail.** Volume III Chapter 10's individuation seam generalizes: an unactualized region still has authoritative aggregate facts (populations, stocks, settlements-in-summary) that answer queries and absorb consequences. Actualization refines truth; it never invents it.
3. **The seam balances.** Whatever crossed into the unactualized region (a fleeing bandit, a trade caravan, a plague) is honored when detail arrives.

Laziness is an implementation strategy under the same flag as all of Chapter 1's promises: architecture never notices.

---

# 4.4 Common Queries

- What package, scenario, and seed produced this world?
- Which layer, and which source (authored / procedural / simulated), produced this fact?
- What did the generation parameters declare for this layer?
- Is this generated world valid against all Volume II/III invariants?
- What differs between two worlds generated from the same package at different seeds? (tooling: seed studies)
- Does regeneration reproduce this world byte-identically?

---

# 4.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Generation is deterministic: package + scenario + seed → identical reality.
2. Generation output satisfies every invariant that binds a running world.
3. Direct fact-writing ends at the first committed tick, permanently.
4. The generation event is chronicled with full provenance.
5. Layers consume only previously committed layers; constraint flows downhill from space.
6. All generator parameters are package data; no constants hide in generator code.
7. Authored facts take precedence; procedure fills around them deterministically.
8. Simulated history is real simulation under real rules, chronicled as such.
9. Lazy actualization is positionally deterministic and aggregate-consistent.
10. A generated world is downstream-indistinguishable from an eternal one.

---

# 4.6 Anti-Patterns

### The Painted Backdrop

Generating scenery without facts — forests with no populations, towns with no households. Everything placed must be *made of* the facts the domains will query, or the first close look tears the canvas.

### Backstory by Fiat

Lore fields describing a past no events support. Author shape, generate substance.

### The Ungoverned Editor

World-editing tools that write facts into a running save "just this once." The privilege of direct writing died at tick zero; resurrection of it voids the chronicle's authority. Editors edit *generation inputs* and regenerate, or operate through in-world mechanisms.

### Seed Drift

Generators whose output depends on iteration order, wall time, or actualization sequence. One nondeterministic layer poisons every layer above it.

### The Perfect Start

Generating worlds in equilibrium — every market cleared, every population at capacity, every grievance resolved. Dead calm is not plausibility; a world should begin *mid-story*, with pressures already in motion. Layer 8 exists precisely to load the springs.

### Generation Creep

Reaching back into generation to fix a live world's problems ("regenerate the northern ore, the economy needs it"). Post-commit, the world's problems are the world's — that is the entire point of having one.

---

# 4.7 Future Evolution

Future versions of Cardinal may introduce:

- plate-tectonic and hydrological space generation
- coupled climate–biome generation
- deep simulated-history tooling (era summaries, dynasty digests)
- constraint-solving placement (declare relationships; solve positions)
- cross-world generation libraries as shared packs

Each grows better worlds.

The contract they grow under — deterministic, valid, privilege-expiring — is settled.

---

## Preparing for the Next Chapter

The world now exists, mid-story, springs loaded.

The next chapter decides *whose* story: **Scenarios** — named starting conditions, perspectives without privilege, and the difference between a world and a situation.

---

# END OF CHAPTER 4
