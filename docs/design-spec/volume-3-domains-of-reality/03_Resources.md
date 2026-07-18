# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 3
# Resources

> *A resource is not a thing. It is a relationship between matter and need.*

---

# Chapter Overview

Physical Reality owns matter.

Living Systems owns the organisms that require it.

Between them sits a question neither domain can answer alone:

**What, in this world, is worth taking?**

The Resources domain answers it.

A resource is matter or energy that some consumer can use. Iron in a mountain, fish in a river, timber in a forest, sunlight on a field, mana in a ley line — all are resources the moment something in the world can extract value from them.

This makes Resources a *bridging* domain.

It translates the physical world's raw substance into stocks that living systems, economies, and societies can reason about — without either side needing to understand the other.

---

# 3.1 Purpose

The purpose of the Resources domain is to give usable matter three properties that raw physical substance lacks:

**Identity.** This deposit is a discoverable, referenceable thing with continuity over time.

**Quantity.** There is a finite, measurable amount, and taking some leaves less.

**Renewal.** Some stocks replenish; the rules of replenishment are facts of the world.

Without this domain, every consumer would query raw material distributions directly, and each would invent its own incompatible notion of "how much is left."

With it, scarcity becomes a single, shared, authoritative truth.

Scarcity is the engine of nearly everything interesting that happens above this layer.

Trade exists because resources are unevenly distributed.

Conflict exists because two parties want the same stock.

Migration exists because stocks deplete.

Technology exists because extraction can be improved.

---

# 3.2 Responsibilities

The Resources domain owns:

- Resource definitions — the classes of usable matter a world package declares
- Deposits and stocks — where resources exist and in what quantity
- Quality and grade — not all iron, soil, or timber is equal
- Regeneration rules — growth rates, seasonal cycles, exhaustion thresholds
- Depletion state — what extraction has done to a stock over time
- Accessibility facts — how difficult a stock is to extract, as a property of the deposit
- Discovery state of deposits *in reality* (what exists, independent of who knows)

---

# 3.3 Non-Responsibilities

The Resources domain does not own:

- **Value or price.** A stock has quantity here. What it is *worth* is the Economy's interpretation.
- **Ownership.** Who may extract is a claim owned by Society and enforced by Institutions.
- **Knowledge of deposits.** Reality knows where every vein lies. A prospector's map is information, with all of information's fallibility.
- **Extraction labor.** The act of mining is performed by organisms and infrastructure; this domain only defines what the act yields.
- **Manufactured goods.** Once matter is transformed by intent, it becomes an artifact in the economic world. Resources end where production begins.
- **The matter itself.** Physical Reality remains the owner of material facts. A resource is an *interpretation layered onto* matter, not a duplicate of it.

---

# 3.4 Canonical Concepts

## Resource

A class of usable matter or energy defined by the world package:

```text
res.iron_ore
res.timber_oak
res.freshwater
res.arable_soil
res.sunlight
res.ley_energy
```

A resource definition specifies what it is extracted from, what extracting it yields, and how its stocks behave over time.

## Deposit

A located, identified concentration of a resource:

```text
Deposit #4471
Resource = res.iron_ore
Region = Northern Ridge
Quantity = 182,000 units
Grade = 0.74
Extraction difficulty = High
```

Deposits are entities. They persist, they have history, and they can be exhausted, contested, and remembered.

## Stock

The current quantity of a deposit or pool.

Stocks change through exactly two mechanisms: extraction (a debit, always conservative) and regeneration (a credit, always rule-driven and deterministic).

## Regeneration

The world package classifies every resource's renewal behavior:

```text
Nonrenewable   — ore, fossil fuel: stock only falls
Renewable      — timber, fish: regrows toward capacity at a rate
Flow           — sunlight, wind, river water: replenishes each tick, cannot be stockpiled at the source
Threshold      — renewable stocks that collapse if driven below a floor
```

Threshold behavior deserves emphasis: a fishery driven below its recovery floor does not regrow.

Consequence, not punishment.

## Yield

The deterministic result of applying an extraction action to a deposit — a function of the deposit's grade, the extractor's capability, and the method used.

---

# Designer Note
## Scarcity Must Be Real

Many games refill their mines because empty mines feel bad.

Cardinal refuses.

If a vein holds 182,000 units, the 182,001st unit does not exist. The village that grew around that mine will face a day when the mine is done, and everything downstream — abandonment, migration, ghost towns, wars over the next ridge — flows from honoring one number.

A world where taking things leaves less is a world where history means something.

---

# 3.5 Domain Interactions

## Consumes from Physical Reality

- material composition (what substance a deposit is an interpretation of)
- location, containment, and accessibility
- environmental state (rainfall feeds aquifers; sunlight feeds fields)

## Consumes from Living Systems / Ecology

- population stocks for living resources (a fishery's stock *is* an ecological population viewed as a resource)
- growth rates for renewable biological stocks

## Provides to other domains

- **Economy:** the supply side of everything — quantities, grades, and depletion that markets turn into price
- **Society:** the geographic logic of settlement ("towns appear where resources are")
- **Conflict:** objects of contention with precise, comparable worth
- **Knowledge:** discovery targets — surveying, prospecting, and geology exist because deposit knowledge is imperfect
- **Institutions:** things to claim, tax, license, and regulate

## The Living Resource Boundary

When the resource is alive — fish, timber, game — the stock is jointly constrained: Ecology owns the population dynamics, Resources exposes the extractable view of it.

The contract is that both read the same underlying facts. There is one truth about how many fish exist, and overfishing is visible to both domains because of it.

---

# 3.6 Common Queries

- What resources exist within this region?
- How much remains in this deposit?
- What does this deposit yield per unit of extraction effort?
- Is this stock renewable, and at what rate?
- How depleted is this stock relative to its history?
- What is the nearest source of a given resource?
- What happens to this stock if current extraction continues?

The last query matters most.

Projection of depletion is what lets agents, economies, and institutions exhibit foresight — or suffer for lacking it.

---

# 3.7 Architectural Contracts

1. Every stock has exactly one authoritative quantity.
2. Extraction is conservative: yielded matter is debited from the stock and enters the world as held matter.
3. Regeneration is deterministic, rule-driven, and defined entirely by the world package.
4. Deposits are entities with persistent identity and history.
5. Quality modifies yield through declared functions, never special cases.
6. Resource definitions add no engine code — a new world package with new resources changes data only.
7. Reality's knowledge of deposits is complete; observers' knowledge flows through the information pipeline.

---

# 3.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Stocks are finite and authoritative.
2. Extraction always debits; matter is never created by taking.
3. Regeneration follows deterministic world-package rules.
4. Nonrenewable means nonrenewable.
5. Collapse thresholds, once crossed, hold.
6. Deposits retain identity from discovery through exhaustion.
7. Value is never stored in this domain.
8. Ownership is never stored in this domain.
9. Living stocks share one truth with Ecology.
10. Depletion is history and is never silently reset.

---

# 3.9 Anti-Patterns

### The Infinite Node

Resource points that yield forever. This deletes scarcity, and with it trade, migration, and conflict motives.

### Spawning Supply

Creating resources at the moment of demand ("the shop needs iron, so iron appears"). Supply must have geography and history.

### Value Creep

Storing prices, rarity tiers, or "worth" on the resource itself. Worth is contextual and belongs to the Economy; a desert prices water differently than a delta, from the same facts.

### The Duplicate Ledger

Letting the economy, ecology, and resource layers each track their own quantity for the same stock. Divergence is guaranteed. One number, one owner.

### Omniscient Prospecting

Letting agents query true deposit locations directly. Discovery must pass through observation and knowledge, or geology becomes a lookup table.

---

# 3.10 Future Evolution

Future versions of Cardinal may introduce:

- geological generation of deposit distributions
- resource chains (ore → bloom → ingot) with declared transformation graphs
- contamination and stock quality degradation
- aquifer and watershed modeling
- energy as a fully accounted flow resource
- survey fidelity models (estimated vs. true stock)

Each extension enriches what supply *is* without changing what the domain *does*:

Resources makes scarcity a fact.

---

## Preparing for the Next Domain

Scarcity alone is inert. It becomes dynamic the moment two parties can exchange.

The next chapter introduces the **Economy**: the domain where quantities become values, exchanges become markets, and the uneven geography of resources becomes the circulatory system of a living world.

---

# END OF CHAPTER 3
