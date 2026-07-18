# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 12
# Cross-Domain Interaction

> *Twelve domains. One reality. The seams are the architecture.*

---

# Chapter Overview

Eleven chapters have partitioned reality into domains, each with owned facts, declared boundaries, and refused responsibilities.

Partition creates a final obligation: the rules by which the parts constitute a whole.

This chapter is Volume III's closing contract. It gathers the interaction principles used piecemeal throughout the volume — ownership, consumption, seams, event flow — and states them once, formally, as the law governing *every* pair of domains, including pairs not yet imagined.

It exists because the alternative is entropy. Without explicit interaction law, every new domain negotiates private arrangements with every existing one, coupling multiplies quadratically, and the engine calcifies into exactly the tangle Cardinal was designed to escape.

The law is small. It fits in four principles:

1. Domains share reality, never interfaces.
2. Every fact has one owner; everyone else consumes.
3. Effects cross boundaries as proposals and events, never as calls.
4. Composition is the scheduler's job, not the domains'.

Everything else in this chapter is elaboration.

---

# 12.1 The Interaction Model

Volume II established that *systems* never call each other. Domains — being sets of facts plus the systems that transform them — inherit the same discipline at larger scale.

## No Domain Interfaces

A domain does not expose an API to other domains.

It exposes *facts*.

When Conflict needs terrain, it does not call Physical Reality. It reads committed facts that Physical Reality owns. When combat damages a wall, Conflict does not tell Physical Reality to update — it proposes a change to a fact Physical Reality owns, and the scheduler commits or resolves it.

```text
        WRONG                          RIGHT

Conflict ──calls──► Physics      Conflict ──reads──► committed facts
                                 Conflict ──proposes──► Δ(wall integrity)
                                 Scheduler ──commits──► new reality
                                 Physics systems ──read──► new reality
```

The consequence is profound: **domains do not know each other exist.** Physical Reality contains no reference to Conflict — not a callback, not an event subscription, not an enum value. It defines what walls are; anything at all may propose changes to walls; its own rules judge those proposals' physical coherence.

This is why Chapter 11's emergence is possible: interactions that were never designed can still occur, because interaction requires no design — only shared reality.

## The Ownership Matrix

Every fact type in a world has exactly one owning domain. The full assignment — established across Chapters 1 through 10 — forms the world's ownership matrix, and three rules govern it:

1. **Owners define and validate.** The owning domain declares the fact type's schema, semantics, and coherence rules. Proposals from anywhere are judged by the owner's rules at commit.
2. **Consumers read freely.** Any domain may read any committed fact. Reading requires no permission, no registration, and no knowledge by the owner.
3. **Views are not copies.** A domain needing another's facts in a different shape (Economy needing "distance," which Physical Reality owns as topology) derives views and caches at its own risk — derived views are never authoritative, per Chapter 1's single-source rule.

## Joint Stocks: The Shared-Truth Pattern

Some facts genuinely serve two masters — a fishery is Ecology's population and Resources' stock. The volume's answer, stated in Chapters 3 and 10, generalizes:

**One underlying fact, one owner, multiple views.**

The population count is owned once (Ecology). Resources' "stock" is a view over it. Both domains' systems propose changes through the same accounting, so overfishing is *automatically* an ecological event. Any design that duplicates a truth so two domains can each own "their copy" is a divergence factory, and forbidden.

---

# 12.2 Effect Chains

Cross-domain causation happens in *chains of commits*, never in single spans.

A raid burns a granary:

```text
Tick N:    Conflict proposes Δ(granary integrity), Δ(grain destroyed)
           Commit. Events recorded.

Tick N+1+: Economy's systems read less grain → prices shift at next clearing
           Society's systems read the deaths → households restructure
           Information carries the news outward at travel speed
           Institutions read the failure to protect → legitimacy debits
           Culture, years later, reads the accumulated memory → the Burning enters lore
```

Note what the chain requires of Conflict: **nothing.** Conflict burned a granary. Every consequence was another domain reading committed reality on its own schedule, at its own timescale.

## Timescale Bridging

Domains legitimately run at different cadences — combat in seconds, markets daily, succession in decades (Volume II, Chapter 2 scheduling). Effect chains bridge timescales automatically, because facts persist: the granary stays burned until the slow domains get around to noticing.

No fast domain waits for a slow one.

No slow domain misses what a fast one did.

## Conflict Resolution Between Domains

When two domains propose incompatible changes in one tick — fire consuming a wall that soldiers are repairing — resolution follows Volume II's scheduler contract, with one addition at domain scale:

**Precedence is declared in the world package, per fact type, and owned by the fact's owner.**

Physical Reality declares how competing physical proposals compose (damage and repair may both apply, in declared order). No proposing domain ever "wins" by being special; there is no priority of importance, only declared composition rules.

---

# 12.3 Adding a Domain

The architecture's health is measured by a single test, stated in Chapter 1 and due here in full:

**A new domain must require zero changes to existing domains.**

The checklist for introducing a domain — say, **Religion** as a first-class domain rather than Culture-plus-Institutions:

1. **Claim facts.** Declare the new fact types it owns (rites-in-standing, divine claims, sacred statuses). Verify against the ownership matrix that no existing owner already holds them. Overlap discovered means the design is wrong — negotiate the boundary *on paper*, never in code.
2. **Declare reads.** List the committed facts it consumes (deaths, settlements, lore, offices). This requires nothing from those owners.
3. **Register systems.** Its systems enter the ordinary scheduler with ordinary read/write declarations and cadences.
4. **Extend the world package.** Its content — creeds, rites, calendars — is data, like every domain's content.
5. **Prove non-interference.** The reference world, run with the domain disabled, is byte-identical to before. Run with it enabled, existing domains' rules are untouched — they merely find new facts in reality to read, or not.

The same checklist, run in reverse, removes a domain: a world package that selects only six of twelve domains (Volume IV's domain selection) simply has no systems proposing those fact types — and no other domain notices, because no other domain ever knew.

---

# Designer Note
## The Boundary Disputes Are the Design Work

Nearly every hard decision in this volume was a boundary dispute.

Is reputation Society's or Information's? (Society's — it is the *aggregate*, not the individual belief.) Is a fishery Ecology's or Resources'? (Ecology owns the count; Resources views it.) Is law Culture's or Institutions'? (The norm is Culture's; the recorded, sanctioned rule is Institutions'.) Is hunger a behavior? (No — biology Living Systems, decision elsewhere, forever.)

This is not pedantry. Every one of these disputes, settled sloppily, becomes a divergence bug: two owners, two truths, and a world that disagrees with itself.

When designing for Cardinal, expect to spend most of your time not writing rules but deciding *whose* rules. The chapters of this volume are precedent. Argue from them the way lawyers argue from case law — and when a genuinely new boundary appears, settle it explicitly, in the specification, before a line of implementation exists.

The seams are the architecture.

---

# 12.4 Common Queries

Asked across domains, and answerable only because interaction law holds:

- Who owns this fact type, and what are its composition rules?
- What domains consume facts of this type? (tooling: reverse-dependency analysis)
- What proposals touched this fact this tick, from which systems, in what resolved order?
- Which committed events explain this cross-domain chain? (Chapter 11's causal audit)
- What would this world look like with domain X disabled? (composition testing)
- Where do two domains hold near-duplicate facts? (divergence risk detection)

---

# 12.5 Architectural Contracts

1. Domains interact exclusively through committed reality.
2. No domain holds a reference to, subscribes to, or invokes any other domain.
3. Every fact type has exactly one owning domain; the ownership matrix is explicit and complete.
4. Owners define schema, semantics, and proposal-composition rules for their facts.
5. Any system may propose changes to any fact; the owner's rules judge all proposals identically.
6. Shared truths are single facts with derived views, never parallel copies.
7. Cross-domain effects propagate as commit chains across ticks, bridging timescales through persistent facts.
8. Domain sets are world-package selections; every domain is optional and independently removable.

---

# 12.6 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Zero direct dependencies exist between domain implementations.
2. The ownership matrix has no gaps and no overlaps.
3. Proposal composition per fact type is declared, deterministic, and owner-defined.
4. No proposing domain receives precedence by identity.
5. Derived views are never written back as authority.
6. Effect chains are fully reconstructable from the event record.
7. Adding a domain changes no existing domain's code, data, or results-when-disabled.
8. Removing a domain leaves remaining domains' behavior well-defined.
9. Cross-domain consistency is enforced at commit, not by inter-domain negotiation.
10. Every boundary decision is recorded in the specification before it is implemented.

---

# 12.7 Anti-Patterns

### The Domain Call

`economy.notify_price_change(...)` from anywhere. The moment domains call each other, the ownership matrix is decorative and the coupling clock starts.

### The God Domain

A "world manager" that orchestrates the others. There is no orchestrator but the scheduler, and the scheduler knows systems, not domains.

### The Private Treaty

Two domains sharing a side-channel — a common cache, a special struct, a bilateral update protocol. Every treaty is invisible coupling that the third domain, and the tenth, will eventually trip over.

### The Copied Truth

"We keep our own population count for performance." Divergence is not a risk but a schedule. Derive views; never fork facts.

### The Mandatory Domain

Engine code that assumes a domain exists ("every world has an economy"). Every domain is a world-package choice; a wilderness world with no Economy, Society, or Institutions must run flawlessly.

### Boundary by Accident

Letting implementation decide ownership ("whoever wrote it first owns it"). Boundaries are specification decisions with precedent — Chapter by Chapter — and they are made before code.

---

# 12.8 Volume III in Retrospect

The twelve chapters of this volume form a single argument:

Reality can be partitioned into domains **(1–10)**, the partition produces phenomena no partition member owns **(11)**, and the partition composes back into one lawful whole **(12)** — provided every seam obeys the same small law: one owner per fact, reads for all, proposals judged at commit, and no domain aware of any other.

What remains is assembly.

---

## Preparing for Volume IV

Volume III defined what can exist in a Cardinal world.

Volume IV — **World Packages** — defines how a particular world chooses among it: which domains, which rules, which content, and which initial conditions turn an architecture of everything into a simulation of *somewhere*.

---

# END OF CHAPTER 12
# END OF VOLUME III
