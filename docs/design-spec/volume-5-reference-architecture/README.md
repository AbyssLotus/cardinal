# Cardinal Architecture Specification — Volume V: Reference Architecture

The construction volume: how Cardinal should be built without freezing innovation. It
discusses tradeoffs rather than dictating implementations — where it names a technique it
gives reasons; where it names an invariant it is restating law from Volumes I–IV. The
existing engine is cited where its scars validate a pattern; where a more robust or
modular design exists, this volume specifies the better design.

## Chapters

1. [The Shape of the Engine](01_The_Shape_of_the_Engine.md) — the four-layer model, dependency law, bootstrap sequence, project layout
2. [Representing Reality](02_Representing_Reality.md) — the Reality Store contract; ECS, relational, fact-store, and hybrid designs compared; memory principles
3. [The Simulation Kernel](03_The_Simulation_Kernel.md) — the tick pipeline, proposals, the scheduler, and execution profiles under observational equivalence
4. [Determinism](04_Determinism.md) — the five doors nondeterminism enters by, their locks, and the alarm system that catches drift
5. [Scale: Parallelism, Streaming, Networking](05_Scale.md) — three directions of scale under one law: the village cannot tell
6. [Events and Messaging](06_Events_and_Messaging.md) — events as history not control flow; the chronicle store; the read-only observation plane
7. [Persistence Machinery](07_Persistence_Machinery.md) — write paths, formats, serialization discipline, and migration engineering
8. [Observability, Debugging, and Testing](08_Observability_Debugging_Testing.md) — meters, the causal debugger, time travel, and the six-layer test pyramid
9. [AI Integration](09_AI_Integration.md) — the narrator, the interpreter, and agent minds: models at the doors, never in the walls
10. [Extension and Evolution](10_Extension_and_Evolution.md) — the five plugin seams, performance philosophy, specification amendment, and the roadmap forward

The construction roadmap for Volumes III–V lives at [../ROADMAP.md](../ROADMAP.md).
