# Volume II
# Constructing Reality
## Chapter 5
# Persistence

> *Persistence is not saving the world. Persistence is preserving reality.*

---

## Why Persistence Exists

Most engines treat persistence as serialization.

When the player quits, the engine writes memory to disk. When the player returns, it restores memory.

Cardinal requires a stronger guarantee.

The persisted world must be sufficient to reconstruct the exact reality that existed at a specific point in simulation time.

Persistence is therefore an architectural concern, not a convenience feature.

---

## The Persistence Contract

The persistence layer SHALL preserve:

- Identity
- Facts
- Relationships
- Historical events
- Simulation time
- World configuration
- Random number generator state
- Schema version

Nothing else is required.

Behavior is regenerated.

Decisions are regenerated.

Derived statistics are regenerated.

Reality is preserved.

---

## Persistence is Below the Simulation

The simulation never writes files.

It commits a new reality.

Persistence observes committed realities and records them.

```text
Systems
    │
    ▼
Commit Reality
    │
    ▼
Persistence
    │
    ▼
Storage
```

This separation allows the storage implementation to evolve independently of the engine.

---

## Snapshots

A snapshot is a complete representation of reality at one simulation tick.

A snapshot must be internally consistent.

It shall never represent a partially committed world.

Snapshots are useful for:

- loading
- checkpoints
- debugging
- testing
- branching simulations

---

## Event History

Snapshots answer:

"What does the world look like?"

Events answer:

"How did it become this way?"

Examples:

```text
Birth

Marriage

War Declared

Bridge Collapsed

Crop Failure
```

Events are immutable.

They explain transitions between realities.

---

## Versioning

Worlds outlive engine versions.

Persistence therefore requires explicit schema versioning.

Every persisted world SHALL record:

- engine version
- schema version
- world package version

Migration is performed explicitly rather than implicitly.

---

## Identity Preservation

Entity identifiers are persistent.

Reloading a world must never assign new identities.

If Entity #1042 existed before persistence, Entity #1042 must exist afterward.

Identity continuity is essential for history, relationships, and debugging.

---

## Referential Integrity

Relationships must remain valid after loading.

Invalid references indicate corruption.

Example:

```text
Citizen
    │
Lives In
    ▼
Town
```

If the town no longer exists, the persistence layer must detect and report the inconsistency.

---

## Deterministic Reload

Loading a saved world and immediately running the next tick shall produce the same result as if the simulation had never stopped.

Persistence is correct only if it preserves determinism.

---

## Storage Independence

The engine shall not depend on a specific storage technology.

Possible implementations include:

- binary files
- SQLite
- PostgreSQL
- object storage
- distributed databases

Storage is an implementation detail.

The persistence contract remains unchanged.

---

## Compression

Compression is permitted.

Loss of simulation fidelity is not.

Persistence may optimize representation but may never alter meaning.

---

## Validation

Persisted data should be validated before becoming active reality.

Validation includes:

- schema compatibility
- identity uniqueness
- reference integrity
- required fact presence
- world package compatibility

Invalid worlds fail before simulation begins.

---

## Engineering Invariants

Every implementation SHALL preserve these rules.

1. Persistence records committed reality only.
2. Snapshots are internally consistent.
3. Events are immutable.
4. Entity identity survives persistence.
5. Relationships retain referential integrity.
6. Schema versioning is explicit.
7. Storage technology is replaceable.
8. Loading preserves determinism.
9. Compression never changes semantics.
10. Validation occurs before activation.

---

## Common Implementation Mistakes

### Saving Derived Data

Statistics, caches, and indexes should be rebuilt after loading whenever practical.

### Saving Mid-Tick

Persisting an uncommitted world creates impossible states.

### Engine-Coupled Formats

Serialization formats tied directly to in-memory structures become fragile as the engine evolves.

### Ignoring Versioning

Schema migration should be deliberate and testable, never accidental.

---

## Future Considerations

Future versions of Cardinal may support:

- incremental snapshots
- event sourcing
- branching timelines
- cloud synchronization
- distributed persistence
- historical queries
- replay databases

These capabilities extend the persistence layer without changing its responsibility.

Persistence exists for one purpose:

To ensure that reality survives beyond the lifetime of the process that simulated it.
