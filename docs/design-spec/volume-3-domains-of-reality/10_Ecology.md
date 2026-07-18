# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 10
# Ecology

> *No organism lives in a world. It lives in a web.*

---

# Chapter Overview

Living Systems (Chapter 2) governs what happens inside an organism.

Ecology governs what happens *between* organisms — at the scale where individuals blur into populations and interactions blur into systems.

A wolf eating a deer is Living Systems: hunger satisfied, matter conserved.

Ten thousand wolf-deer encounters over a decade — thinning herds, shifting migration routes, starving packs, recovering meadows — is Ecology.

This domain owns populations, food webs, habitats, carrying capacity, migration, and succession: the slow systemic machinery that keeps a wilderness plausible whether or not any individual within it is being simulated in detail. It is also the domain that answers for the world's living background — the reason forests do not empty when no one is looking, and the reason they *do* empty when someone hunts them too hard.

---

# 10.1 Purpose

The purpose of the Ecology domain is to make the living world systemic: self-regulating, historied, and vulnerable.

Self-regulating, because populations must find their own levels through feedback — prey abundance feeding predators, predator pressure thinning prey — without authored spawn tables holding anything in place.

Historied, because the state of a forest must be the product of its past: the fire of Y190, the wet decade, the wolf purge the barons ordered.

Vulnerable, because a living system that cannot be damaged cannot be meaningful. Overhunting, overgrazing, deforestation, and drained marshes must have real, propagating, slow-to-heal consequences.

Ecology also carries a load-bearing architectural duty: it is the domain of *aggregate fidelity*. The engine cannot simulate every beetle. Ecology owns the truthful bookkeeping of living things that are real but not individually instantiated — and the seam that lets a member of an aggregate population step into individual existence when the simulation needs it.

---

# 10.2 Responsibilities

The Ecology domain owns:

- Populations — counted stocks of a species in a habitat, for organisms not individually simulated
- Food web relations — who eats whom, declared per world package
- Carrying capacity — what a habitat can sustain, derived from Physical Reality and flora
- Population dynamics — births, deaths, growth, and collapse at aggregate scale
- Habitats — the ecological reading of regions: what can live here, and how well
- Migration — seasonal and pressure-driven movement of populations
- Succession — the slow transformation of habitats: meadow to scrub to forest, and back by fire
- Ecosystem events — blooms, blights, die-offs, irruptions — as deterministic outcomes
- Individuation — the contract by which aggregate members become instantiated organisms, and return

---

# 10.3 Non-Responsibilities

The Ecology domain does not own:

- **Individual organisms.** An instantiated wolf is Living Systems' concern entirely. Ecology resumes ownership only of the pack-as-population.
- **The environment itself.** Temperature, water, soil, and terrain belong to Physical Reality. Ecology consumes them as habitat inputs.
- **Extraction economics.** A fishery's stock is jointly visible with Resources (Chapter 3); the *worth* of fish is Economy's. Ecology owns the population dynamics beneath both.
- **Human decisions.** Ecology presents the state of the forest; the choice to clear-cut it belongs to decision systems consuming knowledge and need.
- **Domesticated exception-making.** A herd of kept cattle is a population like any other — fenced, fed, and culled by owners, but obeying the same dynamics. Domestication changes the parameters, never the architecture.
- **Weather and climate.** These are Physical Reality's environmental state; Ecology is their most attentive consumer, not their author.

---

# 10.4 Canonical Concepts

## Population

The aggregate unit of the living background:

```text
pop.deer.westwood
Species: deer
Habitat: Westwood
Count: 1,240
Age structure: 22% young, 61% mature, 17% old
Health: 0.83
```

A population is an entity with identity and history. It is not a spawner: its count changes only through accounted causes — births, predation, starvation, harvest, migration, disease.

## Food Web

The declared trophic graph of a world package:

```text
grass ← deer ← wolf
grass ← hare ← fox ← wolf
carrion ← raven, fox
```

Every edge is a consumption relation with rates. The web is data; the dynamics that run on it are engine. New species extend the web without new code.

## Carrying Capacity

The sustainable ceiling for a population in a habitat, derived — never authored — from the habitat's actual resources:

```text
K(deer, Westwood) = f(browse biomass, water, cover, winter severity)
```

Because capacity is derived, damaging the habitat *moves the ceiling*. A logged forest carries fewer deer, which starves wolves, which spares hares — the cascade is automatic.

## Migration

Population movement along Physical Reality's topology, driven by season, pressure, or displacement. Migration couples distant habitats: a famine in the north arrives in the south on hooves.

## Succession

The slow, deterministic transformation of habitats through ecological stages, with disturbance — fire, flood, axe — resetting the clock. Succession is why land has a *trajectory*, and why abandoned farmland becomes forest on a schedule measured in decades.

## Individuation

The two-way seam between aggregate and individual:

```text
Population count 1,240
        │  (encounter requires an individual)
        ▼
Instantiate deer #58822 (drawn deterministically from population distributions)
        │  (encounter ends; deer survives)
        ▼
Reabsorb: population count unchanged, memory of the event retained
```

The books must balance across the seam in both directions: a deer killed while instantiated is a deer subtracted from the population; a deer never encountered was nevertheless always countable.

---

# Designer Note
## The Forest Does Not Respawn

The most corrosive convenience in world simulation is the spawn table: wolves appear because the region is a "wolf region," at a rate tuned for encounter pacing.

Cardinal's forests keep books instead.

The Westwood has 1,240 deer because it has browse for roughly 1,300; it has browse for 1,300 because the Y190 fire reset half of it to meadow; it has four wolf packs because that is what 1,240 deer support. Hunt the deer to 400, and the packs starve, split, and turn to Milbrook's sheep — and the reader of the chronicle can trace *why*.

A spawn table answers "what should attack the player here?"

An ecology answers "what does this land actually hold?" — and every other domain inherits the honesty of that answer.

---

# 10.5 Domain Interactions

## Consumes

- **Physical Reality:** habitat inputs — terrain, water, climate, seasonal cycles; disturbance events like fire and flood
- **Living Systems:** the vital events of instantiated organisms, folded back into population accounts
- **Resources:** harvest pressure — every extraction from a living stock is an ecological debit
- **Conflict:** battlefield carrion, burned forests, armies that strip the land
- **Society & Economy (indirectly):** settlement expansion, grazing, clearing — habitat conversion in all its forms

## Provides

- **Resources:** the stock dynamics beneath every living resource — game, fish, timber, forage
- **Living Systems:** the populations from which individuals instantiate, with truthful distributions
- **Physical Reality:** ecological modification — forestation, erosion control, soil enrichment
- **Economy:** abundance and scarcity signals that move harvests and prices
- **Society:** the habitability that decides where settlement is possible — and the collapses that end it
- **Culture & Information:** the blights, irruptions, and vanishings that become omen, lore, and news

---

# 10.6 Common Queries

- What populations inhabit this region, at what counts?
- What does this habitat currently support, and how close to capacity is each population?
- What preys on this species here, and what does it feed?
- What happens to this population under current harvest pressure?
- Where will this herd be in autumn?
- What stage of succession is this land in, and what disturbed it last?
- Draw me an individual from this population.
- Which populations are collapsing, and why?

---

# 10.7 Architectural Contracts

1. Populations are accounted stocks: every change in count has a cause with provenance.
2. The food web is world-package data; dynamics are engine and deterministic.
3. Carrying capacity is derived from habitat facts, never authored per region.
4. Aggregate and individual representations reconcile exactly across the individuation seam.
5. Harvest, predation, and disturbance debit populations through the same accounting.
6. Migration moves populations along real topology at real speeds.
7. Succession advances deterministically and is reset only by recorded disturbance.
8. Ecological collapse is reachable, propagating, and slow to reverse.

---

# 10.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. No population changes count without an accounted cause.
2. Nothing spawns; everything descends.
3. Derived capacity moves when habitat moves.
4. Instantiated individuals are drawn deterministically from population state.
5. Deaths and births reconcile across the aggregate–individual seam.
6. Trophic effects propagate through the declared web, never by special case.
7. Extinct locally means absent until migration or reintroduction.
8. Domesticated populations obey wild dynamics with owned parameters.
9. Ecological time runs whether or not any observer attends.
10. The living background is queryable history, not generated scenery.

---

# 10.9 Anti-Patterns

### The Spawn Table

Creatures generated for encounter pacing. The single fastest way to make a world's wilderness a lie.

### Infinite Prey

Predators that never deplete what they eat. Without trophic accounting, "balance of nature" is a painted backdrop.

### The Static Meadow

Habitats that never change stage. Land must have trajectory — abandonment, regrowth, and disturbance are where landscape history lives.

### Population Teleportation

Rebalancing counts across regions without migration events. Animals travel roads of their own; they do not redistribute by spreadsheet.

### The Untouchable Wild

Ecosystems that cannot be damaged by simulated action. If the fishery cannot collapse, fishing policy is theater.

### Twin Ledgers

Ecology and Resources keeping separate counts for the same living stock. One population, one truth, two views.

---

# 10.10 Future Evolution

Future versions of Cardinal may introduce:

- disease ecology and epizootics crossing into settlements
- genetic drift at population scale feeding individual inheritance
- keystone species modeling and trophic cascade analysis
- soil and nutrient cycling beneath carrying capacity
- climate-driven range shifts over historical time
- co-evolution of harvest culture and stock behavior

Each deepens the web without changing its law:

Every count has a cause.

---

## Preparing for the Next Domain

Ten chapters have each claimed their territory: space, life, stock, trade, kinship, meaning, skill, violence, law, and the web.

The next chapter claims nothing. It examines what happens *between* the claims: **Emergence** — how phenomena no domain owns arise from the interaction of all of them, and why Cardinal's deepest features are the ones no one will ever write.

---

# END OF CHAPTER 10
