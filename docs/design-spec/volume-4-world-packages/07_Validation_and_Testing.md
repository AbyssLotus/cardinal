# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 7
# Validation and Testing

> *An invalid world must never get the chance to be interesting.*

---

# Chapter Overview

Every chapter of this volume has been writing checks against this one.

Content promised no dangling references. Rules promised no missing values. Generation promised output indistinguishable from eternal reality. Scenarios promised stitched casts. Migration promised meaning preserved across versions.

This chapter is where the promises are enforced — and, just as importantly, where enforcement *stops* and judgment begins. Cardinal distinguishes two activities that most pipelines blur:

**Validation** answers: *may this world exist?* It is binary, mechanical, and merciless. A package or save either satisfies every declared invariant or is refused before simulation begins.

**Testing** answers: *is this world any good?* It is statistical, empirical, and advisory. A valid world can still be dead — wolves that never hunt, markets that never move, wars that never end. Testing detects it.

Validation guards the gate. Testing walks the grounds.

---

# 7.1 The Validation Stack

Validation runs in ordered layers, each assuming the last. A failure names its layer, its subject, and its rule — refusal is a diagnostic, never a shrug.

## Layer 1 — Schema

Every declaration parses against its category's schema: fields present, types correct, units and ranges respected where declared. Rules exist for every value the enabled domains consume (Chapter 2's no-godlike-defaults invariant is enforced here: a missing rule fails, loudly, by name).

## Layer 2 — Reference

The composed content graph resolves totally: every id referenced exists, across all packs, after all composition; dependency versions are satisfiable; no id is declared twice; tombstones are honored. The repo's registry — load, validate, cross-reference-check — is this layer's living ancestor.

## Layer 3 — Coherence

The declarations make joint sense against the architecture's own law:

- domain selection satisfies domain dependencies (Chapter 2)
- every fact template lives in its owner's category (Vol. III, Appendix A — the ownership matrix is *executable* here)
- composition orders cover every multi-proposer fact type
- food webs reference only declared species and resources; recipes only obtainable inputs
- conservation is possible: nothing declares matter, money, or population from nowhere

## Layer 4 — World

Generated output (and every loaded or migrated save) satisfies running-world invariants: referential integrity of live facts, identity uniqueness, clock and RNG stream state validity, chronicle causes present for standing facts that require them, package pin honored. Volume II Chapter 5's "validation occurs before activation" — with this volume's additions folded in.

## Layer 5 — Scenario

Scenario arrangements stitch (every authored reference resolves into generated reality), perspectives bind to real entities, observations reference watchable facts, and in-flight processes are states the simulation can actually carry forward.

One rule spans all five layers: **validation is read-only.** A validator that fixes what it finds is an unchronicled writer — the Silent Fixup of Chapter 6, wearing a badge.

---

# 7.2 Testing Worlds

A valid world enters testing: headless, seeded, accelerated runs — the repo's playerless-days pattern grown into method. Four families, in rising order of subtlety:

## Determinism Tests

The same seed run twice is byte-identical; a save-load-continue splices into an unbroken run undetectably; replays reconstruct exactly (golden-transcript tests — the repo's byte-identical combat is the pattern). These test the *engine through the package*: a package cannot cause nondeterminism, but it can expose an engine seam that does.

## Invariant Soaks

Long runs with conservation meters attached: currency reconciles against every recorded source and sink; matter and population counts trace to accounted causes; no fact changes without provenance. (The repo's col-conservation and no-level-scaling tests are founding members.) Soak length matters — a slow leak needs a year to show, which is precisely why the 365-day run exists.

## Liveness Tests

The world exhibits *activity* within expected envelopes: NPCs move, eat, and remember; quests resolve or expire with or without a player; populations neither flatline nor explode; markets clear; the chronicle grows non-trivially. Liveness catches the valid-but-dead world — every check green, nothing happening.

## Phenomenological Tests

Volume III Chapter 11's regression regime, operationalized: reference worlds with expected *emergence signatures*. A vetted seed of Thornwall, run a century, should show stable trade corridors, settlement growth and abandonment, price gradients tracking geography, at least one war and one peace. Signatures are statistical envelopes, not scripts — the war need not be *that* war, but a century without any is a red flag with a chapter of theory behind it.

Phenomenological tests are the deepest safety net Cardinal has: they catch the change that broke nothing and deadened everything.

---

# Designer Note
## The Test That Failed Into a Discovery

The most valuable test failures in a causes-not-outcomes engine are the ones that turn out to be *findings*.

The repo's own history holds the founding example: a year-long telemetry soak caught NPC income bleeding to zero — not a crash, not an invariant breach, a *liveness* anomaly. The cause was real and architectural: no activity restored income; working needed to earn. The fix was a rule, and the world was truer for it.

This is the posture testing should institutionalize. A phenomenological failure means one of two things: the change dishonored a cause (fix the code), or the world's causes genuinely produce a surprise (update the envelope, and study it — Chapter 11 says surprising outcomes with correct causal chains *stand*). Both outcomes are wins. The only loss is an envelope so loose it never fires, or a team that reflexively "fixes" every surprise back to expectation — the Balancing Hand, reinvented as QA.

---

# 7.3 Package-Declared Expectations

Testing infrastructure is engine-side; *expectations* are package-side. A package ships its own:

```text
validation/
├── invariants/      — package-specific conservation and coherence rules
│                      ("vitality is conserved across respawns, minus the tithe")
├── reference/       — vetted seeds + expected signatures at declared horizons
│                      (30-day liveness, 365-day stability, century emergence)
└── probes/          — minimal worlds exercising this package's exotica in isolation
```

The probe pattern deserves its note: the repo's cyberpunk, Destiny, and WoW probes — tiny packages proving guns, resurrection, and mana on an unmodified engine — generalize into method. Every exotic mechanism a package leans on (Chapter 2's rule-values-not-rule-systems) merits a probe world small enough to diagnose in minutes, so failures localize before Thornwall's full weight is in play.

Reference expectations ride the package through Chapter 6's version discipline: a package major that retunes the economy *must* update its signatures, and a signature diff is a reviewable artifact — the world's intended character, under version control.

---

# 7.4 Common Queries

- Is this package valid, and if not, which layer, which subject, which rule?
- Which validation layers does this save pass right now?
- Do this package's reference seeds meet their signatures at every horizon?
- What did the last soak's conservation meters read?
- Which probe covers this mechanism, and is it green?
- How did this change alter the reference world's signature profile?

---

# 7.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. No simulation begins against an unvalidated package, world, or save.
2. Validation layers are ordered; each failure names layer, subject, and rule.
3. Validation is read-only, without exception.
4. The ownership matrix and composition coverage are mechanically enforced (Layer 3).
5. Missing rules are failures, never defaults.
6. Determinism tests gate every engine and package release.
7. Conservation soaks reconcile every declared conserved quantity to accounted causes.
8. Liveness envelopes are defined per package and tested headless.
9. Reference worlds with emergence signatures are versioned package artifacts.
10. A surprising result with a correct causal chain updates envelopes, never the world.

---

# 7.6 Anti-Patterns

### The Fixing Validator

"Repairs" applied during validation. Read-only means read-only; a fixer is a writer without a chronicle.

### Validation by Playtest

Shipping packages whose only verification is someone played it a while. Human attention samples one path; soaks sample years. Both, always — but the machine goes first.

### The Green Dead World

A test suite of pure invariants, no liveness. Everything conserves in a world where nothing happens; stillness passes every ledger check.

### Envelope Rot

Reference signatures never updated as the package's character legitimately evolves — until the suite fails so constantly it gets ignored or deleted. Signatures are living artifacts with owners and review.

### Probe Neglect

Exotic mechanisms tested only inside the full world. When the mana economy breaks inside Thornwall's fully loaded century run, the diagnosis costs a week; the probe would have cost a minute.

### Teaching to the Test

Tuning rules until the reference signature passes, rather than until the causes are right. The signature is a thermometer; healing the thermometer is the oldest quackery there is.

---

# 7.7 Future Evolution

Future versions of Cardinal may introduce:

- signature mining — learning a world's statistical fingerprint automatically
- differential soak reports between package versions
- coverage analysis: which rules and content a test run actually exercised
- fuzzed-seed campaigns hunting rare-seed pathologies
- continuous century-runs as package CI

Each sharpens the same two questions:

May this world exist? Is it alive?

---

## Preparing for the Final Chapter

The machinery of this volume is complete: definition, law, content, birth, situation, preservation, and proof.

One demonstration remains. The final chapter builds **Five Worlds** — a medieval kingdom, a modern city, an ocean ecosystem, a space colony, and a fantasy world — on the unmodified engine, to show that the architecture holds where it matters: everywhere.

---

# END OF CHAPTER 7
