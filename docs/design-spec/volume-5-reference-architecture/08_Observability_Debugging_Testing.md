# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 8
# Observability, Debugging, and Testing

> *A simulation you cannot interrogate is a rumor you are running in production.*

---

# Chapter Overview

Cardinal's kind of correctness is unusually hard to see. A crash announces itself; a wolf population quietly starving because income never restores — the repo's own founding telemetry catch — announces nothing. The engine's failure modes are *worlds that are subtly wrong*, and subtle wrongness is invisible without instruments.

This chapter builds the instruments. Its three sections are one discipline at three timescales:

**Observability** — knowing what the world is doing, continuously.
**Debugging** — explaining what the world did, after the fact.
**Testing** — proving what the world will do, before release.

All three stand on ground earlier chapters prepared, and it is worth seeing how deliberately: determinism (Ch. 4) makes every bug reproducible; the chronicle (Ch. 6) makes every outcome explicable; two-road persistence (Ch. 7) makes every moment revisitable; and the read-only observation plane (Ch. 6) makes all of it safe. Cardinal is, almost by accident of its own law, the most debuggable class of engine it is possible to build. This chapter collects the payout.

---

# 8.1 Observability

## Meters

The engine and every domain export meters — continuously computed, read-only measurements — through the observation plane:

```text
Kernel meters:    tick duration by stage, proposals/tick, conflicts resolved,
                  arena high-water, hash time, durable-frontier lag (Ch. 7)
Store meters:     residency by rung/region (Ch. 5), tier sizes, query profiles
Domain meters:    per domain, declared alongside its systems —
                  vitals distributions, market clearing volumes, population
                  trajectories, grievance counts, legitimacy spreads
World meters:     the package's liveness envelope quantities (Vol. IV Ch. 7),
                  metered live rather than only in soaks
```

Two rules keep meters honest. **Meters are derived, never authoritative** — Volume II's derived-data law; a meter is recomputable from committed state, cached at the plane, persisted never. **Meters consume no streams** — Chapter 4's rolling-debug-print door, bolted: observation must not perturb a single RNG draw.

## Telemetry and the Balance Report

Meters over time become telemetry — the `--report` pattern of the current engine, generalized: any headless run can emit its meter series, and the *same* series drive the live dashboard, the soak analysis, and the reference-signature comparison (Vol. IV Ch. 7). One instrumentation, three consumers; a phenomenon visible in a dashboard is automatically testable in CI, because they read the same numbers.

## The Health of Silence

The subtlest meters watch for *absence* — the green dead world (Vol. IV Ch. 7): chronicle growth rate, event-type diversity, cross-domain chain frequency. A world whose meters are all steady and whose chronicle has stopped saying anything new is not healthy; it is embalmed. Silence alarms are liveness envelopes running continuously.

---

# 8.2 Debugging

Debugging Cardinal is answering *"why is the world like this?"* — and the architecture has already made the answer data.

## The Causal Debugger

The first-class tool, built on chronicle chain queries (Ch. 6):

```text
why fact(X)?         — ancestry: the event chain that produced this value
what-did event(E)?   — descent: everything downstream, cross-domain
trace(A ⇒ B)?        — path: how the drought reached the price
```

This tool is the debugger *most* engine work actually needs — the fifteen-month archaeology dig of Chapter 4's designer note, reduced to a query. It is read-only, runs against live worlds and saves alike, and its language is the historian's language (Vol. III Ch. 11's ask, delivered as tooling).

## Time Travel

Two-road persistence makes every past tick reachable: restore to snapshot, replay to the moment. The debugging moves this unlocks:

- **Rewind-and-watch.** Return to tick N−k with instruments attached that weren't attached live; the replay is identical (Ch. 4), so the bug performs on demand.
- **Divergence bisection.** Twin-run drift at tick T? Binary-search the interval with per-tick hashes to the first differing commit, then diff that tick's proposals to the offending system. Determinism turns "sometimes different" into "this line."
- **Counterfactual runs.** Branch from tick N with one perturbed input (a rule, an action) and diff the futures — Volume III Chapter 11's sensitivity analysis as a debugging verb. Branches are labeled *studies*, never mergeable into the original's history (Vol. II's rollback law).

## The Inspector

The humble tool, specified so it is not forgotten: any entity, fact, region, or event, pretty-printed with provenance, relationships, and meter context — the current CLI's `inspect`, grown up. Most sessions start here; the causal debugger is where they go next.

---

# 8.3 Testing

The pyramid, from fastest to deepest — the lower layers are conventional; the upper layers are Cardinal's own:

```text
1. Unit          — kernel pieces, store contract conformance (Ch. 2),
                   pure system evaluation (hermeticity makes this trivial:
                   committed view in, proposals out, assert)
2. Contract      — per-domain: schemas, composition rules, read/write
                   declarations honored (undeclared reads are test failures)
3. Determinism   — twin runs, save-splice, golden transcripts,
                   cross-platform hashes, chaos ordering (Ch. 4's alarms)
4. Conservation  — soak meters reconciling every conserved quantity
                   (Vol. IV Ch. 7; Kepler is this layer as a world)
5. Liveness      — headless activity envelopes per reference world
6. Phenomenology — emergence signatures over reference-world centuries
                   (Vol. IV Ch. 7; the five worlds of Vol. IV Ch. 8 as CI)
```

Three engineering notes the pyramid needs:

**Probes localize before references diagnose.** The probe worlds (Vol. IV Ch. 7) sit between layers 2 and 4: every exotic mechanism has a minutes-scale world exercising it alone, so a phenomenology failure in the Sundered March starts with green probes and a narrowed suspect list.

**The long layers need a budget, not an excuse.** Century runs are not per-commit material. The reference cadence: layers 1–3 per commit; 4–5 nightly; 6 weekly and per release — with *seed-pinned* runs for comparability and a rotating fuzzed-seed lane hunting rare-seed pathologies. Slow tests that never run are decorations; schedule them or delete them.

**Failures file their own evidence.** A failing soak or signature test automatically attaches: the seed, the manifest, the first anomalous meter tick, and a causal-debugger bookmark into the run's save. The bug report *is* a reproducible world — determinism's whole point, applied to the team's own time.

---

# Designer Note
## The Income Bug, as Liturgy

The founding observability story deserves its retelling in the terms this chapter built.

A year-long soak — layer 5, nightly budget — showed NPC income bleeding to zero. Not a crash (layers 1–3 green), not a conservation breach (layer 4 green: the ledgers balanced perfectly while everyone went broke). A *liveness* anomaly: a meter drifting where its envelope said steady.

The causal query ran backwards from a starving NPC's empty purse: purchases, no wages; schedule shows work; work events chronicle no earnings. The cause was an absence — no rule restored income for labor — and the fix was a rule (`income_per_work_hour`), which is exactly where Volume IV Chapter 2 says world truths live.

Every element of this chapter appears in the story: the meter that saw it, the envelope that flagged it, the chronicle that explained it, the determinism that reproduced it, the rule that fixed it, and the signature suite that now guards it. The engine's instruments are not overhead on the simulation. They are how a simulation this ambitious gets to be *believed*.

---

# 8.4 Common Queries

- What are this world's meters now, versus its reference envelopes?
- Why is this fact what it is? What did this event cause? (causal debugger)
- Where did these twin runs first diverge, and which system's proposal differs?
- What did this world look like at tick N? (time travel)
- What changed in the signature profile between these two engine/package versions?
- Which test layers ran for this release, on which seeds, with what budget?

---

# 8.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Meters are derived, read-only, stream-free, and never persisted as truth.
2. Every domain ships meters beside its systems; liveness envelopes run continuously.
3. Live dashboards, soak reports, and CI signatures consume the same telemetry.
4. Causal chain queries are first-class tooling against live worlds and saves.
5. Any past tick is reachable by restore-and-replay, with instruments attachable retroactively.
6. Counterfactual branches are labeled studies and never merge into source history.
7. Divergence bisection to the first differing proposal is automated.
8. The test pyramid's every layer has an owner, a budget, and a schedule.
9. Every exotic mechanism has a probe world; every reference world has a signature suite.
10. Failing long-run tests attach seed, manifest, meters, and a causal bookmark automatically.

---

# 8.6 Anti-Patterns

### The Perturbing Probe

Instrumentation that consumes streams, forces actualization (Ch. 5's ladder), or nudges timing-dependent anything. If attaching the debugger changes the world, the debugger is a participant.

### Dashboard Divergence

Live meters computed one way, CI signatures another. Two instrumentations disagree eventually, and the argument about which is right consumes the week the bug needed.

### Printf Archaeology

Debugging emergent behavior by adding logs and re-running. The chronicle already recorded everything with causes; a debugging culture that greps instead of querying is paying for the causal debugger and not using it.

### The Heisenberg Fix

"Fixed" bugs that were actually perturbed into hiding by the debugging session — impossible under a stream-free observation plane, reintroduced the day someone violates it. Guard the plane.

### Signature Snooze

Muting phenomenology failures as flaky (Ch. 4's tolerated flake, at world scale). A signature that fires is a world changing character; the response is diagnosis or a deliberate envelope revision (Vol. IV Ch. 7) — never a mute button.

---

# 8.7 Future Evolution

Future versions may add:

- anomaly detection over meter series (learned envelopes beside declared ones)
- visual causal-graph exploration for the debugger
- automatic counterfactual minimization ("smallest perturbation that removes the famine")
- world-diff tooling (structural comparison of two saves' realities)
- always-on flight recorders with ring-buffered fine-grained meters

The instruments grow; the law holds: observe everything, perturb nothing.

---

## Preparing for the Next Chapter

One consumer of the observation plane is unlike the others — it talks back, in prose, and occasionally hallucinates:

**AI Integration** — the narrator, the interpreter, LLM-backed minds, and the hard architectural walls that let stochastic language models serve a deterministic world without ever touching it.

---

# END OF CHAPTER 8
