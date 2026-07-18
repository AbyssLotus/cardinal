# Cardinal Definitive Reference
## Volume I: First Principles
### Chapter 2: The Ontology of a Cardinal World

> **Before Cardinal can simulate a world, it must define what kinds of things may exist within one.**

---

## 1. Purpose of This Chapter

Architecture becomes unstable when fundamental terms remain intuitive. Words such as *reality*, *entity*, *event*, *knowledge*, *memory*, and *history* are often used interchangeably in game systems even though they refer to different phenomena. Cardinal cannot support durable causality if its ontology is ambiguous.

This chapter establishes the canonical conceptual model of a Cardinal world. It does not require that every concept map to one database table or one class. It defines semantic boundaries that implementations must preserve regardless of storage technology.

The ontology is intentionally compact. Cardinal should be capable of representing immense variety through combinations of a limited number of concepts. When a proposed feature appears to require a new fundamental category, contributors should first determine whether it can be expressed as a composition of the concepts defined here.

---

## 2. Reality

**Reality** is the complete canonical state of a Cardinal world at a particular simulation time.

Reality includes all committed facts required to continue the simulation correctly, whether or not any observer knows them. It may include:

- entities and their current state;
- spatial and topological relationships;
- environmental conditions;
- active processes;
- ownership and obligations;
- resources and conserved quantities;
- institutional structures;
- causal links and historical records;
- scheduled future work whose preconditions have already been established;
- world-defined laws and parameters.

Reality does not include uncommitted proposals, narrator embellishments, guesses, cached UI summaries, or beliefs merely because an agent holds them.

Reality is authoritative but not necessarily accessible. Most agents perceive a narrow projection of it. Even engine systems should read only the state relevant to their responsibilities wherever practical, preserving clear dependencies and limiting accidental omniscience.

A reality snapshot is not the world’s history. It is the current result of that history. Cardinal requires both current state and sufficient historical structure to explain how that state arose.

---

## 3. World

A **world** is a lawful, temporally evolving reality instantiated from an engine, a validated world package, and an initial condition.

A world consists of four inseparable elements:

1. **Laws**, defining permitted transitions and invariant constraints.
2. **State**, representing the present configuration of reality.
3. **Time**, ordering change and determining when processes advance.
4. **History**, preserving committed transitions and their enduring consequences.

A world package supplies meaning, parameters, definitions, and initial content. The engine supplies general mechanisms for validation, scheduling, persistence, transition, observation, and extension. A save or running instance supplies concrete history.

Two worlds created from the same package are not the same world once their committed histories diverge. Identity belongs to the instance and its causal lineage, not merely to its template.

---

## 4. Time

**Simulation time** is the ordered coordinate by which state transitions are sequenced.

Time in Cardinal is not equivalent to wall-clock time. A world may advance faster, slower, intermittently, or in batches. What matters is that every committed change occupies a defined position in the world’s temporal order.

Cardinal recognizes multiple temporal resolutions:

- sub-action intervals, such as combat rounds;
- action intervals, such as movement or work;
- hourly or circadian intervals;
- daily ecological and economic intervals;
- seasonal and annual intervals;
- generational or historical intervals.

These resolutions are scheduling strategies, not separate realities. A change computed at a coarse scale must remain compatible with the state that would matter when a region returns to fine simulation.

Time should support:

- deterministic ordering;
- explicit duration;
- interruption and rescheduling;
- delayed effects;
- recurrence;
- deadlines and expiration;
- age and lifecycle progression;
- historical queries.

No system may silently advance an entity through time without recording the corresponding state transition or abstraction result.

---

## 5. Entity

An **entity** is a persistently identifiable participant in world state.

An entity may represent a person, animal, monster, object, vehicle, building, location, organization, weather system, resource deposit, legal instrument, or any other thing whose identity matters across transitions.

Entity identity must be stable. State may change radically while identity persists. A sword may be forged, owned, damaged, stolen, renamed, reforged, displayed, and destroyed while remaining one historical entity. If its material is later used to create another sword, the new sword is a new entity with provenance linking it to the former one.

An entity should exist only when identity has simulation value. Fungible quantities such as grain in a warehouse may be represented as stocks until individual provenance becomes important. Cardinal should not instantiate millions of objects merely to imitate detail. Identity is a semantic cost and should be paid where continuity matters.

Entities possess state through components, records, or equivalent data-oriented structures. The ontology does not require inheritance hierarchies. It requires that entity identity and entity state remain distinguishable.

---

## 6. State

**State** is the set of currently committed properties and relationships necessary to determine valid future transitions.

State includes direct attributes and relational facts. For an agent, state may include health, needs, location, inventory, commitments, goals, memories, beliefs, and institutional roles. For a settlement, it may include population, infrastructure, stores, laws, leadership, market conditions, and cultural tendencies.

State should be:

- explicit enough to inspect;
- serializable;
- validated against schemas and invariants;
- mutated only through authorized transitions;
- free from narrator-only invention;
- compatible with deterministic replay.

Derived values need not always be persisted. They may be computed from authoritative state when doing so is deterministic and affordable. However, expensive or historically significant derivations may be cached if the cache can be invalidated or reconstructed reliably.

Hidden mutable state inside system objects is dangerous. If it can alter simulation outcomes, it belongs in canonical persistence or in a deterministic derivation from persisted inputs.

---

## 7. Process

A **process** is change that unfolds over simulation time rather than occurring as an indivisible transition.

Examples include:

- travel;
- healing;
- construction;
- pregnancy and growth;
- disease progression;
- institutional elections;
- crop growth;
- weather movement;
- research;
- rumor diffusion;
- decay.

Processes have identity when interruption, ownership, provenance, or concurrent interaction matters. They should declare:

- initiating cause;
- participants;
- start time;
- expected or conditional duration;
- resource commitments;
- progress state;
- interruption rules;
- completion and failure outcomes.

A process is not merely a timer. It is an active causal commitment within the world. If a character begins building a bridge, materials may become reserved, labor becomes unavailable elsewhere, and the unfinished structure may itself alter reality.

---

## 8. Action and Decision

An **action** is an attempted state transition initiated by an actor or external controller.

A **decision** is the selection of an intended action or plan from perceived alternatives.

The distinction matters because an agent may decide to act but fail to execute, or execute an action whose result differs from expectation.

A decision belongs to the epistemic world of the actor. It is based on:

- perceived needs and goals;
- beliefs about reality;
- predicted outcomes;
- preferences and values;
- relationships and obligations;
- available capabilities;
- perceived risk;
- temporal urgency.

An action enters canonical reality only through validation and resolution. Intent does not guarantee success. The world determines whether the action is possible and what consequences follow.

Cardinal should preserve enough decision context to explain important behavior without logging every trivial utility calculation forever. Significant decisions may become memories, institutional records, or causal antecedents.

---

## 9. Event

An **event** is a committed occurrence that marks a meaningful transition in world state.

An event should answer:

- what changed;
- when it changed;
- which entities participated;
- which causes enabled or triggered it;
- which rules resolved it;
- which direct consequences were committed;
- who could perceive it.

Not every field update requires a permanent high-level event. Cardinal may use low-level deltas for efficient state mutation and higher-level semantic events for history, memory, causality, and narration. The two must remain reconcilable.

Events are immutable after commit. Later discoveries may change beliefs about an event, and later records may reinterpret it, but the canonical occurrence is not rewritten.

Corrections to corrupted data or migrations must be represented as explicit technical operations, not disguised as changes to historical truth.

---

## 10. Delta and Transaction

A **delta** is a proposed atomic mutation to canonical state.

A **transaction** is the validated, ordered set of deltas committed together as one simulation boundary.

Cardinal’s current simulation loop correctly treats each turn or tick boundary as an atomic commit. This principle must continue as systems grow more complex. No observer should encounter a half-resolved event in which currency was removed but goods were not transferred, an entity died but its active actions continued, or an institution changed leadership without authority records updating.

Transactions should provide:

- precondition validation;
- deterministic ordering;
- invariant enforcement;
- all-or-nothing commit;
- causal metadata;
- auditability;
- recoverability after process failure.

Systems should propose consequences rather than mutating persistence opportunistically. The transaction layer is the membrane between possibility and reality.

---

## 11. Cause

A **cause** is a prior condition, event, decision, or process without which a later event would not have occurred in the same way.

Cardinal should represent causality at useful granularity rather than attempting philosophical completeness. A causal graph may include:

- initiating inputs;
- enabling conditions;
- blocking conditions that were removed;
- decisions;
- resource dependencies;
- institutional authorizations;
- stochastic resolutions;
- immediate parent events.

For example, a character’s death may cite the damaging attack as its immediate cause, the combat encounter as context, the attacker’s goal as motivational ancestry, and a prior bridge collapse as a logistical contributor if it prevented medical aid.

Causality supports debugging, narration, historical explanation, agent reasoning, and counterfactual analysis. It should not become an unbounded duplication of all world state. Systems must identify which antecedents are semantically meaningful.

---

## 12. Fact

A **fact** is a proposition about canonical reality with a truth condition grounded in world state or committed history.

Examples:

- Entity A is currently in Location B.
- The bridge was destroyed at Time T.
- Institution C owns Asset D.
- Agent E performed Action F.

Facts may be current, historical, temporal, relational, or derived. A fact’s truth may change over time without retroactively changing whether it was true earlier.

Facts belong to reality. Agents do not automatically possess them.

This separation is foundational:

> Reality contains facts. Observers contain beliefs.

A knowledge system that stores “known facts” directly on agents without provenance or uncertainty will eventually collapse truth and belief into one field. Cardinal must resist this shortcut.

---

## 13. Observation

An **observation** is information produced when an observer’s perceptual capabilities interact with accessible reality.

Observation depends on:

- spatial access;
- sensory capability;
- attention;
- visibility and concealment;
- environmental interference;
- expertise;
- instrumentation;
- timing;
- world-specific perception laws.

An observation is not guaranteed to be complete or correct. It should preserve the method by which it was acquired. Seeing smoke, reading a ledger, hearing a rumor, inspecting tracks, receiving a sensor signal, and recalling a memory are different epistemic operations.

Perception converts reality into evidence. Cognition converts evidence into belief.

---

## 14. Belief and Knowledge

A **belief** is a proposition held by an epistemic actor, accompanied by confidence, provenance, and interpretation.

Cardinal may use **knowledge** as a convenient term for strongly supported beliefs, but the engine should not create an absolute category that silently grants agents access to truth. Even reliable knowledge has a source and a temporal context.

A belief record should be capable of expressing:

- proposition or reference;
- subject and predicate;
- believed value;
- confidence;
- source;
- acquisition time;
- last confirmation time;
- method of acquisition;
- visibility or secrecy;
- emotional salience;
- transmission history;
- contradiction links;
- status such as suspected, accepted, disputed, or rejected.

Beliefs may be true, false, obsolete, partially true, or impossible to verify. Multiple agents may hold incompatible beliefs about the same fact. Institutions may possess official positions that differ from the private beliefs of their members.

This enables rumor, deception, propaganda, investigation, scholarship, testimony, and discovery as simulation rather than flavor text.

---

## 15. Memory

A **memory** is a persistent representation of an experience, observation, decision, or learned proposition associated with an actor or institution.

Memory is not a perfect copy of history. It is selective, situated, and mutable in salience and interpretation.

Memories should support:

- origin in an event or observation;
- participants and subjects;
- emotional weight;
- confidence;
- accessibility;
- decay or reinforcement;
- retelling;
- reinterpretation;
- influence on relationships and decisions.

The world chronicle, personal memory, and institutional record are distinct.

- The **chronicle** preserves selected canonical events.
- A **personal memory** records what an individual retained.
- An **institutional record** preserves what an organization encoded and can access.

These may disagree without corrupting reality.

---

## 16. Relationship

A **relationship** is persistent state describing how entities are connected socially, legally, materially, spatially, or causally.

Social relationships should not be reduced to one reputation number. They may include independent dimensions such as:

- trust;
- affection;
- respect;
- fear;
- loyalty;
- resentment;
- rivalry;
- obligation;
- debt;
- kinship;
- ideological alignment.

Different decisions consult different dimensions. A guard may respect a criminal’s competence, fear their violence, distrust their promises, and still feel personal affection due to shared childhood.

Relationships change through events, beliefs, memories, institutional rules, and cultural expectations. They are both consequences of history and causes of future behavior.

---

## 17. Institution

An **institution** is a persistent collective actor whose identity, rules, assets, roles, and goals extend beyond any one member.

Examples include governments, guilds, religions, corporations, households, armies, schools, courts, criminal organizations, and informal councils.

An institution may possess:

- membership and role structure;
- authority and decision procedures;
- assets and obligations;
- official knowledge and records;
- policies and norms;
- relationships with other institutions;
- internal factions;
- long-term goals;
- legitimacy;
- succession rules;
- capacity to act through agents.

Institutions convert many individual actions into durable social structure. They are essential for simulating civilization because they preserve intentions and constraints across generations.

An institution is not simply a tag attached to NPCs. It is an entity capable of owning, deciding, remembering, and changing.

---

## 18. Culture

**Culture** is the distributed set of learned norms, values, classifications, practices, symbols, and expectations shared unevenly across a population.

Culture should not be represented as a single immutable template. It exists through adoption, transmission, variation, enforcement, and hybridization.

Cultural traits may influence:

- acceptable behavior;
- food and dress;
- law and punishment;
- architecture;
- language;
- ritual;
- family structure;
- status;
- aesthetics;
- concepts of property;
- attitudes toward institutions and outsiders.

Culture is neither entirely an entity nor merely an attribute. It is a population-level pattern expressed through beliefs, behavior, artifacts, and institutions. Implementations may represent it through trait distributions, networks, and regional profiles while preserving individual variation.

---

## 19. History and Provenance

**History** is the temporally ordered structure of committed events and persistent consequences that produced the present world.

**Provenance** is the lineage by which an entity, fact, record, belief, or resource came to exist in its current form.

History should be active. It changes present probabilities through:

- inherited wealth and debt;
- damaged or constructed infrastructure;
- legal precedent;
- remembered grievances;
- institutional legitimacy;
- demographic change;
- cultural diffusion;
- object provenance;
- environmental transformation.

A history system that merely generates readable summaries is insufficient. History must remain attached to current state through consequences and references.

Provenance provides continuity. A legendary weapon matters because of who forged, carried, lost, and recovered it. A border matters because of prior conflicts and treaties. A scientific fact matters because of experiments, records, and trust networks.

---

## 20. Narrative

A **narrative** is an observer-oriented representation of selected reality, belief, or history.

Narrative may be generated for:

- players;
- logs;
- reports;
- in-world books;
- agent conversation;
- debugging;
- historical analysis.

Narrative is always a projection. It selects, compresses, orders, and interprets. Its authority depends on its source.

A system diagnostic may describe canonical state directly. An NPC may tell a false story. A historian may write from incomplete records. An LLM narrator may produce elegant prose from validated perception context. These are all narratives, but they do not occupy the same epistemic status.

Cardinal should tag narrative outputs with their grounding context where practical. The engine must always be able to distinguish “the world contains this fact” from “a character said this sentence.”

---

## 21. Canonical Layering

The ontology can be summarized as a set of layers:

```text
WORLD LAWS
    constrain
REALITY AND STATE
    evolve through
ACTIONS, PROCESSES, EVENTS, AND TRANSACTIONS
    produce
HISTORY AND CAUSALITY
    become partially accessible through
OBSERVATION
    interpreted as
BELIEF, KNOWLEDGE, AND MEMORY
    influence
DECISION AND INSTITUTIONAL ACTION
    expressed to observers through
NARRATIVE
```

The arrows are not a one-way pipeline. Beliefs influence actions, actions alter reality, reality produces new observations, and history reshapes all later decisions. The layers define authority, not isolation.

---

## 22. Chapter Contract

Implementations derived from this ontology must preserve the following distinctions:

1. Reality is not belief.
2. Belief is not memory.
3. Memory is not canonical history.
4. History is not merely a textual chronicle.
5. Intent is not action.
6. Action is not outcome.
7. An event is not an uncommitted delta.
8. An entity’s identity is not identical to its current properties.
9. An institution is not merely a collection of member tags.
10. Narrative is not a state mutation mechanism.
11. Simulation time is not wall-clock time.
12. Coarse simulation is not permission to create incompatible reality.

These distinctions are the grammar of Cardinal. Systems may be optimized, combined, or implemented through different technologies, but their semantic boundaries must remain visible.

---

## Conclusion

A Cardinal world is not a database of objects and flags. It is a temporally ordered reality in which identifiable entities participate in lawful processes, events commit durable consequences, observers form imperfect beliefs, institutions preserve collective structure, and history continues to shape what can happen next.

This ontology gives Cardinal a common language. The next chapter turns that language into non-negotiable laws.
