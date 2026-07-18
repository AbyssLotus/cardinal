# Project Cardinal

A persistent world simulation engine. Reality is authoritative, simulation precedes
narration, and narrative emerges from system interaction — the narrator only describes
state the simulation has already computed and committed.

**The specification comes first.** Cardinal is built against the five-volume
[Cardinal Architecture Specification](docs/design-spec/README.md):

| Volume | Question it answers |
|---|---|
| [I — First Principles](docs/design-spec/volume-1-first-principles/README.md) | Why does Cardinal exist, and what may never be violated? |
| [II — Constructing Reality](docs/design-spec/volume-2-constructing-reality/README.md) | How is truth represented, advanced, and preserved? |
| [III — Domains of Reality](docs/design-spec/volume-3-domains-of-reality/README.md) | What exists, and who owns each fact? |
| [IV — World Packages](docs/design-spec/volume-4-world-packages/README.md) | How are radically different worlds assembled from one engine? |
| [V — Reference Architecture](docs/design-spec/volume-5-reference-architecture/README.md) | How should the engine be built? |

When code and specification disagree, the specification wins — or is explicitly amended
first (Vol. V Ch. 10 §10.3). Boundary questions argue from
[Appendix A: The Ownership Matrix](docs/design-spec/volume-3-domains-of-reality/APPENDIX_A_Ownership_Matrix.md)
as precedent.

## Repository layout

The source tree is Vol. V Ch. 1's four-layer model, made visible (§1.3: *structure should
read as the architecture*):

```text
kernel/       — the law, executable: reality store, tick, scheduler, RNG, events, identity
domains/      — one module per Volume III domain; never import each other
services/     — persistence, packages, observability, replay: operate around the world
frontends/    — CLI, narrator, tools: consume entitled streams, submit validated input
worlds/       — world packages: content, never code (Vol. IV)
tests/        — the six-layer pyramid (Vol. V Ch. 8 §8.3): kernel, domains, probes, reference
docs/         — the specification and its archives
archive/      — the frozen v0.1 proof of concept (evidence, not precedent)
```

### The dependency law (Vol. V Ch. 1 §1.1 — non-negotiable)

1. Dependencies point downward only: `frontends → services → domains → kernel`.
2. Domains never import domains. Cross-domain effect is committed reality, full stop.
3. Nothing imports world content. Packages arrive through the loader as data, or not at all.
4. Every layer's public surface is explicit; bypass is a build failure, not a code review note.

## Status

**Rebuild phase — scaffold.** The v0.1 proof of concept (six milestones, playable,
365-day-stable) is archived at [archive/poc-v0.1](archive/poc-v0.1/README.md). The
reference engine is being built here, from the specification, in roadmap order
(Vol. V Ch. 10 §10.4):

- [ ] Kernel: reality store contract + tick pipeline (Vol. V Ch. 2–3)
- [ ] Determinism harness: per-tick state hashing, twin-run CI (Vol. V Ch. 4 — *the
      cheapest alarm, first*)
- [ ] Domain registration + first domains (Physical Reality is mandatory: Vol. IV Ch. 2 §2.1)
- [ ] Package loader + validation stack (Vol. IV Ch. 3, Ch. 7)
- [ ] Persistence: chronicle tail + snapshots, two-road recovery (Vol. V Ch. 7)
- [ ] First probe worlds, then the five reference worlds (Vol. IV Ch. 8)

## Hard rules (inherited, still binding)

- The engine contains zero world-specific content (Vol. I; Vol. IV Ch. 1 §1.5.1).
- A full simulation tick runs with the narrator disabled; LLM output is never a
  dependency of state computation (Vol. I; Vol. V Ch. 9 §9.5.2).
- Every tunable number lives in a world package, never in engine code (Vol. IV Ch. 2 §2.2).
