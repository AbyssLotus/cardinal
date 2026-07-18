# Cardinal Architecture Specification Roadmap

## Volumes III--V

> This document defines the remaining scope of the Cardinal Architecture
> Specification. It is not a table of contents as much as a construction
> roadmap. The objective is to finish documenting the architecture
> before committing to implementation details.

## Guiding Principle

Volumes I and II explained **why** Cardinal exists and **how** the
engine is organized.

Volumes III through V answer three different questions:

-   **Volume III:** *What exists?* (Simulation domains)
-   **Volume IV:** *How are worlds assembled?* (World composition)
-   **Volume V:** *How should Cardinal be built?* (Reference
    implementation)

Together they move from philosophy to executable architecture.

------------------------------------------------------------------------

# Volume III --- Domains of Reality

Purpose: Define every major simulation domain, its responsibilities,
boundaries, interactions, and architectural contracts.

Each chapter should include:

-   Purpose
-   Responsibilities
-   Non-responsibilities
-   Canonical concepts
-   Domain interactions
-   Common queries
-   Architectural contracts
-   Engineering invariants
-   Anti-patterns
-   Future evolution

Planned chapters:

1.  Physical Reality
2.  Living Systems
3.  Resources
4.  Economy
5.  Society
6.  Culture
7.  Knowledge
8.  Conflict
9.  Institutions
10. Ecology
11. Emergence
12. Cross-Domain Interaction

The emphasis is on ownership and relationships, not algorithms.

------------------------------------------------------------------------

# Volume IV --- World Packages

Purpose: Explain how radically different simulations can be created from
the same engine.

Topics include:

-   World Packages
-   Configuration
-   Simulation Rules
-   Domain Selection
-   Data Packs
-   Content Pipelines
-   Modularity
-   Procedural Generation
-   World Generation
-   Scenario Authoring
-   Save Formats
-   Versioning
-   Migration
-   Validation
-   Testing World Packages

Example packages should include a medieval kingdom, modern city, ocean
ecosystem, space colony, and fantasy world to demonstrate architectural
flexibility rather than game mechanics.

------------------------------------------------------------------------

# Volume V --- Reference Architecture

Purpose: Provide guidance for building Cardinal without freezing
innovation.

Topics include:

-   Engine bootstrap
-   Project layout
-   Module boundaries
-   ECS and alternatives
-   Messaging
-   Event systems
-   Scheduling
-   Simulation loop
-   Persistence
-   Serialization
-   Networking
-   Determinism
-   Parallelism
-   Streaming
-   Memory management
-   Observability
-   Debugging
-   Testing
-   AI integration
-   Plugin architecture
-   Performance philosophy
-   Future roadmap

This volume should discuss tradeoffs rather than dictate exact
implementations.

------------------------------------------------------------------------

# Documentation Standards

Every chapter should read like an engineering reference rather than
documentation.

Prefer explaining:

-   Why a concept exists.
-   What it owns.
-   What it deliberately does not own.
-   How it collaborates with other domains.
-   How future implementations may evolve.

Avoid locking Cardinal into specific languages, libraries, databases, or
algorithms unless clearly labeled as a reference implementation.

------------------------------------------------------------------------

# Deliverable

By the completion of Volume V, the Cardinal Architecture Specification
should allow an experienced engineer or future Claude Code session to
implement Cardinal from first principles while preserving the philosophy
established in Volumes I and II.

The specification should outlive any individual implementation and serve
as the enduring architectural foundation of the Cardinal project.
