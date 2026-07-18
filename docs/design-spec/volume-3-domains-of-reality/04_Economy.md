# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 4
# Economy

> *Price is not a number assigned to a thing. It is a fact discovered by exchange.*

---

# Chapter Overview

Resources made scarcity a fact.

The Economy makes scarcity *move*.

This domain governs valuation, exchange, production, and the flows of goods, services, and money between entities. It is where a fisherman's catch becomes a merchant's stock, a miner's ore becomes a smith's blade, and a season's drought becomes next winter's famine prices.

The Economy is Cardinal's clearest demonstration of the engine's core thesis:

Nothing here is scripted.

There is no "economy simulator" issuing prices from above.

There are only entities holding things, wanting things, and trading — and from thousands of those small, local, self-interested exchanges, the phenomena we call *markets*, *prices*, *trade routes*, and *wealth* emerge as observations.

---

# 4.1 Purpose

The purpose of the Economy domain is to answer questions of allocation:

- Who holds what?
- What will they surrender to obtain what they lack?
- How do goods travel from where they are abundant to where they are needed?
- What is produced, from what, by whom?

The domain exists so that allocation is *decided by the simulation* rather than by authored tables.

A bakery has bread because a farmer grew wheat, a miller ground it, a carter hauled it, and a baker baked it — and each step happened because it was worth someone's while.

Break any link and the shelf is empty.

An empty shelf that has a *reason* is worth more than a full shelf that has none.

---

# 4.2 Responsibilities

The Economy domain owns:

- Holdings — which entity possesses which goods, and in what quantity
- Transfer — the atomic movement of goods and money between entities
- Exchange — trades, sales, barters, and the terms on which they clear
- Money — the world package's media of exchange and their circulation
- Production — declared transformations of inputs into outputs (recipes, yields, labor requirements)
- Markets — the standing structures where offers meet (a village square, a port exchange, a black market)
- Contracts of exchange — standing agreements: wages, rents, tributes, standing orders
- Local price memory — what exchanges have recently cleared at, as recorded history

---

# 4.3 Non-Responsibilities

The Economy domain does not own:

- **The goods' physical existence.** A crate of apples is matter (Physical Reality) with resource lineage (Resources). The Economy owns *whose* it is and *what it trades for*.
- **Legitimacy of ownership.** The Economy records holdings. Whether a holding is lawful, contested, or stolen is a judgment made by Society and Institutions.
- **Desire.** Wanting bread comes from hunger (Living Systems) via decision systems. The Economy sees only the demand that desire produces.
- **Transport itself.** Movement of goods happens through Physical Reality's space and topology. The Economy decides that goods *should* move; the world governs whether they *can*.
- **Enforcement.** Debt collection, contract enforcement, and punishment of theft belong to Institutions.
- **Global statistics.** GDP, inflation, and trade volume are derived observations, computed by observers, never stored as authoritative state.

---

# 4.4 Canonical Concepts

## Holding

The fundamental economic fact: an entity possesses a quantity of a good.

```text
Entity #218 (Mara, baker)
holds  item.flour × 40
holds  money.col × 312
```

Holdings are relationships between owners and goods, with quantity. Every economic event is ultimately a rearrangement of holdings.

## Transfer

The atomic economic operation. Goods or money move from one holding to another, conservatively — nothing is created or destroyed by movement.

Every trade, theft, gift, tax, and inheritance decomposes into transfers.

## Exchange

Two transfers bound together by agreement:

```text
Mara → Tomas : item.bread × 2
Tomas → Mara : money.col × 3
```

The ratio at which an exchange clears *is* a price. Prices are therefore facts about events, not properties of goods.

## Market

A persistent meeting place for exchange, located in the world, with participants, stock, and memory of recent clearings.

Markets are local. The same good clears at different ratios in different places, and that difference — the spread — is what makes a merchant's journey worth taking.

## Production

A declared transformation:

```text
recipe.bake_bread
consumes: item.flour × 1, res.firewood × 0.2, labor(baking) × 1h
produces: item.bread × 4
```

Recipes are world-package data. The engine guarantees only that production is conservative, deterministic, and consumes real inputs from real holdings.

## Money

A good the world package designates as a general medium of exchange. Money obeys every rule other goods obey: it is held, transferred, conserved, and never minted implicitly.

A world package may define several monies, or none — a pure barter world is architecturally identical.

---

# Designer Note
## No One Sets Prices

The most tempting shortcut in economic simulation is the global price table — one authoritative number per good, adjusted by a formula.

Cardinal treats price as an *emergent observation*: the ratio at which real exchanges recently cleared, in one place, between real holders.

This is more than purity. Local prices are what create the merchant's profession, the smuggler's route, the famine profiteer, and the boomtown. A global price table deletes every one of those stories in exchange for a tidy number.

If you need "the price of bread," ask a market what bread last sold for.

You will get an answer with a location, a history, and a reason.

---

# 4.5 Domain Interactions

## Consumes

- **Resources:** stocks, yields, and depletion — the supply frontier
- **Physical Reality:** distance, topology, and transport constraints — the cost of moving anything
- **Living Systems:** labor capacity, and consumption needs that become demand
- **Society:** settlements (where markets form), households (who pools holdings), reputation (who is trusted to trade)
- **Institutions:** property rules, taxation, currency issuance, contract enforcement
- **Knowledge:** production recipes known to producers; price information carried by travelers

## Provides

- **Society:** wealth distributions that stratify settlements into classes
- **Conflict:** war chests, plunder targets, blockade pressure, and the material limits of armies
- **Institutions:** the tax base, and the disputes that make law necessary
- **Knowledge:** the incentive to improve — better recipes, better routes, better tools
- **Ecology (indirectly):** extraction pressure, as demand pulls on living stocks

---

# 4.6 Common Queries

- What does this entity hold?
- What did this good last clear at, in this market?
- Where is the nearest market with stock of this good?
- What is the spread on this good between these two settlements?
- Can this producer currently afford these inputs?
- Who in this settlement can afford this good?
- What standing obligations does this entity carry?
- Which routes carried goods between these regions recently?

---

# 4.7 Architectural Contracts

1. All economic change decomposes into conservative transfers between holdings.
2. Exchange requires agreement between parties; terms are recorded as events.
3. Prices are recorded observations of cleared exchanges, local to a market, never global state.
4. Production consumes real held inputs and produces outputs deterministically per recipe.
5. Money is conserved; issuance is an explicit institutional act, never a side effect.
6. Transport of goods is subject to Physical Reality without exception.
7. Aggregate statistics are derived on demand and carry no authority.
8. Every economic event has provenance: parties, place, terms, tick.

---

# 4.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Goods and money are conserved through every transfer.
2. Holdings have exactly one authoritative record.
3. No exchange occurs without two consenting sides or an explicit taking event.
4. Prices are facts about exchanges, not properties of goods.
5. Markets are located, local, and historied.
6. Production never creates outputs without consuming declared inputs.
7. Money is never minted or destroyed implicitly.
8. Distance and topology impose real costs on all movement of goods.
9. Derived statistics are recomputed, never persisted as truth.
10. Theft, tax, and tribute are transfers with provenance, not deletions.

---

# 4.9 Anti-Patterns

### The Global Price Table

One authoritative number per good. Deletes locality, arbitrage, merchants, and every economic story worth telling.

### The Bottomless Shop

Vendors with infinite stock and infinite money. A shop is a holder like any other; when it is out of bread, it is out of bread.

### Demand by Decree

Simulating demand with authored curves instead of deriving it from entities that actually need and can actually pay. Authored demand cannot starve, hoard, or panic.

### The Faucet-and-Sink Economy

Creating money on kill and destroying it on purchase. Wealth must circulate; when it concentrates, that concentration should be a discoverable fact with an owner and an address.

### Teleporting Trade

Fulfilling exchanges across distance instantly. If the grain never travels the road, bandits, tolls, weather, and war can never touch it — and the world loses its logistics layer.

---

# 4.10 Future Evolution

Future versions of Cardinal may introduce:

- credit, debt instruments, and interest
- banking and money-changing between currencies
- insurance against loss in transit
- futures and speculation on standing stocks
- firms — multi-person producers with internal structure
- labor markets with negotiated wages

Each builds on the same primitives — holdings, transfers, exchange — without displacing them.

The economy grows more sophisticated.

Its atoms do not change.

---

## Preparing for the Next Domain

Exchange presumes something the Economy cannot supply: parties who persist, trust, cluster, and cooperate.

The next chapter introduces **Society** — the domain of settlements, households, roles, and reputation, where organisms become persons and geography becomes home.

---

# END OF CHAPTER 4
