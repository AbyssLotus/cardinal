# Volume II
# Constructing Reality
## Chapter 3
# Systems

> *A system does not own reality. A system transforms reality.*

---

## The Purpose of a System

Reality is passive.

Systems are the mechanisms that evolve it.

A system observes the current world state, evaluates a well-defined set of rules, and proposes changes. It never owns data and never permanently stores truth. Its responsibility begins with reading reality and ends with producing a deterministic set of proposed modifications.

This separation is one of Cardinal's most important architectural boundaries.

- Reality stores facts.
- Systems transform facts.

Nothing else.

---

## Systems are Pure Transformations

Conceptually, every system follows the same pattern.

```text
Committed World State
        │
        ▼
Read Relevant Facts
        │
        ▼
Evaluate Rules
        │
        ▼
Produce Proposed Changes
        │
        ▼
Scheduler
```

The output of a system is never "behavior."

The output is always a collection of state transitions.

Example:

```text
Input

Health = 24
Bleeding = True

↓

Output

Health = 22
```

Another system may later interpret low health as a reason to flee.

Each system performs one transformation well.

---

## Systems Do Not Call Each Other

A common anti-pattern in simulation engines is direct coupling.

```text
Combat

↓

Inventory

↓

Quest

↓

Dialogue
```

This creates implicit dependencies and unpredictable execution.

Cardinal systems never invoke one another.

Instead they communicate exclusively through reality.

```text
Combat

↓

Reality Updated

↓

Quest observes change

↓

Dialogue observes change
```

Reality becomes the integration point.

---

## Single Responsibility

Every system should have one clearly defined purpose.

Good:

- Movement
- Weather
- Agriculture
- Hunger
- Reputation

Poor:

- NPC Manager
- World Manager
- Gameplay System

Broad systems become difficult to reason about and difficult to parallelize.

Prefer many focused systems over a few monolithic ones.

---

## Read Sets and Write Sets

Every system should explicitly declare:

- the facts it reads
- the facts it may modify

Example:

```text
Movement

Reads

Position
Velocity
Terrain

Writes

Position
Velocity
```

Combat

```text
Reads

Health
Armor
Weapon

Writes

Health
Status Effects
```

Explicit read/write declarations allow:

- dependency analysis
- conflict detection
- parallel scheduling
- tooling
- optimization

---

## Deterministic Evaluation

Given identical inputs, a system must always produce identical outputs.

This includes:

- rule evaluation
- iteration order
- random events
- floating-point behavior
- conflict handling

Hidden state inside a system violates determinism.

A system should behave as though it were a mathematical function.

---

## Idempotence Within a Tick

A system should evaluate reality exactly once per tick.

Running the same system twice against the same committed world should produce the same proposals.

This prevents accidental cascading effects during scheduling.

---

## Domain Independence

Cardinal intentionally avoids "special" systems.

The agriculture system follows the same architectural contract as combat.

The weather system follows the same contract as politics.

The engine distinguishes systems by data, not importance.

This allows new domains to be introduced without modifying the scheduler.

---

## Long-Running Processes

Many phenomena span thousands of ticks.

Examples include:

- crop growth
- erosion
- learning
- aging
- civilization development

These are not special cases.

Each tick contributes a small deterministic transformation until the accumulated state reaches a meaningful threshold.

Time changes the frequency of execution, not the architecture.

---

## Ordering

Some systems depend on the output of others.

Example:

```text
Movement
    ↓
Collision
    ↓
Damage
    ↓
Death
```

Dependencies must be declared explicitly.

The scheduler constructs an execution order from these declarations.

Ordering should never be encoded through hidden assumptions.

---

## Failure

Systems should fail safely.

A failed system must never leave reality partially updated.

If evaluation cannot complete, the current tick is aborted before commit.

Consistency is always more important than progress.

---

## Extensibility

Adding a new simulation domain should require only:

1. Defining new facts.
2. Implementing one or more systems.
3. Registering scheduling information.

No engine code should require modification simply because a new domain exists.

This is how Cardinal remains world-agnostic.

---

## Engineering Invariants

Every implementation SHALL preserve these rules.

1. Systems never own persistent state.
2. Systems only transform committed reality.
3. Systems communicate through reality, never direct calls.
4. Every system declares explicit read and write sets.
5. System evaluation is deterministic.
6. Hidden mutable state inside systems is prohibited.
7. Systems produce proposed changes rather than directly mutating reality.
8. Dependencies are declared, never implied.
9. Systems are domain-independent.
10. Adding new systems must not require changes to the engine architecture.

---

## Common Implementation Mistakes

### Fat Systems

Large systems that mix unrelated concerns become difficult to schedule, parallelize, and test.

### Shared Mutable State

Allowing systems to cache mutable simulation state creates hidden dependencies and nondeterministic behavior.

### Cross-System Calls

Directly invoking another system bypasses the scheduler and destroys architectural isolation.

### Hidden Reads

Reading facts that were never declared makes dependency analysis impossible.

---

## Future Considerations

Future versions of Cardinal may introduce:

- automatic dependency graph generation
- incremental system evaluation
- adaptive scheduling
- distributed simulation
- speculative execution

These optimizations must remain invisible to the simulation itself.

They may improve performance.

They may never change the resulting reality.
