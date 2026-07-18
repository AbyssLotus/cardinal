# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 8
# Five Worlds

> *An architecture is worth exactly as many worlds as it can hold without changing.*

---

# Chapter Overview

Seven chapters of machinery deserve a demonstration.

This chapter designs five world packages — a medieval kingdom, a modern city, an ocean ecosystem, a space colony, and a fantasy world — on the unmodified engine. Not as game pitches: as architectural arguments. Each world is chosen to stress a different part of the specification, and each section shows its stress in terms of the volume's own vocabulary: domain selection, rules, content, generation, scenario.

The claim under test is Chapter 1's extension seam, and it is falsifiable: if any of these worlds *needs engine changes*, the architecture has failed its own standard. Where a world presses hardest against the seam, this chapter says so honestly.

A closing note collects what the five worlds prove jointly — and what they leave for Volume V.

---

# 8.1 Thornwall — The Medieval Kingdom

*The baseline: every domain, at walking pace.*

This volume's running example, completed. Thornwall selects all ten domains and stresses none exotically — its role is to prove the *full stack composes*.

**Selection:** all domains enabled.

**Rules of character:** information at horse speed; permadeath; literacy rare (Knowledge records gated hard); enforcement reach short (Chapter 9's effective-authority gap wide by default).

**Content:** temperate wildlife (shared pack), feudal institution charters, guild knowledge monopolies, the Marchfolk and their neighbors, shillings.

**Generation:** full layer stack with two simulated centuries of history — the feuds are made of raids, the dynasties of marriages.

**Scenario:** *The Lean Years* (Chapter 5), perspective-bound, mid-drought.

**What it proves:** the ownership matrix under full load. A single harvest failure must propagate Physical → Ecology → Resources → Economy → Society → Institutions → Conflict without one line of integration code. Thornwall is the reference world of reference worlds — its century signature (Chapter 7) is the specification's heartbeat.

---

# 8.2 Meridian — The Modern City

*The stress: information density and institutional depth.*

One city, five million people, the present day. No new domains — but three rule regimes turned to extremes.

**Selection:** all domains; Ecology thin (parks, pests, gulls).

**Rules of character:** information at *wire speed* — the Chapter 2 designer note's thought experiment, made canon. Rumor decay near zero; reputation arrives everywhere at once; markets arbitrage in minutes. Institutions dense and layered: city, district, utility, court, union — jurisdiction overlaps as the *normal* condition, Chapter 9's contested-authority machinery running constantly rather than at crisis.

**Content:** the deep catalog world — thousands of goods, hundreds of professions and techniques, institution charters nested five deep. Meridian is the content pipeline's scaling test (Chapter 3): composition, provenance, and reference checking at six digits of declarations.

**Generation:** authored macrostructure (the street grid, the founding institutions), procedural population, one simulated decade to warm the economy.

**Scenario:** unbound-perspective study scenarios dominate — *Meridian* is a world for observation instruments as much as play: epidemiology drills, transit strikes, market-shock studies, each an arrangement plus observations.

**What it proves:** that "modernity" is rule values, not new mechanisms — and it names the seam honestly: instant information *strains* the information pipeline's provenance model (millions of observations per tick), which is a Volume V performance problem, not an architecture change.

---

# 8.3 Pelagia — The Ocean Ecosystem

*The stress: subtraction.*

A reef, a shelf, a trench, a current system. No people. No tools, no trade, no law, no language.

**Selection:** Physical Reality, Living Systems, Resources, Ecology, Conflict. Economy, Society, Culture, Knowledge, Institutions: **disabled.**

**Rules of character:** three-dimensional space in earnest — depth, pressure, and light as first-class environmental state; currents as topology (Chapter 1 of Volume III: connectivity is not geometry — a drifting larva's "road network" is the current map). Conflict runs pure predation: hostility from ecology's food web, engagements without grievances, no terms, no truces.

**Content:** species, materials, resources. Nothing else — the validation stack must *reject* a stray culture declaration, because the domain is off (Chapter 2's selection semantics, exercised).

**Generation:** layers 1–3 and 8 only; history is population history.

**Scenario:** perspective optional and non-human when present (a shark's information pipeline is a legitimate binding); observations dominate — bleaching thresholds, trophic-cascade watches, recovery envelopes.

**What it proves:** removability (Volume III, Chapter 12) at half the matrix. Every domain the ocean lacks must leave *no residue*: no vestigial prices, no ghost settlements, no assumed hands. Pelagia is the world that keeps the engine honest about its defaults — it is the package the Convenient Default anti-pattern cannot survive.

---

# 8.4 Kepler Station — The Space Colony

*The stress: closed loops and lethal environment.*

Four thousand souls in a can, eleven light-minutes from help.

**Selection:** all domains; Ecology present but *engineered* — hydroponics bays and algae vats are its habitats.

**Rules of character:** conservation with nowhere to hide. Thornwall's matter books balance against a continent's slack; Kepler's balance against tanks. Air, water, and calories are flow resources (Chapter 3 of Volume III) in loops small enough that every leak is a plot. Environmental state is *manufactured*: warmth, pressure, and light exist because devices make them — Physical Reality's environment consuming Economy's infrastructure — and the vacuum one bulkhead away enforces Volume III's physical constraints at their most absolute. Death is permadeath; the population is four digits; every person is load-bearing (Knowledge's last-smith problem — Chapter 7 of Volume III — is Kepler's *default condition*: the station may hold exactly one mind that can repair the recycler).

**Content:** modest catalog, deep recipes — maintenance chains where every input's absence cascades. Institutions sized to a village with a state's stakes: one charter, few offices, legitimacy swinging hard on every rationing decision.

**Generation:** almost fully authored — the station *was designed*, and generation honors that; a short simulated shakedown cruise warms the social graph.

**Scenario:** *Eleven Light-Minutes* — a resupply failure, a perspective aboard, and observations on the loops: days of air, days of calories, days of consent to authority.

**What it proves:** the invariant soak (Chapter 7) as *drama*. Kepler is conservation testing turned into a world — if the engine's books have any leak, four thousand people find it by suffocating. It is the package you validate the persistence and conservation layers against, because its margins forgive nothing.

---

# 8.5 The Sundered March — The Fantasy World

*The stress: exotic rules without exotic mechanisms.*

Thornwall's cousin, where the impossible is ordinary.

**Selection:** all domains.

**Rules of character:** every conceit lands on an existing mechanism, per Chapter 2's discipline. Mana: a Knowledge resource pool; spells: techniques with delivery types (the repo's probes proved the pattern — beam, area, projectile). Resurrection: `permadeath: false` with location and cost — and *social* consequence intact: the revenant's heirs have inherited, his office is filled, his death was chronicled; the world does not un-happen (Chapter 6's continuity debt, enforced by rules). Curses: modifiers with `removal: null`. The undead: a species whose need set omits hunger and whose necromantic "ecology" regenerates from battlefields — a food web with graveyards as producers. Sacred geography: regions with environmental state (consecration as a field, like temperature) that modifiers and techniques consume.

**Content:** the exotica catalog — and, critically, *probe worlds for every one of them* (Chapter 7): a resurrection probe, a curse probe, a necromancy probe, each minimal, each proving its mechanism in isolation on the unmodified engine before the March's full weight loads.

**Generation:** deep simulated history under the exotic rules from the start — so lore, for once, may be *true*: the thousand-year lich is chronicled, the sundering event is layer 9's real work.

**Scenario:** perspective-bound high adventure — but every quest a standing situation (Chapter 5), every dungeon an ecology, every prophecy just an observation someone in-world wrote down early.

**What it proves:** the extension seam's *positive* case. The fantasy world is where the temptation to fork the engine is strongest — and where the specification's bet pays or breaks. If resurrection, mana, and the walking dead are rule values and content, the seam holds. Where a conceit genuinely cannot be expressed — true time travel, say, against Volume II's append-only history — the seam speaks its other sentence: that is a domain proposal or a refusal, *never* a hack.

---

# Designer Note
## One Engine, Falsifiably

The five worlds were chosen as a proof structure, and it is worth making the structure explicit:

Thornwall proves the domains *compose*. Meridian proves rules alone span centuries of "setting." Pelagia proves domains *subtract* cleanly. Kepler proves the invariants hold when margins vanish. The March proves exotica lands on existing mechanisms.

Between them they cover the specification's load-bearing walls: the ownership matrix under full load and half load, rule regimes at both extremes of speed and scarcity, content at both extremes of catalog size, generation from fully procedural to fully authored, scenarios bound, unbound, and non-human.

And each is a *standing falsification target*: any future engine change that quietly breaks one of the five — a default that wounds Pelagia, a leak that Kepler's margins expose, a hardcode the March cannot express around — fails Volume IV as a whole. Keep all five green, or the claim "world-agnostic" is marketing.

---

# 8.6 Engineering Invariants

Every implementation SHALL preserve these rules.

1. All five reference worlds run on the unmodified engine, by construction and by CI.
2. Each world maintains reference seeds and signatures per Chapter 7.
3. Pelagia's disabled domains leave no residue in its running state.
4. Kepler's conserved loops reconcile exactly over arbitrary horizons.
5. The March's exotica are expressed entirely as rule values, content, and probes.
6. Meridian's content scale passes composition and validation within declared budgets.
7. Thornwall's full-stack cascade signature is a release gate for the engine.
8. No reference world receives engine-side special cases, ever.
9. A conceit inexpressible as package data triggers a domain proposal or refusal — never a fork.
10. The five worlds' packages are maintained as living specification artifacts alongside these volumes.

---

# 8.7 Volume IV in Retrospect

This volume answered the roadmap's second question — *how are worlds assembled?* — in eight movements:

A package is the complete, inert definition of a world (1). Its constitution is domain selection and rules (2); its vocabulary of kinds is content (3); its birth is generation (4); its offered moments are scenarios (5); its future across versions is provenance and migration (6); its right to exist and evidence of life are validation and testing (7); and its proof is five worlds that share every line of engine code while sharing almost nothing else (8).

One question remains open, and it is the roadmap's last:

*How should Cardinal actually be built?*

---

## Preparing for Volume V

Volume V — **Reference Architecture** — descends from specification to construction: engine bootstrap, module boundaries, scheduling and messaging, persistence machinery, parallelism, observability, and the discipline of building all of the above without freezing the innovation this specification exists to protect.

The philosophy is written. The worlds are designed.

What remains is the engine.

---

# END OF CHAPTER 8
# END OF VOLUME IV
