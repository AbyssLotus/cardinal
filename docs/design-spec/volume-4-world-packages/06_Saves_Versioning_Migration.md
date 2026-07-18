# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 6
# Saves, Versioning, and Migration

> *A save is not a file. It is a world trusting you with its future.*

---

# Chapter Overview

Volume II, Chapter 5 established persistence's contract: committed reality, preserved completely, reloadable deterministically.

This chapter finishes the story that chapter deliberately left open — because a persisted world does not live alone. It lives *against* a package and an engine, and all three evolve at different speeds. The engine gains features monthly. The package balances rules and grows content yearly. The save just grows — five simulated centuries, a chronicle of millions of events, families whose founding marriages predate three engine versions.

The questions of this chapter are the questions of that triangle:

What exactly does a save contain, and what does it reference? What do the three version numbers mean, and who may move independently of whom? And when movement breaks compatibility — as it eventually must — what does an honest migration owe the world it operates on?

The stakes are stated in Volume II and repeated here as this chapter's north star: **worlds outlive engine versions.** A Cardinal save from the project's first decade should open in its third.

---

# 6.1 What a Save Is

A save is three things and their binding:

```text
Save
├── Reality      — the complete committed fact state at a tick (snapshot),
│                  per Vol. II Ch. 5's persistence contract: identities, facts,
│                  relationships, clock, RNG stream states
├── History      — the event chronicle from generation forward, append-only
├── Provenance   — what made this world: package id + exact version,
│                  scenario, seed, generation config, engine version,
│                  save-format version, and the save's own migration record
└── (reference)  — the sealed package it runs against — referenced, not copied
```

Two boundaries define the format's character:

**The save persists state, never definition.** No species templates, no rules, no recipes are stored in the save — those live in the sealed package the provenance references. A deer in the save is an entity with facts; *what a deer is* remains the package's declaration. This is Volume II's "behavior is regenerated" clause at package scale, and it is what keeps saves migratable at all: definition has one home.

**The save persists reality, never presentation.** No caches, no derived statistics, no narration, no observer conveniences. Everything rebuildable is rebuilt on load (Volume II's invariant), because everything stored is something migration must someday carry.

## The Chronicle Is Load-Bearing

A note easy to miss: the event history is not a luxury log. Volume III made grievances, reputation, lore, and legitimacy *trace to events*; Chapter 11 made emergence auditable through them. A save that trimmed its chronicle to save space would leave standing facts pointing at absent causes — referential corruption, not compression. Chronicle *compaction* (era summaries with retained referents) is legitimate implementation craft; chronicle amputation is data loss.

---

# 6.2 The Version Triangle

Three versioned artifacts, three compatibility relationships:

```text
        Engine  e.g. 0.4.2
       /      \
save-format    package-API
     /            \
  Save ———————— Package   e.g. thornwall 2.4.0
        provenance
```

**Save ↔ Engine: the save-format version.** The engine's persistence layer declares which save formats it reads. Newer engines read older formats through migration (§6.3). Older engines refuse newer formats — cleanly, by declared version check, with a message naming the versions (never by crashing into unexpected bytes).

**Package ↔ Engine: the declared range.** The manifest's `engine: ">=0.3, <0.5"` from Chapter 1, enforced at load. An engine outside the range refuses the package; the package's statement is authoritative even when it is conservative.

**Save ↔ Package: the exact pin.** The save's provenance pins the *exact* sealed package version that produced its facts. This is the triangle's strictest edge, because package changes reach into live state (§6.4). A save opens against its pinned version by default; opening against any other version is a migration, never a substitution.

The discipline across all three edges is identical: version checks are explicit, refusals are early and articulate, and *nothing silently proceeds across a version boundary.*

---

# 6.3 Save-Format Migration

The mechanically simple case: the engine's representation of facts changed shape.

Format migrations are:

**Explicit.** A migration is a named, versioned transformation `format N → N+1`, shipped with the engine, run deliberately — never an implicit fixup during load.

**Sequential and append-only.** A format-1 save reaching a format-4 engine runs 1→2→3→4 in order. Each migration adds and transforms; none discards simulation-meaningful information. (The repo's own M6 milestone — `save_format` in metadata, append-only migrations in a transaction, refusal of newer saves — is this section already alive in miniature.)

**Transactional and recorded.** A migration completes entirely or leaves the original untouched; a completed migration appends itself to the save's migration record — the save remembers its own surgeries.

**Semantically inert.** The world before and after a format migration is *the same world*: same facts, same determinism from here forward. Format migration moves representation, never meaning. Any change that would alter simulation outcomes is not a format migration, whatever it calls itself.

---

# 6.4 Package Migration

The hard case, and the honest heart of this chapter: the *definition* of the world changed under a live world.

Package changes divide by blast radius:

**Compatible (patch/minor):** additions and inert fixes. New species not yet placed, new recipes, new scenarios, vocabulary and description corrections, new content categories. Existing saves reference nothing that changed; the pin may advance after validation confirms no existing fact's meaning moved.

**Breaking (major):** anything that reaches into standing facts. A retuned hunger curve (every organism's trajectory shifts), a removed species (live deer reference it), a rebalanced recipe (holdings and prices were built on the old one), an altered composition order (Chapter 2 — the same proposals now commit differently).

Breaking changes demand a *package migration*: a package-authored, versioned transformation over the save that reconciles standing state with new definition. And package migrations owe the world two debts that format migrations never incur:

**The debt of causes.** A migration that changes facts is an event in the world's history and must be chronicled as one — provenance: package upgrade, version, tick. The alternative is facts that changed for no recorded reason, which Volume II forbids and Chapter 11's auditability cannot survive.

**The debt of continuity.** Identity persists (Volume II): the retired species' living members are migrated to a successor or to a chronicled end — never vanished. The renamed id is tombstoned with a forward reference (Chapter 1). The rebalanced economy keeps its holdings; the *rules* moved, and the world will re-equilibrate by simulation, not by fiat adjustment of anyone's wealth.

A breaking change without a shipped migration leaves existing saves *permanently pinned* — valid forever against their sealed version. That is the fallback and it is honorable: abandonment is declared, never accidental.

---

# Designer Note
## The Decade-Old Save

Design every format and migration decision against one scenario:

A save begins under engine 0.3, Thornwall 1.0. Over ten real years it crosses six format migrations and four package majors. Its chronicle holds two simulated centuries. Its oldest living entity predates every current line of engine code.

Now its keeper asks the questions the architecture must answer *yes* to: Does it open? Does it replay deterministically from any era's snapshot *using that era's pinned versions*? Can the historian trace a standing grievance to a chronicled cause across the 2.0 migration? Does the migration record explain every surgery?

This scenario is why definition lives outside the save, why migrations are chronicled events, why pins are exact, and why refusal is always explicit. Every shortcut this chapter forbids is a shortcut that quietly kills the decade-old save — and with it, the promise that makes persistent worlds worth building.

---

# 6.5 Common Queries

- What package version, scenario, seed, and engine produced this save?
- What migrations — format and package — has it undergone, and when?
- Can this engine open this save, and if not, exactly why?
- What would upgrading this save to package 3.0 change, before committing to it?
- Which standing facts were touched by the last package migration, under what provenance?
- Does this save replay deterministically against its pins?

---

# 6.6 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Saves persist state and provenance, never definition or derivation.
2. The chronicle is part of the save and is never amputated.
3. Provenance pins the exact sealed package version.
4. All version checks are explicit; no load silently crosses a version boundary.
5. Format migrations are sequential, transactional, recorded, and semantically inert.
6. Package migrations are package-authored, versioned, and chronicled as world events.
7. Migration never deletes identity; it transforms or chronicles an ending.
8. A save whose pins are honored replays deterministically, forever.
9. Breaking package changes without migrations leave saves pinned, by declaration.
10. The original save survives any failed migration untouched.

---

# 6.7 Anti-Patterns

### Definition Snapshotting

Copying rules and templates into the save "for safety." Now definition has two homes, migrations must chase both, and the pin is decorative.

### The Silent Fixup

Load-time repairs that patch inconsistencies without record. Every silent fixup is an unchronicled event — a small lie the historian inherits.

### Fiat Reconciliation

Package migrations that "fix" the world's state to match new balance intentions — resetting prices, topping up populations. Migration reconciles *references and meaning*; re-equilibration is the simulation's job. A migration that plays god is the Balancing Hand (Vol. III, Ch. 11) with a version number.

### Version Optimism

Attempting loads across undeclared version gaps because "it'll probably work." The cheapest corruption to prevent is the one refused at the door.

### The Trimmed Chronicle

Deleting old events for space. Compact with referents intact, or keep it all; standing facts must never point at nothing.

### Migration by Regeneration

"Just regenerate the world under the new version." That is not migration but replacement — a different world wearing the old one's name, with ten years of history discarded. Chapter 4's generation privilege does not resurrect for convenience.

---

# 6.8 Future Evolution

Future versions of Cardinal may introduce:

- incremental and differential snapshots
- era-tiered chronicle storage with uniform query
- migration dry-runs with full impact reports
- branching saves (one history, many futures) with shared ancestry storage
- archival formats designed for decade-scale dormancy

Each serves the same keeper of the same decade-old save.

---

## Preparing for the Next Chapter

Everything now exists to be wrong at scale: packages, generation, scenarios, migrations.

The next chapter is the discipline that catches it: **Validation and Testing** — the layers that refuse invalid worlds before they run, and the tests that prove a world is not just valid but *alive*.

---

# END OF CHAPTER 6
