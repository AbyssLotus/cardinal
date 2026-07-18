# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 5
# Scenarios

> *A world is a place. A scenario is a place, a moment, and a reason to care.*

---

# Chapter Overview

Generation produces worlds. But no one experiences "a world" — they experience a situation in one: a caravan guard on the Thornwall road in the drought year; a new-made reeve in a town that hates the crown; an observer watching an ocean with no people in it at all.

A scenario is a package's named, versioned declaration of a starting situation:

```text
scenario.lean_years
  world: generated per generation/, seed-range vetted
  start: year 203, early spring       (the second dry year)
  perspective: authored person, Wren of Milbrook, caravan guard
  situation: [standing facts and pressures — see §5.2]
  observations: [named conditions worth watching — see §5.4]
```

Scenarios are the thinnest layer of the package, and the discipline of this chapter is keeping them thin. A scenario *selects and arranges*; it never adds mechanisms, exceptions, or protections. Everything a scenario does must be expressible as ordinary facts in an ordinary world — because that is all it is permitted to produce.

---

# 5.1 Scenario and World

The separation earns its keep three ways:

**One world, many scenarios.** Thornwall generates once; *The Lean Years*, *The Succession*, and *A Quiet Life* all start inside it — different years, different perspectives, different pressures, same universe. Authoring a scenario costs days; a world costs months.

**Scenarios are reproducible.** A scenario names its generation inputs (package version, generation config, seed or vetted seed-range). Starting it twice produces the same situation — Volume II's determinism, extended to "the beginning."

**Scenarios are honest.** Because a scenario can only arrange facts the world validates, it cannot promise what the world cannot deliver. A scenario claiming a besieged city must arrange a real siege — real army, real supply state, real hostilities — or fail validation.

---

# 5.2 The Situation

A scenario's substance is its opening arrangement, applied as the final generation layer (it is, in effect, a privileged consumer of Chapter 4's layer 8–9 machinery, and its privilege expires identically at first commit):

**Clock.** The start date, season, and time — chosen for pressure. *The Lean Years* starts in early spring with granaries at winter's floor.

**Standing facts.** The situation's load-bearing arrangements, every one an ordinary fact with provenance: the drought's second year (environmental state trending), Milbrook's grain stores at 0.2, the toll war simmering (a Conflict hostility), the reeve's office vacant (an Institutions state).

**Cast.** Authored persons the situation needs — named, related, positioned, and *ordinary*. Wren has the facts any person has: kin, holdings, proficiencies, reputation, memories seeded from the generated history she is stitched into.

**In-progress processes.** Situations begin mid-motion: a caravan two days out, a quest of the old spec's kind already posted, an army already marching. Anything the simulation can carry forward, a scenario may start in flight.

## Stitching

Authored cast and facts must *join* the generated world, not float on it: Wren's mother is a generated person; her grudge references a chronicled event; her employer's caravan carries real goods on a real route. Validation enforces the stitching — an authored fact referencing nothing is the painted-backdrop anti-pattern at scenario scale.

---

# 5.3 Perspective Without Privilege

A scenario may designate a perspective: the entity (or entities, or none) through which narration and play are offered.

The whole of Cardinal's law on perspective fits in one sentence:

**Perspective is an information-and-narration binding, never a simulation binding.**

The perspective entity gets a bound observer pipeline (Volume II, Chapter 4) and the narrator's attention (Volume I). It gets nothing else. Wren obeys permadeath if the world's rules say so, starves by the same curves, and is unknown to every system — the engine cannot distinguish her from any other organism, because no fact marks her as special.

Consequences, accepted deliberately:

- The perspective can die. The scenario declares what happens next (end, or rebind elsewhere) — the *world* does not flinch either way.
- The world does not orbit the perspective. The succession crisis resolves with or without Wren's involvement. Distance from the action is real distance.
- An unbound scenario is valid. The ocean world's scenario has no perspective at all: it is a world offered to observation. So is a historian's scenario opening a century-old save.

## Endings Are Observations

Scenarios may declare *observations*: named conditions over committed facts, watched by tooling and narration.

```text
observation.milbrook_starves:   settlement grain < survival threshold by midsummer
observation.roads_reopen:       toll war terms signed
```

Observations can drive framing — an epilogue when one fires, a scenario "completed" marker, a study's data point. They can drive nothing else. No observation may steer, protect, spawn, or nudge: a "win condition" in Cardinal is a fact pattern someone chose to watch for, exactly as a famine is (Volume III, Chapter 11). The scenario author who wants Milbrook savable must arrange a savable situation, not a saving mechanism.

---

# Designer Note
## The Adventure That Is Actually There

The deepest scenario-authoring habit to unlearn is the *promised* adventure: content that fires when the protagonist arrives, quests instantiated at need, villains waiting in suspended animation.

Cardinal scenarios promise nothing they have not already made true. The bandit camp exists now, eating now, raiding on its own schedule whether Wren ever rides north. The vacant reeve's office is decaying *now* — and if Wren dawdles a season, someone else holds it, because Institutions kept simulating.

This costs the author control, and buys the one thing scripted worlds cannot fake: the situation is *load-bearing*. Every rumor Wren hears traces to real events; every opportunity is real and therefore genuinely missable; every "quest" is a standing situation that would have resolved some other way in her absence — and the chronicle can prove it.

Author situations, not stories. The stories are what happens next.

---

# 5.4 Common Queries

- What scenarios does this package offer, against which generation configs?
- What does this scenario arrange — clock, standing facts, cast, in-flight processes?
- Is every authored element stitched into generated reality?
- What perspective, if any, does it bind, and what happens on perspective death?
- Which observations does it declare, and what facts do they watch?
- Do two runs of this scenario from the same seed produce identical starts?

---

# 5.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. A scenario produces only ordinary facts in a validated world.
2. Scenario privilege expires at first commit, identically to generation's.
3. Scenarios are deterministic given package, generation config, and seed.
4. Authored cast and facts must stitch — every reference resolves into the generated world.
5. Perspective binds information and narration only; no simulation fact marks it.
6. Perspective death is a world event like any other; the scenario declares only the binding's fate.
7. Scenarios without perspective are first-class.
8. Observations read committed facts and cause nothing in the world.
9. Win, loss, and completion exist only as observations.
10. No scenario may introduce mechanisms, exceptions, or rules — Chapter 2 owns law.

---

# 5.6 Anti-Patterns

### The Spawned Adventure

Content instantiated by protagonist proximity. If it does not exist while unobserved, it is not in this engine's world.

### Plot Armor by Scenario

Any arrangement whose purpose is protecting the perspective — the enemy that pulls punches, the fatal fall that cannot occur. Volume I's directive has no scenario-shaped exception.

### The Puppeteered Cast

Authored persons with scripted futures ("the duke will betray the crown in autumn"). Author the duke's grievances, debts, and disposition — then let him decide. If the betrayal must happen for the scenario to work, the scenario is a story wearing a simulation's clothes.

### Observation Creep

Observations that acquire effects — first an epilogue, then a reward, then a spawned consequence. The read-only line is bright, and everything interesting about Cardinal is on the other side of it.

### The Floating Start

Casts and facts unconnected to the generated world — the hero from nowhere, the McGuffin without provenance. Stitch or fail.

---

# 5.7 Future Evolution

Future versions of Cardinal may introduce:

- scenario composition (situations layered on situations)
- perspective handoff chains (generational play; the historian's roving eye)
- observation libraries for studies and challenges
- scenario capture — freezing any live moment of any save as a new scenario
- guided-briefing tooling that renders the situation's causal context to the player

Each multiplies what a moment in a world can be for.

None touches what a scenario is: an arrangement of true things.

---

## Preparing for the Next Chapter

A world, mid-story, with someone to care about it — and now years of committed history accumulating behind it.

The next chapter is about keeping all of it: **Saves, Versioning, and Migration** — the formats that persist a world, the versions that describe it, and the discipline that carries a decade-old save across engine and package generations alive.

---

# END OF CHAPTER 5
