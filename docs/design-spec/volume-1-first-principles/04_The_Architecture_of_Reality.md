# Cardinal Definitive Reference
## Volume I: First Principles
### Chapter 4: The Architecture of Reality

> **Cardinal is a causal kernel surrounded by world-defined laws, persistent state, bounded minds, collective actors, and observational interfaces.**

---

## 1. Purpose of This Chapter

The previous chapters defined why Cardinal exists, what exists within a Cardinal world, and which laws every implementation must obey. This chapter describes the architecture required to turn those principles into an executable reality.

This is not a class diagram and does not mandate one programming language, database, or concurrency model. It defines responsibilities, authority boundaries, data flow, and extension contracts.

The current Cardinal codebase already provides several correct foundations:

- validated world packages;
- strict separation between engine and world content;
- deterministic random substreams;
- a hierarchical world clock;
- a delta-based simulation loop;
- transactional SQLite persistence;
- a narrator that can be disabled or degraded safely;
- invariant and long-run tests.

The architecture in this chapter extends those foundations rather than discarding them.

---

## 2. Architectural Overview

Cardinal should be understood as seven cooperating layers:

```text
1. WORLD DEFINITION
   Laws, schemas, content, parameters, initial conditions

2. CANONICAL REALITY
   Entities, relationships, resources, processes, institutions, environment

3. SIMULATION KERNEL
   Clock, scheduler, systems, validation, transactions, deterministic RNG

4. HISTORICAL-CAUSAL LAYER
   Events, provenance, causal links, chronicle, migrations, replay metadata

5. EPISTEMIC-COGNITIVE LAYER
   Perception, beliefs, memory, goals, planning, relationships, communication

6. AGGREGATE SOCIAL LAYER
   Institutions, culture, economy, ecology, demographics, politics

7. OBSERVATION AND CONTROL
   CLI, APIs, players, agents, telemetry, debuggers, narrators, tools
```

Authority flows inward toward canonical reality and the simulation kernel. Interpretation flows outward toward observers.

No outer layer may bypass the kernel to mutate reality.

---

## 3. The World Definition Layer

The world definition layer describes what kind of reality is being instantiated.

It consists of:

- package manifest;
- rules and constants;
- schemas;
- content definitions;
- effect and capability declarations;
- geography and topology;
- initial populations and institutions;
- simulation module configuration;
- version and migration metadata.

### 3.1 Registry as Boundary

All world content enters the engine through the registry. The registry is responsible for:

- loading package files;
- schema validation;
- ID registration;
- cross-reference validation;
- capability compatibility checks;
- package version checks;
- normalized access to content definitions.

Engine systems must not reach into package directories directly. They consume validated definitions through registry contracts.

### 3.2 Rules as Declared Reality

Every tunable value that expresses how a world behaves belongs in world data. Engine code may contain algorithms, but not hidden genre assumptions.

A rule should include units and semantic meaning. For example, a migration threshold should not be a bare number named `0.7`; it should be a documented ratio tied to a population-pressure model.

### 3.3 Capability-Based Design

World content should declare capabilities rather than force the engine to infer them from type names.

An entity may be:

- movable;
- living;
- damageable;
- ownable;
- communicative;
- institutional;
- perceiving;
- tradable;
- traversable;
- executable as a device.

Capabilities determine applicable systems. This allows a ship, corporation, animal, city, and autonomous machine to share mechanisms without occupying one inheritance hierarchy.

---

## 4. Canonical Reality Store

The canonical reality store is the persistent source of truth for the running world.

It contains normalized state sufficient to:

- resume after interruption;
- reproduce future outcomes;
- inspect current conditions;
- validate invariants;
- resolve references to historical entities;
- support multiple fidelity levels.

### 4.1 Persistence Categories

Cardinal should distinguish at least five persistence categories:

1. **Active state**: current entity and system properties.
2. **Processes**: ongoing work with future consequences.
3. **Scheduled work**: deterministic future evaluations.
4. **Historical records**: immutable events and provenance.
5. **Epistemic state**: beliefs, memories, observations, records, and secrets.

These categories may share storage technology, but their lifecycle rules differ.

### 4.2 Stable Identity

Every persistent entity and significant process receives a stable identifier. Human-readable definition IDs identify templates. Runtime IDs identify historical instances.

The engine must never confuse “an anneal blade definition” with “the anneal blade carried by a particular character.”

### 4.3 Append-Only Historical Tables

Historical records should favor append-only design. Active state may be updated for efficient reads, while events preserve transition lineage.

The architecture need not become pure event sourcing. Cardinal should use event sourcing where it strengthens history and replay, while retaining materialized current state for practical simulation performance.

---

## 5. Simulation Kernel

The simulation kernel is the smallest trusted core capable of advancing reality lawfully.

Its responsibilities are:

- ordering time;
- selecting due work;
- providing deterministic random substreams;
- constructing read contexts;
- invoking systems;
- collecting proposed deltas and events;
- validating transitions and invariants;
- committing atomically;
- exposing committed results to observers.

The kernel should know little about specific domains. Combat, economy, cognition, and ecology are systems operating through kernel contracts.

---

## 6. The Canonical Simulation Cycle

Every simulation boundary follows a disciplined cycle.

### Phase 1: Determine Boundary

The clock and scheduler determine the next meaningful simulation time. Boundaries may be initiated by:

- player or external action;
- due process step;
- system tick;
- deadline;
- environmental transition;
- queued institutional decision;
- fidelity transition.

### Phase 2: Construct Snapshot Context

Systems receive a stable read view for the boundary. The context includes:

- current time;
- relevant canonical state;
- due work;
- validated world definitions;
- deterministic RNG handles;
- system configuration;
- causal parent context.

### Phase 3: Evaluate Systems

Systems compute proposals. A proposal may contain:

- state deltas;
- new events;
- process changes;
- future schedules;
- observations;
- invariant assertions;
- causal metadata.

Systems must not commit directly.

### Phase 4: Reconcile

The kernel resolves ordering and conflicts among proposals. Reconciliation includes:

- ownership conflicts;
- resource conservation;
- entity lifecycle conflicts;
- incompatible movements;
- duplicate schedules;
- cross-system dependencies.

The current engine’s incremental delta behavior, where later tick boundaries see earlier committed results, should be preserved. Within one atomic boundary, ordering must be explicit.

### Phase 5: Validate

Schemas and invariants confirm that the proposed next state is lawful.

Validation includes:

- referential integrity;
- conservation checks;
- capability constraints;
- state-machine legality;
- temporal consistency;
- authority checks;
- world-specific invariant hooks.

### Phase 6: Commit

The persistence layer applies all reconciled deltas and event records in one transaction.

### Phase 7: Derive Perception

Once state is committed, perception systems determine what each relevant observer could detect.

### Phase 8: Notify and Narrate

Structured outputs are provided to:

- narrator;
- CLI or UI;
- telemetry;
- debugging tools;
- automation clients;
- agent memory systems.

Only committed outcomes are exposed as reality.

---

## 7. Scheduler and Multi-Scale Time

A near-life world cannot evaluate every entity at maximum frequency. Cardinal therefore requires a deterministic multi-scale scheduler.

### 7.1 Work Units

Scheduled work should identify:

- target time;
- responsible system;
- entity or region scope;
- process reference;
- priority class;
- causal origin;
- fidelity requirement;
- deterministic tie-break key.

### 7.2 Temporal Hierarchy

Different systems operate at natural intervals:

- combat at seconds;
- movement at minutes;
- needs and schedules at hours;
- markets and ecology at days;
- institutions at weeks or months;
- culture and demographics at seasons or years.

The scheduler should advance directly to the next due boundary when no finer action is required.

### 7.3 Fidelity Tiers

A recommended tier model is:

- **Tier 0: Immediate**: full detail for active encounters and closely observed entities.
- **Tier 1: Local**: individual simulation for loaded settlements and nearby agents.
- **Tier 2: Regional**: cohorts, aggregate stocks, and scheduled institution actions.
- **Tier 3: Global**: statistical trends, macro flows, and infrequent historical transitions.

World packages may rename or configure tiers. Every subsystem must declare how it promotes and demotes state.

### 7.4 Fidelity Contracts

Each fidelity transition must preserve:

- conserved resources;
- entity identity where still relevant;
- active obligations;
- causal commitments;
- demographic totals;
- process progress;
- historical events;
- boundary interactions.

A coarse region cannot invent prosperity that conflicts with its resources and trade. A promoted population cannot materialize individuals whose ages, roles, assets, and relationships contradict aggregate history.

---

## 8. System Contract

Every Cardinal system should implement a common conceptual contract.

### Inputs

- immutable simulation context;
- validated definitions;
- scoped state query interface;
- deterministic RNG substream;
- due work or triggering events.

### Outputs

- proposed deltas;
- semantic events;
- causal references;
- schedules;
- observations;
- diagnostics;
- invariant claims.

### Required Properties

A system must be:

- deterministic;
- side-effect controlled;
- persistence-aware;
- world-agnostic;
- observable;
- testable in isolation;
- compatible with transactional commit.

Systems should communicate through committed state, events, and explicit proposals rather than hidden direct calls. Direct synchronous composition is acceptable when it represents one domain operation, but the dependency must remain visible.

---

## 9. Historical-Causal Layer

The historical-causal layer makes Cardinal more than a state machine.

### 9.1 Event Records

A semantic event record should support:

- event ID and type;
- simulation timestamp;
- participating entities;
- location or scope;
- causal parents;
- triggering action or process;
- direct state deltas;
- visibility metadata;
- salience;
- world-specific tags.

### 9.2 Causal Graph

Causal links need not connect every low-level delta. They should connect events and decisions that explain meaningful outcomes.

The graph supports queries such as:

- Why is food expensive here?
- Why does this institution distrust that family?
- Which event caused this road to be abandoned?
- What conditions led to this war?
- Which beliefs motivated this action?

### 9.3 Chronicle

The chronicle is a selected, human-readable index of historically salient events. It is not the complete event store and not the only history.

Chronicle selection should be data-driven and may vary by world, region, institution, or observer.

### 9.4 Provenance

Entities and records should link to creation, transfer, transformation, and destruction events where relevant. Provenance enables material culture and inherited meaning.

---

## 10. Epistemic Architecture

The epistemic layer mediates between reality and minds.

### 10.1 Perception Pipeline

```text
Canonical event or state
        ↓
Perceptual eligibility
        ↓
Signal quality and concealment
        ↓
Observation record
        ↓
Interpretation
        ↓
Belief update
        ↓
Memory formation
```

Perception must be world-defined. A magical sensor, security camera, animal scent, written archive, and eyewitness account are different channels.

### 10.2 Belief Store

Beliefs should be queryable by subject, proposition, confidence, source, and time. Contradictions must be representable.

### 10.3 Memory Store

Memories store actor-centered traces of experience. Memory consolidation and decay may operate at lower frequency than perception.

### 10.4 Communication

Communication is the transfer of claims, not truth. A message includes:

- sender;
- receiver or audience;
- proposition;
- claimed source;
- channel;
- time;
- disclosure intent;
- distortion risk;
- secrecy.

The recipient evaluates the claim using trust, prior belief, evidence, and cultural expectations.

---

## 11. Cognitive Architecture

Agents require a layered cognition model rather than one monolithic “AI tick.”

### 11.1 Needs and Pressures

Needs create motivational pressure. They may be physiological, social, emotional, economic, ideological, or institutional.

### 11.2 Goals

Goals represent desired future states. They have:

- horizon;
- urgency;
- value;
- owner;
- origin;
- persistence;
- success conditions;
- abandonment conditions.

### 11.3 Candidate Generation

Agents generate candidate actions and plans from capabilities, known opportunities, social norms, duties, and remembered strategies.

### 11.4 Evaluation

Candidates are evaluated against beliefs rather than omniscient state. Factors include:

- expected utility;
- confidence;
- risk;
- cost;
- time;
- relationships;
- legality;
- identity and values;
- institutional obligation.

### 11.5 Commitment

Selected plans reserve attention, resources, time, and obligations. Interruptions should have costs.

### 11.6 Explanation

The system should retain a compact decision trace for significant choices:

- active pressures;
- considered candidates;
- decisive factors;
- selected action;
- confidence;
- triggering belief.

This is required for debugging and believable explanation.

---

## 12. Relationships and Social Networks

Relationships should be stored as multidimensional, directional state. Social systems query the dimensions relevant to each decision.

Relationship updates arise from:

- direct interaction;
- observed behavior;
- rumors and testimony;
- institutional membership;
- cultural stereotypes;
- debt and obligation;
- kinship;
- repeated cooperation or betrayal.

Network-level structures such as influence, isolation, faction formation, and reputation emerge from these local links.

Public reputation should not replace personal belief. It is an aggregate informational phenomenon with its own sources and audiences.

---

## 13. Institutions as Runtime Actors

Institutions require first-class runtime state.

An institution system should model:

- charter or purpose;
- membership;
- roles and authority;
- treasury and assets;
- official policies;
- decision mechanisms;
- records;
- relationships;
- internal factions;
- legitimacy;
- succession;
- active plans.

Institutions act through authorized agents or institutional processes. A government does not swing a sword, but it can issue orders, transfer funds, enact laws, appoint officials, and mobilize forces.

Collective intent should not be reduced to the arithmetic average of member goals. Institutional procedures select which preferences become action.

---

## 14. Economy, Ecology, and Material Flows

Aggregate systems should operate through shared concepts of stocks, flows, transformations, constraints, and feedback.

### Economy

- goods and services;
- supply and demand;
- production inputs;
- labor;
- ownership;
- prices;
- credit and obligation;
- transport;
- institutional policy.

### Ecology

- populations;
- carrying capacities;
- predation;
- reproduction;
- migration;
- disease;
- climate and habitat;
- resource competition.

These systems should interact. Weather affects crops. Ecology affects hunting. Transport affects prices. Institutions regulate extraction. Culture affects consumption.

The architecture should favor interoperable state over sealed minigames.

---

## 15. Culture Layer

Culture operates as distributed traits and norms rather than one controlling object.

The culture system should support:

- trait adoption probabilities;
- geographic and social diffusion;
- institutional reinforcement;
- generational transmission;
- mutation and hybridization;
- norm enforcement;
- material expression through artifacts and architecture.

Culture influences decision evaluation but does not dictate identical behavior. Individuals may conform, strategically perform, reject, reinterpret, or combine cultural expectations.

---

## 16. Observation and Control Interfaces

External interfaces interact with the world through explicit boundaries.

### 16.1 Commands

Player, API, and automation input is converted into proposed actions. Commands do not mutate tables directly.

### 16.2 Inspection

Inspection tools may access canonical truth for developers. In-world interfaces must remain perception-bound.

### 16.3 Narrators

Narrators receive:

- committed event results;
- authorized observations;
- relevant memories and beliefs;
- style and world context;
- strict prohibition on new state claims.

### 16.4 Telemetry

Telemetry should expose system health and emergent behavior without becoming part of reality unless explicitly represented as an in-world instrument.

### 16.5 Modding and Tools

Editors and validators should operate on world packages and migrations, not bypass runtime invariants.

---

## 17. Extension Architecture

Cardinal should support growth through explicit extension points:

- schemas;
- capabilities;
- system registration;
- event types;
- effect resolvers;
- perception channels;
- fidelity adapters;
- invariant hooks;
- narrator adapters;
- telemetry exporters;
- migration steps.

Extensions must declare dependencies and deterministic ordering. Plug-in flexibility must not produce invisible system order or uncontrolled side effects.

---

## 18. Testing Architecture

Every subsystem requires multiple forms of proof.

### Unit Tests

Validate isolated resolution and edge cases.

### Invariant Tests

Prove conservation, lifecycle, authority, and no-scaling rules.

### Golden Determinism Tests

Run identical inputs and compare byte-stable canonical outputs or normalized event histories.

### Persistence Continuation Tests

Simulate, save, reload in a new process, continue, and compare against uninterrupted execution.

### Long-Run Tests

Advance months or years without players and inspect stability, collapse, diversity, and nontrivial history.

### Genre Probes

Run foreign world packages to detect genre leakage.

### Counterfactual Tests

Alter one initial condition or event and confirm downstream history changes lawfully while unrelated substreams remain stable where expected.

---

## 19. Recommended Near-Term Architectural Sequence

The next expansion beyond the current engine should proceed in an order that maximizes system amplification:

1. **Causal event metadata and decision traces.**
2. **Epistemic belief model with provenance.**
3. **General goal and intent engine.**
4. **Multidimensional relationships.**
5. **Institutions as first-class actors.**
6. **Historical provenance for objects, places, and organizations.**
7. **Multi-scale scheduler and fidelity transitions.**
8. **Culture and demographic transmission.**
9. **Advanced production, logistics, labor, and finance.**
10. **Narrative and analysis tools built on the completed causal substrate.**

This sequence prioritizes feedback loops over isolated mechanics.

---

## 20. Architecture Contract

A conforming Cardinal architecture must ensure:

- one canonical reality store;
- one lawful transition membrane;
- deterministic scheduling;
- transactional commit;
- append-only semantic history;
- bounded perception;
- explicit belief and memory;
- motivated action;
- first-class institutions;
- compatible multi-scale simulation;
- world-defined meaning;
- observational narration;
- inspectable causes and decisions.

Specific modules may evolve. These boundaries should not.

---

## Conclusion

Cardinal’s architecture is designed to turn continuity into an engine capability. The simulation kernel preserves law. Persistence preserves consequence. The historical layer preserves causality. The epistemic layer limits minds. Cognitive systems create motivated action. Institutions and culture create structures larger than individuals. Narrators expose the resulting world without controlling it.

Together, these layers form not a content pipeline, but a machine for producing history.
