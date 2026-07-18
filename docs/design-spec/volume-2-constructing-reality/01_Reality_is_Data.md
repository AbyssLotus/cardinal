# Volume II
# Constructing Reality
## Chapter 1
# Reality is Data

> *"Reality is not made of objects. It is made of facts."*

---

## The First Engineering Principle

Volume I established that reality is the authoritative source of truth. It defined the philosophical principle that nothing within the simulation may alter reality through observation, narration, or belief. Reality exists independently of those who perceive it.

Volume II now asks a different question.

If reality is authoritative, how should it be represented inside a computer?

The answer is surprisingly simple.

Reality is data.

Not objects.

Not classes.

Not scripts.

Not behavior trees.

Not artificial intelligence.

Reality is an evolving collection of facts describing the current state of existence.

Every other system within Cardinal exists to discover, transform, preserve, or reason about those facts.

This distinction appears subtle, but it changes the architecture of the entire engine.

Traditional engines ask:

> *What object is this?*

Cardinal asks:

> *What is currently true?*

The engine never reasons about abstractions like "villager" or "dragon." Those are merely convenient labels assigned by humans. The simulation reasons only about measurable state.

A villager is simply an entity possessing a collection of facts.

A dragon is another entity possessing a different collection of facts.

Neither possesses special status within the engine.

Only facts matter.

---

## Reality Exists Independent of Interpretation

Imagine a tree.

A traditional engine might represent it as:

```text
Tree
 ├── Position
 ├── Health
 ├── Species
 ├── Mesh
 ├── Inventory
```

Cardinal does not begin there.

Instead it begins with truth.

```text
Entity #58231

Position = (104, 22)
Height = 11.2 m
Mass = 1,480 kg
Species = Oak
Age = 94 years
Water = 67%
Temperature = 18.3°C
Alive = True
Burning = False
```

There is no "Tree."

The engine never needs that concept.

Reality contains facts.

Humans interpret those facts.

Another world package may interpret an equivalent collection of facts as a giant mushroom.

The engine should never care.

Reality should never contain concepts.

Reality contains facts.

---

## Truth is Atomic

Every fact should represent one indivisible statement.

Good:

```text
Health = 82
```

Bad:

```text
Healthy = True
```

"Healthy" is interpretation.

Measurements belong in reality.

Interpretation belongs elsewhere.

The same applies to wealth, power, beauty, danger, and every other contextual concept.

Reality stores evidence.

Observers derive meaning.

---

## Facts Have Identity

Every fact possesses provenance.

```text
Fact
├── Owner
├── Type
├── Value
├── Timestamp
├── Source
└── Metadata
```

This allows Cardinal to answer questions most engines cannot.

- Who changed this?
- When?
- Why?
- Which subsystem produced it?

Every fact has a history.

---

## Reality is a Graph

Reality is not a list.

Reality is a graph of entities connected through relationships.

Ownership.

Friendship.

Citizenship.

Trade.

Marriage.

Parenthood.

Roads.

Rivers.

Political allegiance.

Every relationship is itself a first-class fact.

---

## Behavior Does Not Exist

Behavior is never stored.

Behavior emerges.

Instead of:

```text
Wolf
 ├── Hunt()
 ├── Patrol()
 ├── Sleep()
```

Cardinal stores:

```text
Energy = 18%
Hunger = 91%
Threat = None
Nearby Deer = True
Pack Nearby = True
Time = Dawn
```

Decision systems transform those facts into new facts.

Behavior is an outcome, not a property.

---

## Identity Persists

Everything changes.

Identity does not.

Children become adults.

Trees become lumber.

Kingdoms become empires.

Without persistent identity there can be no continuity.

Without continuity there can be no history.

Identity is therefore sacred.

---

## Reality is Complete

Reality never contains "unknown."

Reality always knows.

Observers may not.

Reality:

```text
Ore = Iron
Mass = 4281 kg
```

Miner:

```text
Unknown
```

Geologist:

```text
Probably Iron
```

Knowledge varies.

Reality does not.

---

## Derived Data

Population.

GDP.

Crime rate.

Military strength.

These are observations.

Reality stores only the underlying facts.

Derived values should be recomputed whenever possible rather than persisted.

---

## Atomic Change

Reality is never modified one statement at a time.

Systems propose changes.

Conflicts are resolved.

The world commits atomically.

Every simulation tick produces one coherent new reality.

Never a partially updated one.

---

## The Shape of Reality

Everything inside Cardinal ultimately becomes one of four things.

### Entities

Persistent identities.

### Facts

Atomic statements describing reality.

### Relationships

Connections between entities.

### Events

Irreversible changes recorded in history.

Events explain how reality changed.

They are not reality itself.

---

## Engineering Invariants

Every implementation of Cardinal SHALL preserve these principles.

1. Reality is the only authoritative state.
2. Reality consists of atomic facts.
3. Facts possess provenance.
4. Relationships are first-class citizens.
5. Behavior emerges from state.
6. Identity persists through change.
7. Knowledge is separate from reality.
8. Derived data should not be persisted when it can be recomputed.
9. World updates are atomic.
10. History records change but never replaces reality.

These invariants are constitutional. Every future subsystem must uphold them regardless of language, optimization strategy, or implementation details.
