# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 10
# Extension and Evolution

> *An architecture succeeds when its authors become unnecessary and its laws do not.*

---

# Chapter Overview

Every long-lived engine faces the same fate: the people change, the hardware changes, the ambitions change — and the codebase either evolves under law or decays under cleverness.

This closing chapter is about aging well. It specifies the extension mechanisms (how new capability enters without forking), the performance philosophy (how speed is pursued without corruption), and the evolution discipline (how the specification itself changes). It ends where the roadmap asked it to: with the future, held properly.

---

# 10.1 Plugin Architecture

Cardinal needs no plugin system *invented* — the architecture's existing seams are the plugin system. This section names them and closes the gaps.

## The Five Seams

```text
1. World packages     — new worlds, rules, content        (Vol. IV; data only)
2. Domains            — new kinds of reality               (Vol. III Ch. 12's
                        checklist; registered at bootstrap, Ch. 1)
3. Stores             — alternate reality-store backends   (Ch. 2's contract +
                        conformance suite)
4. Services           — new observers and operators        (Ch. 6's plane;
                        read-only by construction)
5. Frontends          — new consumers and input sources    (entitled streams in,
                        validated actions in the front door)
```

Each seam has a contract already specified in this volume; *plugin* is just the word for third-party code at a seam. The gaps to close are packaging and trust:

**Manifests and capabilities.** A plugin declares its seam, its versions (engine range, like packages — Vol. IV Ch. 1), and its capabilities. A service plugin declaring observation-only can be granted exactly that; a frontend plugin gets entitled streams and the input door, nothing else. Capability is enforced at the seam boundary, not promised in documentation.

**Trust tiers by seam.** Seams 1 is inert data (validated, sandboxed by nature — the strongest tier). Seams 4–5 touch only read-only planes and the validated input door (contained by construction). Seams 2–3 are *engine-privileged* — a domain or store runs inside the tick — and third-party code there is held to the engine's own bar: conformance suites, determinism gates (Ch. 4), review. There is no sandbox that makes a nondeterministic store acceptable; privileged seams are earned, not opened.

**No sixth seam.** The recurring request will be hooks *inside* the tick — "just let my plugin peek at proposals." The answer is the whole specification: extension happens at the seams because the middle is where the law lives. A capability the seams cannot express is a specification proposal (§10.3), not a hook.

---

# 10.2 Performance Philosophy

Volume III Chapter 1 said it first: *performance emerges from architecture, not shortcuts.* This section makes it operational, as four rules and a budget.

**Rule 1 — Correctness is not negotiable currency.** No optimization may spend determinism, conservation, provenance, or the single write path to buy speed. The forbidden trades are enumerable and enumerated (the anti-pattern lists of ten chapters); everything else is open season.

**Rule 2 — Optimize questions, not loops.** The read-mostly principle: Cardinal's cost centers are queries (spatial, relational, causal) and the tick's evaluation sweep. The architecture already concentrated the hot paths — store tiers (Ch. 2), scheduler hints (Ch. 3), residency ladders (Ch. 5) — and each is *invisible by contract*, which is what makes aggressive optimization safe there.

**Rule 3 — Measure against reference worlds.** Performance claims are meter deltas (Ch. 8) on the five worlds (Vol. IV Ch. 8) at pinned seeds: ticks/second at Thornwall's century, Meridian's content scale, Kepler's conservation density. A speedup that cannot name its reference-world delta is an anecdote; one that moves a signature (Vol. IV Ch. 7) is a bug wearing a stopwatch.

**Rule 4 — Budgets before heroics.** Each layer carries a declared budget (tick-stage timings, memory by tier and region, wire bytes per client) with meters watching. Optimization effort goes where budgets are breached, in order of breach — not where optimization is fun. The budget table is a living artifact beside the signature suites.

And the philosophy's summary, worth a frame: **the slow correct engine can always be made fast; the fast incorrect engine has nothing worth accelerating.** Every profile in this volume — the tiered store, the serialized commit, the aggregate ladder — exists so that speed has somewhere lawful to come from.

---

# 10.3 Evolving the Specification

The five volumes are themselves an artifact under version control, and they change under discipline:

**Precedent first.** Boundary questions argue from Appendix A's rulings; architectural questions argue from the invariant lists. Most "new" questions are old rulings wearing new nouns, and the argument-from-precedent habit is what keeps eleven hundred pages coherent under many hands.

**Amendments are explicit.** A change to an invariant, a ruling, or a contract is a specification amendment: written, rationaled, versioned in the documents *before* implementation follows (Vol. III Ch. 12's boundary-by-accident prohibition, applied to the spec itself). The specification never discovers its own contents from the codebase.

**Implementations carry profiles; the spec carries law.** Where this volume offered choices (store families, float levels, execution profiles), implementations declare which they hold — and conformance suites certify it. New profiles join by demonstrating equivalence, not by argument.

**The five worlds are the constitution's court.** Any proposed change — engine, spec, or plugin — faces them: all five green, or the change explains itself (Vol. IV Ch. 8's standing falsification). A specification with an executable test of its own claims is the rarest kind; keep it that way.

---

# Designer Note
## The Engine That Outlives Its Authors

Run the thought experiment the roadmap's deliverable implies: a decade from now, a team that never met this specification's authors — or a Claude session with no memory of these conversations — sits down to build Cardinal from these five volumes alone.

What must be true for them to succeed? They need the *why* (Volume I), the *law* (Volume II), the *map of what exists* (Volume III and its matrix), the *shape of a world* (Volume IV), and the *tradeoffs of construction* (this volume) — each stated without dependence on code that may no longer exist, each testable against reference worlds they can regenerate from seeds, each amendable through a procedure that survives its amenders.

That is the standard these volumes were written to. Not documentation of an implementation — implementations are weather — but a constitution for every implementation, with its own court and its own case law. The current engine was this specification's first reader and its richest source of scars; the next engine will be its first pure product; and the specification's success will be measured, always, the same way:

The worlds go on. The chronicle stays true. And the village cannot tell.

---

# 10.4 The Roadmap Forward

Held loosely, ordered by dependency rather than date:

**Near** — the specification's debts to the present:
- close the current engine's gaps against Volume III (factions→Institutions as specified; hostility gating behavior per Conflict Ch. 8)
- stand up the conformance suites (store contract, execution-profile equivalence)
- per-tick state hashing and twin-run CI (Ch. 4's cheapest alarm, first)

**Middle** — the reference architecture realized:
- the hybrid store (Ch. 2) behind the contract, migrated invisibly
- full propose/resolve/commit kernel with published ordering (Ch. 3)
- the causal debugger and time-travel tooling (Ch. 8) as first-class products
- the five worlds built out as living packages with signature suites

**Far** — the promises that need the middle first:
- parallel evaluation, then region sharding (Ch. 5, behind equivalence proofs)
- server-authoritative multiplayer on the lockstep core
- distributed simulation and the read-replica historian's archive
- recorded-mind libraries and the mature AI seat (Ch. 9)

Each item lands under the same two gates it would face at any scale: the invariants hold, and the five worlds stay green.

---

# 10.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Extension occurs only at the five seams; no mechanism exposes the tick's interior.
2. Plugins declare seam, versions, and capabilities; enforcement is at the boundary.
3. Privileged seams (domains, stores) meet the engine's own conformance and determinism gates.
4. No optimization trades correctness properties for speed, ever.
5. Performance claims are meter deltas on pinned reference worlds.
6. Layer budgets are declared, metered, and drive optimization priority.
7. Specification changes precede implementation changes, with written rationale.
8. Implementation choices are declared profiles, certified by conformance suites.
9. The five reference worlds gate every engine, specification, and plugin change.
10. The specification remains sufficient, alone, to build the engine again.

---

# 10.6 Anti-Patterns

### The Hook Bazaar

Extension points sprinkled into the tick "for flexibility." Each hook is a seam the law does not cover, and the bazaar's currency is the architecture itself.

### Benchmark Theater

Optimizing synthetic microbenchmarks while reference-world ticks/second stagnates. The five worlds are the benchmark; everything else is rehearsal.

### Spec Drift

The codebase quietly diverging from the volumes until the volumes describe a fiction. The amendment procedure exists precisely so divergence is either corrected or *ratified* — never ambient.

### The Sacred Implementation

Refusing better designs because the current code works. This volume's own instruction cuts the other way: existing code is evidence, never precedent. Profiles retire; the law is what persists.

### Roadmap Worship

Treating §10.4's ordering as commitments with dates. The roadmap is dependency structure; the moment it becomes a schedule, correctness starts negotiating with calendars — and Rule 1 already settled who wins.

---

# 10.7 Closing the Specification

Five volumes, one argument:

A world can be true (I). Truth can be computed (II). The computed truth has anatomy (III). Anatomies can be authored (IV). And the whole of it can be built, scaled, watched, and evolved without ever once cheating (V).

The deliverable the roadmap named is hereby delivered: an experienced engineer — or a future Claude session — holding these volumes can implement Cardinal from first principles, preserving the philosophy of Volume I through every line.

What remains is not specification.

What remains is worlds.

---

# END OF CHAPTER 10
# END OF VOLUME V
# END OF THE CARDINAL ARCHITECTURE SPECIFICATION
