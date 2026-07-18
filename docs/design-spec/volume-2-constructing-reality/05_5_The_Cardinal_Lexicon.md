# Volume II
# Constructing Reality
## Chapter 5.5 (Addendum)
# The Cardinal Lexicon

> *Large systems fail from inconsistent assumptions long before they fail from incorrect code.*

---

## Purpose

Volumes I and II define Cardinal's architecture. This addendum defines its vocabulary.

Architecture only scales when contributors share the same mental model. If two engineers use the same word to mean different things, the implementation will slowly diverge from the design.

The following terms are normative. Throughout Cardinal documentation, code, RFCs, and discussions, they should carry these meanings unless explicitly redefined.

---

# Reality

**Reality** is the complete, authoritative state of the simulated world at a specific simulation tick.

Reality is objective.

Reality does not depend on observation, memory, or belief.

There is exactly one reality.

---

# Fact

A **Fact** is the smallest atomic statement that can be true about reality.

Examples:

```text
Position = (14, 82)

Health = 91

Owner = Entity 52

Temperature = 22°C
```

Facts are atomic.

Facts possess provenance.

Facts never contain interpretation.

---

# Entity

An **Entity** is a persistent identity to which facts may be attached.

Entities are not classes.

Entities are not behavior.

Entities simply exist.

Everything else is described through facts and relationships.

---

# Relationship

A **Relationship** is an explicit connection between two or more entities.

Examples include:

- owns
- knows
- lives_in
- married_to
- parent_of

Relationships are first-class simulation data.

---

# System

A **System** is a deterministic transformation of committed reality.

Systems:

- read reality
- evaluate rules
- propose changes

Systems never own truth.

---

# Tick

A **Tick** is the smallest indivisible unit of simulation.

All accepted changes become visible simultaneously at the end of a tick.

Reality never changes between ticks.

---

# Proposal

A **Proposal** is a requested modification produced by a system.

Examples:

```text
Health

91 → 73

Position

(5,4) → (6,4)
```

A proposal is not reality.

It becomes reality only after commitment.

---

# Commit

A **Commit** is the atomic acceptance of all valid proposals for a simulation tick.

After commitment:

- reality advances
- history grows
- persistence may record the new world

No observer ever experiences a partially committed world.

---

# Event

An **Event** records that reality changed.

Events explain *why* reality transitioned from one state to another.

Reality describes what is true.

Events describe how it became true.

---

# Information

**Information** is an observer's local understanding of reality.

Information may be:

- incomplete
- uncertain
- outdated
- incorrect

Reality is singular.

Information is personal.

---

# Observation

An **Observation** is information acquired directly from reality.

Observation is constrained by world rules such as vision, sensors, communication, or magic.

Observation never alters reality.

---

# Knowledge

**Knowledge** is information currently accepted by an observer as true.

Knowledge belongs to entities.

It is never global.

---

# Memory

**Memory** is historical knowledge retained after observation.

Memories can become stale.

Reality does not.

---

# Belief

A **Belief** is an interpretation constructed from knowledge and memory.

Beliefs influence decisions.

Beliefs do not define reality.

---

# Decision

A **Decision** transforms information into intended action.

Decision systems consume beliefs rather than world state.

---

# Action

An **Action** is an attempt to modify reality.

Actions produce proposals.

Actions do not directly mutate the world.

---

# Persistence

**Persistence** is the long-term preservation of committed reality.

Persistence is responsible for ensuring a world survives beyond the execution of a process.

It preserves reality.

Not behavior.

---

# Determinism

**Determinism** guarantees that identical inputs always produce identical realities.

This property enables replay, debugging, synchronization, and verification.

---

# World Package

A **World Package** defines the rules of a particular universe.

It specifies domains such as:

- physics
- biology
- economics
- culture
- institutions

The engine executes rules.

The world package defines them.

---

## Architectural Grammar

Most discussions about Cardinal can be expressed using a surprisingly small grammar:

```text
Reality
    ↓
Systems
    ↓
Proposals
    ↓
Commit
    ↓
Reality
```

or

```text
Reality
    ↓
Observation
    ↓
Knowledge
    ↓
Memory
    ↓
Belief
    ↓
Decision
    ↓
Action
    ↓
Reality
```

If a new feature cannot be placed naturally within one of these loops, its design should be questioned before implementation.

---

## Closing Thoughts

This lexicon is intentionally small.

New terminology should be introduced only when it represents a genuinely new architectural concept.

A shared vocabulary reduces ambiguity, improves design discussions, simplifies documentation, and makes the engine easier to evolve over decades.

When contributors use the words *Reality*, *Fact*, *Proposal*, or *Commit*, there should be no debate about their meaning.

The architecture begins with shared language.
