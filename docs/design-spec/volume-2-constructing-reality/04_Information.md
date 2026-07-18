# Volume II
# Constructing Reality
## Chapter 4
# Information

> *Reality is universal. Information is local.*

---

## Why Information Deserves Its Own Architecture

Reality and information are not the same thing.

Reality describes what is true.

Information describes what an observer believes to be true.

Confusing these concepts is one of the most common mistakes in simulation design. Many games implicitly grant every AI perfect knowledge because they read directly from the world state.

Cardinal deliberately forbids this.

Every intelligent decision is made from information, never from reality itself.

---

## The Information Pipeline

Information always flows in one direction.

```text
Reality
    │
    ▼
Observation
    │
    ▼
Knowledge
    │
    ▼
Memory
    │
    ▼
Belief
    │
    ▼
Decision
    │
    ▼
Action
    │
    ▼
Reality
```

Every intelligent entity participates in this loop.

Systems that bypass it produce omniscient actors.

---

## Observation

Observation is the process of acquiring information from reality.

Observations may be constrained by:

- vision
- hearing
- distance
- weather
- lighting
- obstructions
- technology
- magical rules defined by a world package

Observation is never guaranteed.

Failure to observe does not change reality.

---

## Knowledge

Knowledge is verified information available to an entity.

Examples:

```text
Known Location
Known Identity
Known Ownership
Known Danger
```

Knowledge is local.

Two entities occupying the same world may possess entirely different knowledge.

Knowledge may also expire as the world changes.

---

## Memory

Knowledge becomes memory when it persists beyond immediate observation.

Memory contains historical information.

```text
Saw wolf yesterday.

Merchant charged 5 gold.

Bridge collapsed.
```

Memory does not update automatically.

If the bridge has since been rebuilt, an entity still remembers the collapse until new observations replace or refine that memory.

---

## Belief

Beliefs are interpretations of knowledge and memory.

They are not required to be correct.

Examples:

```text
The king is trustworthy.

The forest is haunted.

The merchant cheats customers.
```

Beliefs influence future decisions.

Reality does not.

---

## Uncertainty

Information should carry confidence where appropriate.

Example:

```text
Bandits Nearby

Confidence: 0.82
```

This allows agents to make decisions under uncertainty rather than binary certainty.

World packages may define additional uncertainty models.

---

## Information Sources

Every piece of information has an origin.

Possible sources include:

- direct observation
- conversation
- documents
- institutions
- inference
- rumors
- dreams
- sensors
- magical effects

Recording provenance enables trust systems to reason about credibility.

---

## Information Decay

Information becomes stale.

Examples:

- enemy locations
- market prices
- weather forecasts
- troop movements

Entities should periodically reduce confidence in aging information unless refreshed.

Decay models are domain-specific but deterministic.

---

## Sharing Information

Information spreads through explicit mechanisms.

Examples:

```text
Conversation

Letter

Town Crier

Trade Network

Religious Institution
```

Information never teleports.

Propagation has cost, delay, and fidelity.

This makes communication infrastructure part of the simulation rather than a scripting convenience.

---

## Misinformation

False information is first-class simulation data.

Reality may state:

```text
King Alive = True
```

A rumor may state:

```text
King Dead
Confidence: 0.61
Source: Traveler
```

Both coexist.

The contradiction exists in information, not in reality.

---

## Decisions

Decision systems consume beliefs, knowledge, and goals.

They do not consume reality directly.

Two entities presented with identical reality but different beliefs should be capable of making different choices.

This enables diplomacy, fear, religion, propaganda, and scientific discovery to emerge from the same architecture.

---

## Information and Reality

Reality never changes because someone believes something.

Information may eventually change reality through action.

This distinction preserves causality.

```text
Reality

↓

Information

↓

Decision

↓

Action

↓

Reality
```

No arrow skips a stage.

---

## Engineering Invariants

Every implementation SHALL preserve these rules.

1. Reality and information are separate architectural layers.
2. Observation reads reality but never modifies it.
3. Knowledge belongs to individual observers.
4. Memory is historical and may become outdated.
5. Beliefs may be incorrect.
6. Decisions operate on information, not reality.
7. Every information source records provenance.
8. Information propagates through explicit mechanisms.
9. Uncertainty is represented explicitly where appropriate.
10. Information cannot alter reality except through actions.

---

## Common Implementation Mistakes

### Omniscient AI

Allowing decision systems to query world state directly eliminates exploration, deception, and uncertainty.

### Shared Knowledge

Global knowledge stores remove individuality and invalidate local reasoning.

### Perfect Communication

Instant information transfer destroys logistics, reconnaissance, and institutional importance.

### Mixing Beliefs with Facts

Never overwrite reality because enough entities believe something.

Beliefs influence behavior.

Facts define reality.

---

## Future Considerations

Future versions of Cardinal may support:

- probabilistic reasoning
- trust networks
- institutional knowledge
- language translation layers
- collective memory
- scientific models
- misinformation campaigns

Each extends the information architecture without changing its fundamental contract:

Reality is singular.

Information is personal.
