# Volume II
# Constructing Reality
## Chapter 2
# Time

> *Time is not a measurement. It is the mechanism by which reality changes.*

---

## Why Time Matters

Most game engines treat time as a continuously increasing floating-point value. Systems receive a `deltaTime`, update themselves independently, and produce a new frame.

Cardinal deliberately rejects this model.

Rendering may be continuous.

Reality is not.

Reality advances through discrete, deterministic transitions. Every change to the world belongs to exactly one simulation tick. Between ticks, the world is immutable.

This decision exists for one reason:

**The same inputs must always produce the same world.**

---

## The Simulation Clock

Cardinal defines two independent clocks.

### Simulation Time

Simulation time represents the authoritative progression of reality.

```text
Year → Day → Tick
```

Simulation time is:

- deterministic
- persistent
- replayable
- independent of wall-clock time

### Wall Time

Wall time is the host computer's clock.

It determines how quickly simulation ticks are processed, but it never becomes part of the simulation itself.

Stopping the simulation pauses reality. Advancing faster simply processes more ticks per second.

---

## The Tick

A tick is the smallest indivisible unit of change.

Within a tick:

1. Reality is read.
2. Systems evaluate the current state.
3. Systems propose changes.
4. Conflicts are resolved.
5. All accepted changes are committed atomically.
6. History is recorded.

No system may observe a partially updated world.

---

## Determinism

Determinism is not an optimization.

It is an architectural requirement.

Given:

- identical initial world state
- identical world package
- identical inputs
- identical random seed

Cardinal SHALL produce identical results.

This enables:

- replay
- debugging
- multiplayer synchronization
- regression testing
- historical simulation

Any subsystem that introduces non-deterministic behavior violates the engine contract.

---

## Event Ordering

Many events appear simultaneous.

They are not.

Cardinal defines a total ordering for every change within a tick.

Example:

```text
Tick 41822

Movement
↓

Collision

↓

Damage

↓

Death

↓

Inventory Transfer

↓

Quest Update
```

The ordering itself becomes part of the engine specification.

Changing the order changes reality.

---

## Reads and Writes

Systems never modify reality directly.

Instead they operate in two phases.

### Read Phase

Systems inspect the committed world.

No mutations are allowed.

### Proposal Phase

Systems produce proposed changes.

Example:

```text
Health

82 → 54

Position

(10,12) → (11,12)
```

### Commit Phase

After conflict resolution, accepted changes become the new reality.

This guarantees every system reasons about the same world state.

---

## Conflict Resolution

Multiple systems may propose incompatible changes.

Example:

```text
Combat
Health = 0

Healing
Health = 30
```

The scheduler must resolve conflicts according to documented precedence rules before committing the next world state.

Conflict resolution is deterministic and repeatable.

---

## Replay

Because every tick is deterministic, reality can be reconstructed.

A replay requires:

- initial snapshot
- event log
- deterministic engine version

The resulting world must exactly match the original execution.

Replay is an architectural capability, not a debugging feature.

---

## Rollback

Rollback creates an earlier world state without changing history.

Possible implementations include:

- periodic snapshots
- event sourcing
- delta chains
- hybrid checkpointing

Rollback exists to support tooling, testing, and debugging.

It is not equivalent to rewriting history inside the simulation.

---

## Scheduling

Not every system must execute every tick.

Each system declares its scheduling policy.

Examples:

```text
Movement
Every Tick

Agriculture
Every Hour

Politics
Daily

Economy
Daily

Geology
Monthly
```

Lower-frequency systems still observe deterministic simulation time.

Scheduling changes performance, never correctness.

---

## Randomness

Random numbers are inputs.

They are never requested directly from the operating system.

Instead Cardinal uses deterministic random streams.

Each stream is seeded from the simulation.

```text
World Seed
↓

Weather RNG

Combat RNG

Economy RNG

Ecology RNG
```

Independent streams prevent unrelated systems from influencing one another.

---

## Parallel Execution

Systems may execute in parallel only when their read/write dependencies permit.

Parallelism must never alter the observable result.

Correctness always has higher priority than throughput.

---

## Time as History

Each committed tick creates a new historical state.

History is therefore an ordered sequence of realities.

```text
Reality(t0)
↓

Reality(t1)
↓

Reality(t2)
↓

Reality(t3)
```

Events explain why transitions occurred.

Reality records what became true.

---

## Engineering Invariants

Every implementation SHALL preserve the following rules.

1. Reality changes only at tick boundaries.
2. A tick is an atomic transaction.
3. Systems read committed state and propose changes.
4. World updates are deterministic.
5. Event ordering is explicitly defined.
6. Randomness is deterministic and reproducible.
7. Scheduling affects performance, not correctness.
8. Replay must reconstruct identical reality.
9. Rollback never rewrites simulated history.
10. Parallel execution must be observationally equivalent to sequential execution.

Time is therefore more than a counter.

It is the contract that preserves causality across the entire simulation.
