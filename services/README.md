# services/

Operators around the world, never participants in it. Services subscribe to the
observation plane (Vol. V Ch. 6 §6.2) — read-only fan-out of committed state and
events — and can influence simulation content in exactly zero ways. The one nuance:
durability subscribers may gate acknowledged *progress*, never alter a committed tick.

**Governing spec:** Vol. V Ch. 1 (layer law), Ch. 6 (the observation plane),
Ch. 7 (persistence), Ch. 8 (observability); Vol. IV Ch. 3, 6, 7 (packages, saves,
validation).

| Directory | Duty | Spec |
|---|---|---|
| `persistence/` | chronicle tail + snapshots; two-road recovery (restore and replay agree) | Vol. V Ch. 7 |
| `packages/` | loader, composition pipeline, five-layer validation stack | Vol. IV Ch. 3, Ch. 7 |
| `observe/` | meters, telemetry, causal-chain queries, liveness envelopes | Vol. V Ch. 8 |
| `replay/` | twin runs, save-splice, divergence bisection, time travel | Vol. V Ch. 4, Ch. 8 |

**Law:** no backchannels (Vol. V Ch. 6 §6.5) — a service that writes a flag the
simulation reads has voided the security model of three chapters. Validation is
read-only; a validator that fixes what it finds is an unchronicled writer
(Vol. IV Ch. 7 §7.1).
