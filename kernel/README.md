# kernel/

Volume II made executable: facts, ticks, commits, determinism. The kernel knows fact
types, proposals, and streams — it has never heard of a deer, a price, or a sword
(Vol. IV Ch. 1, Designer Note).

**Governing spec:** Vol. V Ch. 1 (layer law), Ch. 2 (Reality Store), Ch. 3 (tick
pipeline & scheduler), Ch. 4 (determinism); Vol. II throughout.

## Planned structure

```text
kernel/
├── store/       — the Reality Store behind its contract (Vol. V Ch. 2 §2.1):
│                  resolve/read/query/apply/snapshot/history; apply() is the ONLY
│                  mutation path in the entire engine
├── tick/        — the seven-stage pipeline (Vol. V Ch. 3 §3.1): schedule, evaluate,
│                  resolve, validate, commit, chronicle, observe
├── scheduler/   — cadence calendar + DAG ordering from declared read/write sets;
│                  execution order is derived, deterministic, and published (§3.2)
├── rng/         — the seeded stream forest; substreams issued per (system, tick,
│                  scope-key), content-keyed, counter-based (Vol. V Ch. 4 §4.1)
├── events/      — chronicle assembly from proposal causes; append-only, causally
│                  linked, indexed (Vol. V Ch. 6 §6.1)
├── identity/    — permanent ids, tombstones, no reuse ever (Vol. II; Vol. V Ch. 2 §2.1.4)
└── hash/        — canonical per-tick state hashing (Vol. V Ch. 4 §4.2 — build this
                   EARLY; every determinism test reduces to comparing it)
```

## Law binding this directory

- Small kernel, heavy law: no domain knowledge, no world knowledge, no conveniences
  (Vol. V Ch. 1, Designer Note). A kernel still gaining features in year three is
  absorbing someone's domain.
- Systems evaluate hermetically: scoped committed reads in, proposals out, nothing else
  (Vol. V Ch. 3 §3.1). No wall-clock, no OS entropy, no unordered iteration on any
  simulation path (Vol. V Ch. 4 §4.1, Doors 1–4).
- A failed tick leaves reality exactly at N−1 (Vol. V Ch. 3 §3.5.5).
- Simpler execution profiles (e.g. the POC's serialized delta application) are valid only
  under proven observational equivalence to the canonical pipeline (Vol. V Ch. 3 §3.5.10).
