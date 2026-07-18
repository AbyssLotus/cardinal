# services/persistence/

Vol. V Ch. 7 — Persistence Machinery, implementing the contracts of Vol. II Ch. 5 and
Vol. IV Ch. 6.

Planned: chronicle tail appender (every tick), snapshot capture (whole-tick-consistent
views, CoW where the store allows), the durable frontier, crash recovery
(kill-and-recover CI), self-describing chunked formats decoupled from memory layout,
the schema registry, and migration engines — format migrations as chunk-streaming
transforms; package migrations *through the engine* as chronicled batches via the
single write path (Ch. 7 §7.3). Dry-run reports before any surgery.

**Law:** persists state and provenance, never definition or derivation (Vol. IV Ch. 6
§6.1); restore and replay must agree (Ch. 7, Designer Note — run the audit in CI);
the chronicle is never amputated.
