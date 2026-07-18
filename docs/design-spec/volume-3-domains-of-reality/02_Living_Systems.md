# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 2
# Living Systems

> *Life is not a property an entity has. It is a process an entity sustains.*

---

# Chapter Overview

Physical Reality established the stage: space, materials, environment, and constraint.

Living Systems introduces the first inhabitants.

This domain is responsible for organisms — entities that maintain themselves against decay, consume resources, grow, reproduce, adapt, and die.

Everything that distinguishes a forest from a rock formation originates here.

Yet the domain is deliberately narrower than "everything alive does."

Living Systems governs the *biology* of individual organisms.

It does not govern their thoughts, their societies, or the population-level patterns that emerge when many organisms share a world. Those belong to Knowledge, Society, and Ecology respectively.

The boundary is simple to state:

Living Systems owns what happens **inside** an organism.

Other domains own what happens **between** them.

---

# 2.1 Purpose

The purpose of Living Systems is to define what it means for an entity to be alive.

Within Cardinal, life is not a boolean.

Life is the continuous satisfaction of maintenance requirements.

An organism persists only while it acquires energy, avoids lethal conditions, and repairs accumulated damage. When maintenance fails, the organism dies — not because a rule says so, but because the state that constituted "being alive" can no longer be sustained.

This framing produces a crucial architectural property:

Death is never a special event handcrafted by gameplay.

Death is the terminal state of a failed process.

Starvation, exposure, disease, injury, and age are all the same phenomenon viewed through different facts.

---

# 2.2 Responsibilities

Living Systems owns:

- Organism identity and lifecycle stage
- Vital state (health, energy, hydration, respiration)
- Needs and their decay rates
- Metabolism — the conversion of consumed matter into maintained state
- Growth and development
- Reproduction and inheritance
- Senescence and natural death
- Disease, injury, and recovery
- Sensory capability (what an organism *can* perceive, not what it knows)
- Physical capability (strength, speed, stamina as biological facts)

Every one of these is a fact attached to an entity, owned by this domain, and consumed by others.

---

# 2.3 Non-Responsibilities

Living Systems deliberately does not own:

- **Decisions.** Choosing to eat is a decision system consuming hunger. Hunger itself is biology.
- **Knowledge and memory.** An organism's beliefs belong to the information architecture (Volume II, Chapter 4).
- **Populations.** Herd dynamics, predation pressure, and carrying capacity belong to Ecology.
- **Value.** A cow is biomass here. It is wealth in the Economy domain.
- **Social bonds.** A pack, family, or tribe is a relationship structure owned by Society.
- **Environmental conditions.** The organism experiences cold; Physical Reality owns the temperature.

An organism in Cardinal is therefore a *meeting point* of domains, not the property of any single one.

---

# 2.4 Canonical Concepts

## Organism

An entity possessing metabolic state.

Nothing more is required.

A bacterium, an oak, a wolf, and a dragon are all organisms — differing only in the facts attached to them and the rates at which those facts change.

## Needs

A need is a fact that decays over time and must be restored through interaction with the world.

```text
Hunger = 0.62      (rises until eating)
Hydration = 0.81   (falls until drinking)
Fatigue = 0.34     (rises until resting)
Warmth = 0.90      (tracks environment)
```

Needs are measurements, never behaviors.

A system elsewhere may translate a critical need into a goal. Living Systems only guarantees that the need exists, decays deterministically, and responds to satisfaction.

## Metabolism

Metabolism is the transformation contract at the center of the domain:

```text
Consumed matter + Current state
        │
        ▼
Energy gained, waste produced, needs restored
```

Every world package defines its own metabolic rules — what counts as food, how efficiently it converts, what happens to the remainder.

The engine only requires that the transformation is deterministic and conservative: matter consumed is matter removed from the world.

## Lifecycle

Organisms move through stages:

```text
Conception → Growth → Maturity → Senescence → Death
```

Stages are thresholds over accumulated state, not scripted transitions.

A world package may define radically different lifecycles — metamorphosis, sporing, division, undeath — while preserving the same architectural shape: stage is derived from facts, and stage changes are events.

## Inheritance

Reproduction produces new organisms whose defining characteristics derive deterministically from their parents plus a seeded variation stream.

This single mechanism, applied over enough generations, is what permits adaptation to emerge without an "evolution system" ever being written.

---

# 2.5 Domain Interactions

## Consumes from Physical Reality

- environmental state (temperature, water, illumination, air)
- material properties of potential food
- containment and shelter
- terrain traversability

## Provides to other domains

- **Ecology:** individual births, deaths, and consumption events that aggregate into population dynamics
- **Economy:** biomass, labor capacity, livestock, and harvestable growth
- **Society:** persons — organisms that other domains layer roles onto
- **Conflict:** health, stamina, and injury as the substrate of combat
- **Physical Reality:** ecological modification (grazing, burrowing, forestation)

The recurring pattern from Chapter 1 repeats:

Living Systems owns the organism.

It does not own what the organism means to anyone else.

---

# Designer Note
## The Deer Does Not Know It Is Prey

A deer in Cardinal has hydration, fatigue, hearing range, and sprint speed.

It does not have a "prey" flag.

Prey is an interpretation made by the wolf's decision system, consuming the wolf's hunger (Living Systems), the wolf's knowledge of the deer's location (Information), and the terrain between them (Physical Reality).

The moment "prey" becomes a stored fact, the simulation stops discovering relationships and starts reciting them.

---

# 2.6 Common Queries

Living Systems must answer questions such as:

- Is this organism alive?
- What does this organism require to remain alive?
- How long can it survive current conditions?
- Can this organism perceive events at this distance?
- What can this organism physically do right now?
- What happens if it consumes this material?
- Is this organism capable of reproduction?
- What did this organism inherit?

Note that every query concerns capability and state.

None concerns intention.

---

# 2.7 Architectural Contracts

1. Vital state is continuous, not categorical. "Wounded" and "starving" are interpretations of numbers, never stored truths.
2. All need decay and restoration is deterministic and rate-based, with rates defined by the world package.
3. Consumption is conservative: whatever an organism eats, drinks, or breathes is debited from Physical Reality.
4. Sensory capability defines the *upper bound* of observation. The information pipeline decides what is actually observed.
5. Death is a state transition recorded as an event. The entity's identity and history persist; its metabolic processing stops.
6. Reproduction is a deterministic function of parent state and a seeded stream.
7. No organism receives special engine treatment. A player character is an organism with the same contracts as a sparrow.

---

# 2.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Life is maintained state, never a flag.
2. Needs decay deterministically and are restored only through world interaction.
3. Metabolism conserves matter between organism and world.
4. Lifecycle stages derive from state thresholds.
5. Inheritance is deterministic given parents and seed.
6. Sensory limits bound observation but never grant knowledge.
7. Death terminates processing, never identity.
8. Biological facts are owned here and consumed elsewhere.
9. No organism is architecturally privileged.
10. Population-level phenomena are never computed inside this domain.

---

# 2.9 Anti-Patterns

### The Health Bar World

Reducing vitality to a single number that only combat can lower. Organisms should be able to die of thirst, cold, age, and infection through the same architecture that lets them die of wounds.

### Scripted Death

Killing entities by fiat ("the quest requires it") rather than by driving their vital state. If narrative needs a death, narrative should introduce causes.

### The Omniscient Stomach

Letting hunger directly trigger pathfinding to the nearest food. Hunger is biology; finding food requires knowledge; walking there requires Physical Reality. Collapsing the pipeline produces organisms that navigate to food they have never seen.

### Species as Class Hierarchies

Encoding "Wolf extends Predator extends Animal." Species are bundles of facts and rates in the world package. Behavior differences must emerge from different values, not different code.

---

# 2.10 Future Evolution

Future versions of Cardinal may introduce:

- genetics with expressed and recessive traits
- epidemiology and immune memory
- detailed anatomy and locational injury
- microbial simulation and decomposition
- plant physiology (root systems, canopy competition)
- symbiosis and parasitism contracts

Each extension deepens the fidelity of organisms without changing the domain's responsibility:

Living Systems answers what it costs to stay alive.

---

## Preparing for the Next Domain

Organisms consume the world to survive.

The next chapter examines what they consume: **Resources** — the domain that gives raw matter identity, quantity, quality, and renewal, forming the bridge between physical substance and economic value.

---

# END OF CHAPTER 2
