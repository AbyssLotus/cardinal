# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 1
# Physical Reality

> *Reality is not defined by physics. It is defined by the existence of consistent relationships.*

---

# Chapter Overview

Volumes I and II established the philosophical and architectural foundations of Cardinal.

They answered questions such as:

- Why does Cardinal exist?
- What distinguishes a simulation engine from a game engine?
- How should information be represented?
- Why are facts, systems, persistence, and domains the fundamental building blocks of reality?

Those questions intentionally stopped just before describing reality itself.

Volume III begins where the previous volumes ended.

Instead of discussing the architecture **of the engine**, this volume discusses the architecture **of existence**.

Every simulation, regardless of its purpose or setting, ultimately consists of interacting domains.

Physical Reality is the first of those domains.

It is the substrate upon which every other domain depends.

Life cannot exist without a place to live.

Economies cannot function without movement of goods.

Societies cannot form without geography.

Conflict cannot occur without distance.

Knowledge cannot spread without communication paths.

Every higher-order phenomenon is constrained by the characteristics of the physical world beneath it.

For this reason, Physical Reality occupies a unique position within Cardinal's architecture.

It is simultaneously one of the simplest domains conceptually and one of the most influential architecturally.

---

# 1.1 Purpose

The purpose of the Physical Reality domain is not to simulate physics.

This distinction is important.

Physics engines simulate rigid bodies, collision detection, gravity, and motion.

Physical Reality is considerably broader.

It defines the **shared language** through which every domain reasons about existence.

When another domain asks questions such as:

- Where is this object?
- Can these entities interact?
- What surrounds this organism?
- How far must this resource travel?
- Is this settlement isolated?

those questions are answered here.

Physical Reality therefore owns the representation of space itself.

It provides the context in which all other simulation occurs.

---

# Designer Note
## Physics Is A Consumer

Many simulation engines place the physics engine at the center of their architecture.

Cardinal deliberately does not.

Physics is simply one consumer of Physical Reality.

Navigation consumes Physical Reality.

Weather consumes Physical Reality.

Ecology consumes Physical Reality.

Combat consumes Physical Reality.

Rendering consumes Physical Reality.

No single subsystem owns reality.

Reality exists independently.



---

# 1.2 Architectural Goals

The Physical Reality domain is designed around a small number of enduring architectural goals. These goals intentionally describe *what* the domain should accomplish rather than *how* a particular implementation should accomplish it.

Implementations will evolve over time. Hardware will change. New algorithms will be discovered. World Packages will introduce novel requirements.

The architectural intent, however, should remain stable.

## Generality

Physical Reality should support worlds of radically different scales and characteristics.

Examples include:

- A single laboratory simulation
- A procedurally generated dungeon
- A living medieval kingdom
- An ocean ecosystem
- A Dyson swarm
- An abstract graph-based economy
- A magical universe where gravity is optional

None of these worlds should require changing the architectural contracts of the domain.

Instead, each World Package specializes the implementation while preserving the same conceptual model.

## Determinism

Given identical state and identical inputs, Physical Reality should produce identical results.

Determinism enables replay, debugging, rollback networking, scientific experimentation, reproducible testing, and historical analysis.

Determinism is therefore treated as an architectural objective rather than an optimization.

## Scalability

The architecture should scale independently of implementation.

Whether simulating one room or one billion square kilometers, the conceptual model should remain unchanged.

Scaling is an implementation concern.

Architectural consistency is not.

---

# 1.3 Domain Ownership

One of the recurring ideas throughout Cardinal is that every fact has exactly one authoritative owner.

Physical Reality owns physical facts.

This includes:

- Position
- Orientation
- Adjacency
- Containment
- Elevation
- Material composition
- Environmental conditions
- Physical regions
- Spatial connectivity

Every other domain consumes these facts.

For example:

Living Systems consumes temperature but does not define temperature.

Economy consumes distance but does not define distance.

Society consumes settlements but does not define terrain.

Conflict consumes line-of-sight but does not define visibility.

This separation dramatically reduces duplication throughout the engine.

## What This Domain Does Not Own

Physical Reality intentionally does not own:

- biological processes
- intelligence
- decision making
- ownership
- economics
- governments
- culture
- rendering
- networking

If another domain wishes to reason about a physical fact, it references the authoritative source rather than maintaining its own copy.

---

# Designer Note
## Reality Before Gameplay

Many engines quietly allow gameplay requirements to reshape the underlying simulation.

A weapon might ignore walls because "it feels better."

A resource might teleport because pathfinding is expensive.

A creature may know about another creature despite having no possible means of perception.

Cardinal intentionally separates simulation from gameplay.

Gameplay systems are free to simplify, hide, exaggerate, or reinterpret reality.

Reality itself should remain internally consistent.

Consistency is what allows emergence.

Without consistency, surprising behavior is impossible because every exception must be handcrafted.



---

# 1.4 The Nature of Space

Space is one of the few concepts that nearly every other domain depends upon.

Cardinal intentionally treats space as a first-class architectural concern rather than an implementation detail.

Space answers a deceptively simple question:

> "Where can something exist?"

Everything else follows from that answer.

Movement becomes possible because space exists.

Distance becomes measurable because locations exist.

Boundaries become meaningful because regions exist.

Interaction becomes possible because entities occupy neighboring locations.

Without space, there is no notion of "next to," "inside," "above," or "far away."

These relationships form the vocabulary of reality.

## Space Is Not Coordinates

Perhaps the most common mistake made when designing simulations is confusing space with coordinates.

Coordinates are merely one representation of space.

A two-dimensional grid is a representation.

Latitude and longitude are a representation.

Hexagonal tiles are a representation.

Voxel coordinates are a representation.

Continuous floating-point coordinates are a representation.

A graph of connected rooms is also a representation.

Even a social network may be interpreted as a form of abstract space.

Cardinal deliberately separates the concept of space from the mathematical system used to describe it.

This separation allows different World Packages to select the representation most appropriate for their simulation without changing the architectural contracts of the engine.

## Physical Space versus Logical Space

Many simulations contain multiple overlapping notions of space.

Physical space describes where something exists.

Logical space describes how something is connected.

These are not always identical.

A mountain village may be geographically close while remaining several days away because only one dangerous pass connects them.

Two spacecraft may be visually adjacent while existing in different orbital transfer windows.

Two rooms separated by a locked vault door are physically adjacent yet effectively disconnected.

Cardinal therefore encourages systems to distinguish geometric proximity from navigational connectivity.

The distinction produces more realistic emergent behavior while avoiding assumptions that every nearby object is easily reachable.

---

# 1.5 Topology

Geometry answers:

"Where?"

Topology answers:

"Connected how?"

This distinction becomes increasingly important as simulations grow.

Road networks, cave systems, river basins, sewer systems, air routes, tunnels, and wormholes are fundamentally topological structures.

They cannot be fully understood through coordinates alone.

A world may therefore expose multiple overlapping topologies simultaneously.

Examples include:

- Road topology
- River topology
- Political topology
- Electrical topology
- Communication topology
- Underground tunnel topology
- Shipping lane topology

Each represents a different interpretation of connectivity.

No single topology is considered authoritative for every system.

Instead, each consuming domain selects whichever topology best represents its own reasoning.

For example:

An economy may optimize around shipping lanes.

Predators may optimize around terrain accessibility.

Birds may largely ignore road topology altogether.

This flexibility allows radically different behaviors to emerge from the same underlying world.



---

# 1.6 Spatial Relationships

Coordinates by themselves have very little meaning.

Meaning emerges from relationships between locations.

The Physical Reality domain exists primarily to maintain these relationships in a consistent and queryable form.

Almost every simulation question can ultimately be reduced to a spatial relationship.

Examples include:

- What is nearby?
- What contains this object?
- What can be seen?
- What can be reached?
- What blocks movement?
- What shares a boundary?
- What lies upstream?
- What is sheltered from the wind?

Notice that none of these questions ask for raw coordinates.

They ask about relationships.

Coordinates are merely one possible implementation detail used to answer them.

## Cardinal Spatial Primitives

Every implementation should be capable of expressing a small vocabulary of relationships.

These include, but are not limited to:

- Adjacent
- Connected
- Contains
- Intersects
- Overlaps
- Above
- Below
- Enclosed
- Exposed
- Reachable
- Visible
- Occupied

World Packages may introduce additional relationships without modifying the engine itself.

For example:

An orbital simulation may define "shares orbit."

A naval simulation may define "same watershed."

A magical world may define "arcanely linked."

The architecture should accommodate these additions naturally.

## Relative Position

Most intelligent actors reason in relative terms rather than absolute coordinates.

A wolf does not know it is standing at (1832.4, 927.1).

It knows that prey is:

- uphill
- across the river
- behind the rocks
- downwind
- inside the forest

Cardinal therefore encourages higher domains to reason using relative spatial concepts whenever practical.

Doing so naturally produces behavior that is easier to understand, more portable across representations, and more closely aligned with how real organisms perceive their environment.

---

# 1.7 Regions

Reality is easier to understand when individual locations can be grouped into larger structures.

These structures are referred to collectively as regions.

A region represents an area that shares one or more meaningful characteristics.

Examples include:

- Forests
- Lakes
- Cities
- Counties
- Biomes
- Watersheds
- Climate zones
- Kingdoms
- Cave systems

Importantly, a region is not required to be geometric.

A region may be discontinuous.

An island nation may consist of hundreds of separate islands.

A trade district may span multiple cities.

A fungal network may connect forests separated by many kilometers.

The defining characteristic of a region is semantic coherence rather than geometric simplicity.

## Overlapping Regions

Locations frequently belong to many regions simultaneously.

A single farmhouse might exist within:

- Earth
- North America
- United States
- Pennsylvania
- Lancaster County
- Amish farmland
- Temperate climate
- Limestone watershed
- Fox territory

Each of these regions answers a different class of questions.

Cardinal intentionally avoids forcing locations into a single hierarchy.

Instead, overlapping regional classifications are expected and encouraged.

This allows different domains to interpret the same world through their own lens without duplicating physical information.



---

# 1.8 Containment

Not every physical relationship is measured by distance.

Some are defined by enclosure.

Containment answers questions such as:

- What is inside this building?
- Which inventory contains this tool?
- Which room contains this person?
- Which valley contains this village?
- Which planet contains this ecosystem?

Containment establishes context.

Every entity should be able to answer not only *where* it exists, but also *within what* it exists.

## Hierarchical Containment

Containment often forms natural hierarchies.

For example:

```
Planet
└── Continent
    └── Region
        └── City
            └── Building
                └── Room
                    └── Cabinet
                        └── Drawer
                            └── Object
```

This hierarchy is conceptual rather than prescriptive.

World Packages are free to introduce different containment structures while preserving the same architectural intent.

A spacecraft, coral reef, ant colony, or fantasy dungeon may each expose very different hierarchies.

The important property is that containment provides increasingly refined context.

## Dynamic Containment

Containment is not always static.

A crate loaded onto a wagon changes containment.

A fish caught in a net changes containment.

Passengers boarding a train become part of a moving container.

Clouds envelop mountains.

Floodwaters temporarily contain roads.

The Physical Reality domain should therefore treat containment as state rather than immutable structure.

---

# 1.9 Materials

Space describes where reality exists.

Materials describe what reality is composed of.

Every physical object ultimately exists because some collection of materials occupies space.

Cardinal intentionally avoids prescribing a universal material catalog.

Instead, materials are expected to expose characteristics rather than identities.

Examples include:

- Density
- Hardness
- Elasticity
- Conductivity
- Transparency
- Porosity
- Flammability
- Thermal capacity
- Toxicity
- Brittleness

Higher-level domains reason about these properties instead of hardcoded names.

A bridge does not fail because it is "wood."

It fails because the material cannot support the required stress.

A cave floods because its permeability allows water movement.

Fire spreads because nearby materials satisfy ignition conditions.

This property-oriented approach naturally supports future materials without changing engine behavior.

## Composite Materials

Real objects rarely consist of one substance.

A house contains timber, glass, steel, insulation, wiring, and concrete.

A tree contains bark, sap, leaves, water, and air pockets.

Composite materials should therefore be expected rather than treated as exceptional.

Simulation fidelity can increase over time without requiring architectural redesign.

---

# Designer Note
## Properties Over Names

Architectures that reason about named materials eventually accumulate endless special cases.

Architectures that reason about properties naturally generalize.

A future World Package may invent crystal, mithril, graphene, living stone, or programmable matter.

If those materials expose meaningful properties, the rest of Cardinal should require little or no modification to interact with them.



---

# 1.10 Environmental State

Space alone is static.

Reality is not.

Every location in the world exists within an ever-changing environmental context. Temperature shifts, humidity rises, rivers flood, seasons change, sunlight moves across the landscape, and storms reshape the conditions experienced by every living thing.

Rather than treating these as isolated systems, Cardinal views them as facets of a shared environmental state.

Environmental state describes the measurable conditions associated with a location at a particular moment in time.

Examples include:

- Temperature
- Atmospheric pressure
- Humidity
- Wind
- Illumination
- Water level
- Soil moisture
- Air quality
- Radiation
- Magical influence (for fantasy worlds)

Physical Reality owns these values because they are properties of locations rather than properties of organisms.

A deer experiences cold.

The forest owns the temperature.

## Environmental Fields

Many environmental values are best understood as continuous fields rather than discrete objects.

Temperature does not belong to individual tiles.

It varies across space.

Wind flows.

Smoke diffuses.

Sound propagates.

Radiation diminishes with distance.

Implementations may choose grids, voxels, graphs, sparse samples, mathematical functions, or entirely different techniques to represent these fields.

The architecture only requires that higher domains can consistently query environmental conditions.

## Time and Change

Environmental state is expected to evolve continuously.

Some changes are predictable.

- Sunrise
- Sunset
- Seasons
- Tides

Others are emergent.

- Wildfires
- Dust storms
- Volcanic eruptions
- Floods

Still others are entirely dependent on the World Package.

The Physical Reality domain should therefore avoid assuming fixed update rates or specific simulation models.

Instead it provides the shared context upon which specialized systems operate.

---

# 1.11 Physical Constraints

Reality is defined as much by limitations as by possibilities.

Physical constraints describe what cannot occur.

Examples include:

- Solid objects cannot occupy the same space.
- Mountains obstruct direct travel.
- Water flows downhill.
- Walls block vision.
- Closed doors prevent movement.
- Thick fog limits visibility.
- Vacuum prevents unprotected respiration.

These constraints are not gameplay rules.

They are consequences of the world's physical state.

Higher-level domains should discover these constraints through queries against Physical Reality rather than embedding assumptions of their own.

## Constraint Composition

Constraints frequently overlap.

A traveler may be blocked simultaneously by:

- terrain
- political borders
- darkness
- flooding
- hostile wildlife

Each constraint originates from a different domain, yet all contribute to the final outcome.

This illustrates one of Cardinal's central architectural principles:

No domain should need complete knowledge of another.

Each contributes its own facts.

Emergent behavior arises from their composition.

---

# Designer Note
## Reality as Shared Context

The purpose of the Physical Reality domain is not to answer every question.

Its purpose is to provide a trustworthy foundation from which other domains can answer their own questions.

Living Systems asks:

"Can I survive here?"

Economy asks:

"Can I transport goods here?"

Society asks:

"Can people settle here?"

Conflict asks:

"Can I attack from here?"

Each domain sees the same reality through a different lens.

That shared foundation is what allows independently developed systems to interact without bespoke integrations.



---

# 1.12 Querying Reality

The Physical Reality domain exists primarily to answer questions.

Contrary to intuition, most simulation systems do not spend the majority of their time updating the world.

They spend it asking questions about the world.

Examples include:

- Which predators are nearby?
- Which buildings can this citizen reach?
- What is the closest source of fresh water?
- Can this signal travel between two settlements?
- Is this tree exposed to sunlight?
- Which villages are downstream?
- What objects occupy this room?

The architecture should therefore optimize for answering questions rather than merely storing information.

## Read-Mostly Architecture

Reality changes continuously, but it is observed far more often than it is modified.

Thousands of organisms may query the same forest before a single tree falls.

Millions of visibility calculations may occur before sunrise changes the lighting.

This asymmetry suggests an important architectural principle:

> The world should be inexpensive to observe.

Implementations are encouraged to organize spatial data in ways that accelerate common queries while remaining free to choose the underlying structures that best fit the simulation.

No particular indexing strategy is mandated.

Future implementations may employ spatial trees, graphs, sparse fields, hierarchical partitions, procedural generators, distributed storage, or techniques not yet invented.

The contract remains the same:

Reality must answer questions consistently.

## Query Composition

Powerful simulations emerge from composing simple questions.

For example, "Find the safest campsite" is not a primitive operation.

Instead, it may be derived from a series of independent queries:

- Is the terrain flat?
- Is fresh water nearby?
- Is the location dry?
- Is fuel available?
- Is predator activity low?
- Is the area sheltered from wind?
- Is ownership permitted?

Each answer originates from one or more domains.

No domain needs explicit knowledge of the final decision.

This pattern appears repeatedly throughout Cardinal.

Complex behavior should emerge from combining simple truths.

---

# 1.13 Domain Interaction Contracts

Physical Reality serves as one of the primary providers of shared context across Cardinal.

Rather than exposing implementation details, it exposes stable concepts that other domains can depend upon.

## Living Systems

Consumes:

- terrain
- environmental state
- material properties
- shelter
- water
- climate
- containment

Provides:

- organisms
- biomass
- ecological modification

## Economy

Consumes:

- distance
- transport networks
- regions
- resources
- accessibility

Provides:

- infrastructure
- extraction sites
- construction

## Society

Consumes:

- geography
- settlement locations
- barriers
- climate
- regional identity

Provides:

- cities
- borders
- ownership
- infrastructure

## Conflict

Consumes:

- visibility
- terrain
- cover
- movement constraints
- fortifications

Provides:

- destruction
- fortifications
- environmental damage

Notice the recurring pattern.

Physical Reality rarely owns the actors.

It owns the stage upon which they act.

---

# Designer Note
## The Stage Never Leaves

Actors come and go.

Civilizations rise and collapse.

Species evolve.

Governments disappear.

Roads decay.

Wars reshape borders.

Yet the mountain remains.

The river continues to flow.

The valley continues to collect rain.

One of the defining characteristics of the Physical Reality domain is its relative permanence.

It provides continuity across generations of simulated history.

That continuity allows history itself to emerge as something more than a sequence of disconnected events.



---

# 1.14 Engineering Invariants

As implementations evolve, certain architectural truths should remain stable. These invariants are not implementation requirements but guiding principles that preserve interoperability between domains.

## Single Source of Physical Truth

A physical fact should have one authoritative owner.

If elevation is represented in multiple locations, one representation must be considered canonical while the others are treated as derived or cached views.

Duplicating authority inevitably creates divergence.

## Representation Independence

No consumer should depend on whether space is represented internally as:

- grids
- quadtrees
- octrees
- graphs
- voxels
- procedural functions
- sparse chunks

Consumers ask questions.

Physical Reality decides how those answers are obtained.

This separation allows implementations to evolve without cascading changes throughout the engine.

## Stable Identity

Places are more than coordinates.

A mountain pass that shifts internally because of chunk streaming is still the same mountain pass.

Regions, landmarks, rivers, and other persistent features should possess identities independent of their storage representation whenever practical.

Doing so enables history, storytelling, navigation, and long-lived references across simulation time.

---

# 1.15 Performance Philosophy

Performance should emerge from architecture rather than shortcuts.

Cardinal intentionally avoids recommending premature optimization techniques as architectural requirements.

Instead, implementations are encouraged to identify expensive questions and optimize the mechanisms used to answer them.

The architecture should never require sacrificing correctness merely to achieve acceptable performance.

Caching, spatial partitioning, asynchronous generation, procedural reconstruction, streaming, and approximation are all implementation strategies.

They are not architectural concepts.

If a faster implementation is discovered in ten years, the architecture should remain valid.

## Scale Through Locality

Reality is overwhelmingly local.

A squirrel rarely needs to know conditions on another continent.

A farmer generally reasons about neighboring fields rather than global geography.

Implementations should exploit this locality whenever possible.

The architecture neither requires nor forbids global knowledge.

It merely recognizes that most meaningful interactions occur within constrained neighborhoods.

---

# 1.16 Future Evolution

The Physical Reality domain is intentionally incomplete.

Future versions of Cardinal may introduce concepts such as:

- relativistic space
- orbital mechanics
- deformable terrain
- plate tectonics
- volumetric oceans
- fluid dynamics
- erosion
- magnetic fields
- subterranean geology
- programmable materials

None of these additions should require redefining the architectural purpose of the domain.

Instead, they extend the richness of reality while preserving the same conceptual responsibilities established in this chapter.

The measure of a healthy architecture is not whether it predicts every future feature.

It is whether future features can be added without rewriting its foundations.



---

# 1.17 Closing Thoughts

Every simulated world, regardless of genre or fidelity, ultimately rests upon an answer to one deceptively simple question:

**"What does it mean for something to exist?"**

Within Cardinal, existence begins with Physical Reality.

Before organisms evolve, there must be an environment in which evolution is possible.

Before civilizations emerge, there must be geography capable of supporting settlement.

Before economies arise, there must be distance, resources, and movement.

Before history can unfold, there must be a world capable of remembering where events occurred.

For this reason, Physical Reality should be viewed as the foundation of every other domain rather than merely another subsystem.

It provides the stable frame of reference that allows independently developed systems to cooperate without direct knowledge of one another.

The Physical Reality domain deliberately avoids solving every problem.

Instead, it provides the vocabulary with which every other domain can express its own problems.

This distinction is subtle but fundamental.

A robust architecture does not answer every question.

It ensures that every question can be asked consistently.

As Cardinal evolves, individual implementations of terrain, materials, environmental simulation, and spatial indexing will undoubtedly change.

Some will become faster.

Some will become more accurate.

Some will be replaced entirely.

Those changes should leave the architectural contracts described in this chapter intact.

The map may change.

Reality should not.

---

## Preparing for the Next Domain

With a coherent physical world established, Cardinal can introduce its first dynamic inhabitants.

The next chapter explores **Living Systems**: the domain responsible for organisms, survival, growth, adaptation, reproduction, and the emergence of ecosystems.

Where Physical Reality answers **where** existence occurs, Living Systems begins answering **what** exists and **how it persists**.

---

# END OF CHAPTER 1
