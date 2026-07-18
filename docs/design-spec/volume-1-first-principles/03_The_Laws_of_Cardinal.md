# Cardinal Definitive Reference
## Volume I: First Principles
### Chapter 3: The Laws of Cardinal

> **A Cardinal law is an invariant that remains binding even when violating it would be easier, faster, or more dramatic.**

---

## 1. Purpose and Normative Language

This chapter defines the constitutional laws of the Cardinal engine. They are not implementation suggestions. They establish the behavior that all subsystems, world packages, narrators, tools, and future extensions must preserve.

The terms **MUST**, **MUST NOT**, **SHOULD**, **SHOULD NOT**, and **MAY** are normative:

- **MUST / MUST NOT** identify requirements necessary for Cardinal compatibility.
- **SHOULD / SHOULD NOT** identify strong defaults that require documented justification when violated.
- **MAY** identifies permitted flexibility.

When two implementation goals conflict, the laws in this chapter take priority over convenience and short-term feature velocity.

---

## Law 1: Reality Is Authoritative

Canonical simulation state MUST be the final authority on what exists and what has occurred.

No narrator, UI, cache, world description, prompt, or agent belief may override canonical reality. External systems may propose actions or interpretations. Only validated simulation transactions may change the world.

### Why This Law Exists

Without a single authority, contradictions become inevitable. An NPC remembers an event that was never committed. A narrator destroys an object still present in inventory. A quest declares a city saved while its population and structures remain unchanged.

### Implementation Consequences

- State mutations MUST pass through authorized transition and persistence mechanisms.
- Outcome prose MUST be generated after commit.
- Debugging tools MUST distinguish canonical fields from derived views.
- Caches that influence outcomes MUST be reconstructible from authoritative state.
- LLM output MUST NOT be parsed as canonical consequence unless it is first converted into a constrained proposal and resolved by simulation rules.

### Violation Test

If disabling the narrator changes simulation outcomes, this law has been violated.

---

## Law 2: Simulation Precedes Narration

The engine MUST compute, validate, and commit consequences before any narrative system describes them.

Narration MAY choose perspective, tone, emphasis, and compression. It MUST remain grounded in committed state and authorized perception.

### Why This Law Exists

Language is capable of generating plausible but unsupported claims. Cardinal must never mistake eloquence for causality.

### Implementation Consequences

- Every user-facing action follows the sequence: parse, validate, simulate, commit, perceive, narrate.
- Narrator context MUST be perception-scoped.
- The deterministic plain narrator MUST remain a supported fallback.
- Narrative failures MUST NOT roll back or corrupt valid simulation commits unless narration is explicitly part of an external delivery transaction rather than world truth.

---

## Law 3: Every Consequence Requires Sufficient Cause

Every significant state transition MUST be attributable to explicit antecedents and rules.

“Significant” includes transitions that alter identity, life, ownership, institutional authority, durable relationships, world resources, major beliefs, or historical trajectory.

### Why This Law Exists

Uncaused outcomes are authored interventions disguised as simulation. They prevent explanation and undermine replay.

### Implementation Consequences

- Events SHOULD carry causal references.
- Systems MUST identify initiating inputs and enabling conditions.
- Random outcomes MUST cite deterministic random substreams and resolution rules.
- Scheduled events MUST retain the cause that scheduled them.
- Administrative or migration changes MUST be labeled as such rather than rewritten as in-world events.

### Important Qualification

Cardinal does not require an exhaustive metaphysical graph. It requires sufficient causal lineage for explanation, debugging, and historical meaning.

---

## Law 4: Consequences Persist

A committed consequence MUST remain part of the world until another lawful transition changes, transforms, repairs, consumes, supersedes, or destroys it.

The engine MUST NOT silently reset state because an encounter ended, a player left a region, or a narrative sequence completed.

### Why This Law Exists

Persistence is the difference between a world and a scene. A burned granary must remain burned. A depleted market must remain depleted. A promise must remain an obligation until fulfilled, forgiven, forgotten, invalidated, or broken.

### Implementation Consequences

- Encounter state MUST reconcile with persistent entity state.
- Off-screen simulation MUST continue or explicitly abstract ongoing processes.
- Temporary effects MUST declare duration and expiration behavior.
- Destruction SHOULD preserve historical provenance even when active state is removed.
- Cleanup processes MUST distinguish removal of computational data from erasure of historical fact.

---

## Law 5: History Is Append-Only

Committed in-world history MUST NOT be retroactively rewritten.

Later events MAY correct records, challenge interpretations, reveal hidden causes, or change beliefs. They MUST NOT alter the fact that earlier events occurred as committed.

### Why This Law Exists

Stable history is required for causality, replay, provenance, trust, and institutional continuity.

### Implementation Consequences

- Event records are immutable after commit.
- Retcons are represented as explicit branch, reset, migration, or alternate-world operations.
- Save migrations preserve semantic history and record technical transformations.
- A mistaken in-world record remains a mistaken record even after truth is discovered.

### Exception Boundary

Corrupted persistence may be repaired, but repairs MUST be auditable technical actions and MUST NOT masquerade as natural history.

---

## Law 6: Information Has Provenance

Every persistent belief, rumor, learned proposition, or institutional record SHOULD identify how it entered the epistemic system.

High-impact information MUST include source, acquisition time, and confidence or reliability context.

### Why This Law Exists

Information without provenance becomes magical omniscience. Provenance enables trust, rumor, investigation, deception, scholarship, and error.

### Implementation Consequences

- Direct observation, testimony, inference, record access, and memory recall MUST be distinguishable.
- Rumor transmission MUST preserve lineage while allowing distortion and confidence decay.
- Agents MUST NOT query unrestricted reality when making in-world decisions.
- Institutional knowledge MUST be accessible through roles, records, or communication pathways.

---

## Law 7: Facts and Beliefs Are Separate

Canonical facts MUST be stored and evaluated independently from what agents believe.

An agent MAY believe a false proposition, reject a true proposition, or remain unaware of a relevant fact.

### Why This Law Exists

Without this separation, misinformation, secrecy, discovery, surprise, and perspective cannot be simulated.

### Implementation Consequences

- Decision systems consume beliefs and perceptions, not unrestricted truth.
- Debug tools MAY compare belief against reality, but in-world actors MUST NOT receive that comparison automatically.
- Dialogue systems MUST ground statements in speaker beliefs, goals, and willingness to disclose.
- Knowledge updates MUST be explicit epistemic transitions.

---

## Law 8: Decisions Require Motivation

Autonomous decisions MUST arise from represented pressures, goals, commitments, values, beliefs, habits, or institutional directives.

Randomness MAY break ties, model uncertainty, or generate variation. It MUST NOT substitute for motivation in consequential decisions.

### Why This Law Exists

An agent that acts randomly can appear busy but not alive. Motivation connects behavior to identity and history.

### Implementation Consequences

- Utility or planning systems SHOULD expose the factors contributing to a selected action.
- Agents MAY hold contradictory goals.
- Goals SHOULD vary by horizon, urgency, and persistence.
- Decisions SHOULD account for expected cost, risk, capability, social consequence, and information confidence.
- Significant decisions SHOULD be explainable after the fact.

---

## Law 9: Constraints Are Real

An actor MUST be limited by capabilities, resources, access, time, information, law, geography, relationships, and world-defined physical rules.

The engine MUST NOT allow an agent to satisfy a goal merely because the goal is important.

### Why This Law Exists

Constraints generate tradeoffs. Tradeoffs generate meaningful behavior. Without constraints, utility systems converge on frictionless optimal actions and social systems lose their texture.

### Implementation Consequences

- Plans MUST validate required resources and pathways.
- Travel requires traversable topology and duration.
- Communication requires a channel and reach.
- Institutional actions require authority and capacity.
- Resource commitments MUST be reconciled.
- Failure, delay, substitution, and abandonment MUST remain possible outcomes.

---

## Law 10: The Player Is Not Exempt

Player-controlled entities MUST obey the same world laws as comparable autonomous entities unless a world package explicitly declares a lawful asymmetry.

### Why This Law Exists

Hidden exemptions transform the world into a responsive stage and break causal integrity.

### Implementation Consequences

- Damage, movement, resource use, legal status, knowledge limits, and death rules apply consistently.
- No level scaling may alter other entities solely because of player progression.
- Player importance emerges from world state and consequences.
- Tutorial or accessibility assistance SHOULD operate through presentation or explicit world rules rather than secret reality changes.

---

## Law 11: Systems Are World-Agnostic

Engine code MUST implement general mechanisms and MUST NOT import or depend directly upon specific world content.

Every tunable world number MUST reside in validated world data or a declared ruleset, not as an unexplained engine constant.

### Why This Law Exists

World-specific assumptions create architectural sediment. They make every future genre more expensive and less coherent.

### Implementation Consequences

- `engine/` MUST NOT import from `worlds/`.
- World packages MUST enter through registry and schema validation.
- IDs, capabilities, resource types, delivery methods, and effect semantics SHOULD be generic.
- Engine defaults, when unavoidable, MUST be explicit, documented, and overrideable where world meaning is involved.

---

## Law 12: Determinism Is Preserved

The same validated inputs, seed policy, engine version, world version, and event ordering MUST produce the same canonical results.

### Why This Law Exists

Determinism enables scientific inspection, replay, testing, and causal confidence.

### Implementation Consequences

- Randomness MUST use seeded, named substreams.
- Iteration over unordered collections MUST be normalized before it can affect outcomes.
- Concurrency MUST not alter semantic ordering.
- Wall-clock time, process IDs, memory addresses, and external network timing MUST NOT influence canonical results.
- External model calls MAY inform noncanonical narration or produce constrained proposals, but their raw variability MUST NOT directly mutate reality.

---

## Law 13: Conservation Is Explicit

Quantities declared conserved by a world MUST move through reconciled transfers, transformations, production, and destruction.

### Why This Law Exists

Economies and ecologies lose meaning when resources silently appear or vanish.

### Implementation Consequences

- Currency movement MUST reconcile across sources and sinks.
- Item transfer MUST update all ownership and inventory records atomically.
- Production MUST consume declared inputs or invoke a declared generative law.
- Destruction MUST identify sinks or transformed outputs where relevant.
- Abstract replenishment systems MUST be modeled as explicit boundary processes rather than concealed corrections.

Conservation is world-defined. A magical world may create matter, but creation must still follow that world’s laws.

---

## Law 14: Scale Must Not Change Truth

Changing simulation fidelity MUST NOT create logically incompatible outcomes.

A distant region may be simulated statistically, but its aggregate results must reconcile when entities return to detailed simulation.

### Why This Law Exists

Multi-scale simulation is necessary for performance. Without truth compatibility, it becomes teleportation between unrelated models.

### Implementation Consequences

- Every abstraction level MUST declare preserved invariants.
- Promotion to finer fidelity MUST materialize state consistent with aggregate history.
- Demotion to coarser fidelity MUST summarize ongoing processes without losing required commitments.
- Cross-region interactions MUST be resolved at a level capable of representing their important consequences.
- Fidelity changes SHOULD be deterministic and auditable.

---

## Law 15: No Important Hidden State

Any state capable of affecting canonical outcomes MUST be persisted or deterministically derivable from persisted data.

### Why This Law Exists

Hidden runtime state breaks save integrity, replay, and debugging.

### Implementation Consequences

- Long-running processes, encounter states, cooldowns, reservations, and scheduled work MUST survive restart when they affect future outcomes.
- System-local caches MUST be nonauthoritative.
- Serialization boundaries MUST be tested through save, reload, and continued simulation.
- Model prompts and transient service responses MUST NOT conceal world state.

---

## Law 16: Transitions Are Atomic

Each simulation boundary MUST commit a complete, invariant-valid set of consequences or commit nothing.

### Why This Law Exists

Partial events create impossible worlds.

### Implementation Consequences

- Systems propose deltas.
- Persistence applies ordered deltas in transactions.
- Cross-system effects must reconcile before commit.
- Failure handling must prevent duplicate or partial application.
- Observers only perceive committed boundaries.

---

## Law 17: Death, Destruction, and Removal Preserve Traces

When an entity ceases active participation, the engine SHOULD preserve the historical fact of its existence and relevant provenance.

### Why This Law Exists

A world that deletes its dead also deletes the causes of its present.

### Implementation Consequences

- Dead agents take no living actions.
- Destroyed entities may become tombstones, remains, debris, records, or inactive historical rows.
- References from memories, history, ownership, and causality remain resolvable.
- Data retention MAY be compacted, but semantic lineage SHOULD survive.

---

## Law 18: Emergence Is Not Randomness

Cardinal MUST distinguish emergent behavior from arbitrary variation.

Emergence occurs when understandable local rules and persistent interactions produce unenumerated higher-order outcomes.

### Why This Law Exists

Random event generators can produce novelty without structure. Cardinal seeks novelty with causality.

### Implementation Consequences

- Random world events SHOULD enter through systems and create normal consequences.
- Designers SHOULD prefer feedback loops over isolated content rolls.
- Telemetry SHOULD measure whether systems materially influence one another.
- A surprising result should be reproducible and explainable from state.

---

## Law 19: Observability Is a Core Capability

The engine MUST provide tools to inspect state, history, causality, system decisions, and invariant health.

### Why This Law Exists

A complex simulation that cannot explain itself cannot be trusted or improved.

### Implementation Consequences

- Systems SHOULD emit structured decision traces at configurable levels.
- The CLI or equivalent tooling MUST support inspection without requiring narrative interpretation.
- Long-run telemetry MUST expose population, resources, markets, institutions, beliefs, and process health.
- Invariant failures MUST include enough context to reproduce the responsible transition.

---

## Law 20: Content Is Data, but Data Is Lawful

World packages MAY define extraordinary realities, but all content MUST pass schema, reference, and invariant validation before simulation.

### Why This Law Exists

Data-driven architecture is not permission for incoherent data.

### Implementation Consequences

- IDs must resolve.
- Capabilities must reference supported mechanisms.
- Effects must declare valid targets and scopes.
- Rule combinations should be validated for contradiction where practical.
- Package versions and migration requirements must be explicit.

---

## 21. Resolving Conflicts Between Laws

The laws reinforce one another, but implementation tradeoffs may create apparent conflicts.

Examples:

- **Persistence versus performance:** use lawful abstraction, not deletion of consequences.
- **Determinism versus external AI:** restrict AI to narration or deterministic, validated proposal interfaces.
- **World agnosticism versus optimized systems:** specialize behind generic contracts and capability declarations.
- **History versus storage cost:** compact representation while preserving semantic lineage and required queries.
- **Atomicity versus multi-system complexity:** stage deltas and validate cross-system invariants before commit.

When a compromise is necessary, it MUST be documented through an architecture decision record describing:

1. the laws involved;
2. the chosen interpretation;
3. preserved invariants;
4. known limitations;
5. a path to stronger compliance if one exists.

---

## 22. Cardinal Compatibility Checklist

A new subsystem is constitutionally compatible only when the answer to each applicable question is yes:

- Does it operate on canonical state rather than narrative claims?
- Are its changes committed through atomic deltas?
- Can its outcomes be reproduced from the same inputs?
- Are its important causes inspectable?
- Do its consequences persist?
- Does it separate facts from actor beliefs?
- Do autonomous decisions have represented motivations?
- Are constraints and resources enforced?
- Does it avoid player-only exemptions?
- Is world-specific meaning supplied through data?
- Does it preserve truth across fidelity changes?
- Can its state survive save and reload?
- Can tools explain what it did and why?

A subsystem that cannot meet these requirements may still be useful as an experiment, but it must not silently redefine Cardinal’s architecture.

---

## Conclusion

The Laws of Cardinal ensure that the engine’s ambition survives contact with implementation. They prevent a gradual slide from simulated reality into convenient illusion.

They may make some features harder to build. That difficulty is intentional. Cardinal is defined not by what it can depict, but by what it can truthfully compute.
