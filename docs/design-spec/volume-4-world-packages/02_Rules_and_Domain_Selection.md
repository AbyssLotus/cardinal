# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 2
# Rules and Domain Selection

> *A universe is a small number of decisions applied without exception.*

---

# Chapter Overview

Before a package declares a single deer or shilling, it must answer two structural questions:

**Which kinds of reality exist here?** — domain selection.

**What laws does each obey?** — rules.

Together these form the package's *constitution*: the part that changes rarely, constrains everything, and makes one world feel fundamentally unlike another long before content differs. A modern city and a medieval kingdom might share most of their domain list; what separates them is rules — how fast information travels, what medicine can heal, what a state can enforce.

This chapter defines both mechanisms and the discipline they demand.

---

# 2.1 Domain Selection

A package declares which of the engine's domains run:

```text
domains:
  physical:      required by every world
  living:        enabled
  resources:     enabled
  ecology:       enabled
  economy:       disabled
  society:       disabled
  culture:       disabled
  knowledge:     disabled
  conflict:      enabled
  institutions:  disabled
```

That particular selection is a wilderness: things live, eat, fight, and die, and nothing trades, believes, or governs. It is a complete, valid, runnable world — Volume III Chapter 12's removability invariant, exercised deliberately.

## Selection Semantics

Disabling a domain means *no systems propose its fact types*, and its content declarations are rejected at validation. It does not mean the engine pretends the domain never existed: the ownership matrix is universal, so a disabled domain's territory simply lies vacant.

Three consequences follow:

1. **No silent substitution.** A world without Economy has no prices — not implicit ones, not hardcoded ones. If wolves do not trade, nothing does.
2. **Dependencies are declared, not assumed.** Some domains consume others heavily (Ecology without Living Systems is empty bookkeeping). A package's selection must satisfy each enabled domain's declared requirements, checked at validation.
3. **Selection is constitutional.** Enabling a domain mid-save is a migration event (Chapter 6), not a toggle. Worlds may grow new kinds of reality, but only deliberately, with a recorded transition.

## Physical Reality Is Not Optional

One domain is mandatory. Every fact needs somewhere to exist; every entity needs the possibility of position, containment, or region. Even the most abstract package — a pure market simulation — inhabits at least a trivial space. The engine provides no world without a where.

---

# 2.2 Rules

Rules are the package's tunable laws: every number, rate, threshold, curve, and switch that the domains consume.

```text
rules:
  physical:
    day_length_hours: 24
    seasons: [spring, summer, autumn, winter]
    gravity: standard
  living:
    need_decay:
      hunger_per_hour: 0.03
      fatigue_per_hour_active: 0.06
    death:
      permadeath: true
  conflict:
    morale_break_threshold: 0.35
    reaction_dodge_max_projectile_speed_ms: 40
  ecology:
    regrowth_model: logistic
    collapse_floor_fraction: 0.15
```

Volume I made the commitment; this section makes it total: **every tunable number lives here.** An engine binary contains mechanisms and zero magic numbers. If a constant would change the simulation's outcome and a designer might ever wish it different, it is a rule.

## Rules Are Laws, Not Suggestions

A rule applies to everything in its scope, always. `permadeath: true` binds the protagonist exactly as it binds a sparrow — Volume I's "no player protection" directive is enforced *here*, structurally: there is no per-entity exception syntax, so plot armor is not merely discouraged but inexpressible.

Where worlds genuinely need differentiated physics — magic users metabolize differently, undead ignore hunger — the mechanism is *scoped rules over declared categories* (rules keyed to species, modifiers, or regions declared in content), never named-individual exceptions. The law may distinguish kinds. It may not distinguish Steve.

## Exotic Physics Are Ordinary Rules

The fantasy world does not "break" rules; it sets them:

```text
rules:
  physical:
    gravity: none            # a void-city world
  living:
    death:
      permadeath: false
      respawn: {location: region.sanctum, cost: {res.vitality: 10}}
  knowledge:
    techniques:
      resource_pools: [mana]  # spellcasting is technique + pool, per Vol III Ch. 7
```

The lesson, learned in the repo's genre probes (guns, resurrection, mana on an unmodified engine): exotic worlds are rule *values*, not rule *systems*. If a fantasy conceit cannot be expressed as values over existing mechanisms, that is a domain-boundary conversation (Chapter 1's extension seam), not a hack.

## Composition Rules

Volume III Chapter 12 assigned each fact's owner the duty of declaring how competing proposals compose. Packages supply those declarations:

```text
composition:
  physical.structure_integrity:
    order: [decay, damage, repair]
  living.health:
    order: [disease, damage, healing]
    floor: 0
```

Composition is a rule like any other — declared, deterministic, validated, and identical for all proposers.

---

# Designer Note
## The Rulebook Is the World

Two packages with identical content and different rules are different worlds.

Take Thornwall, and set information's travel speed (Culture/Information rules) from horse-speed to instant. Nothing else changes. But reputation now arrives before travelers do, markets arbitrage in a day, rumors stop degrading, distant wars are near, and the entire strategic value of geography collapses. You have not tuned Thornwall; you have built its modernity.

This is why rules deserve constitutional respect and version discipline. Content grows a world. Rules *are* one. The most consequential edits an author will ever make fit on one line, and Chapter 7's reference-world testing exists in large part to catch what those one-line edits do.

---

# 2.3 Common Queries

- Which domains does this package enable, and are their dependencies satisfied?
- What is the value of this rule, in this scope, for this category?
- Which rules consume this content category (species, region, modifier)?
- What composition order governs this fact type?
- How do two package versions differ in rules alone?
- Which rules did this simulation outcome actually consume? (tooling: rule provenance)

---

# 2.4 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Domain selection is explicit, validated, and constitutional.
2. Physical Reality is enabled in every world.
3. A disabled domain's fact types are proposed by nothing and declared by nothing.
4. Domain dependencies are declared and checked, never assumed.
5. Every tunable number is a package rule; engine binaries contain no magic numbers.
6. Rules bind uniformly within their declared scope; no named-individual exceptions exist.
7. Scoped rules key on declared categories only.
8. Composition orders are package-declared, deterministic, and proposer-blind.
9. Rule changes to a live save are migration events with provenance.
10. Exotic physics are rule values over existing mechanisms, never mechanism forks.

---

# 2.5 Anti-Patterns

### The Implicit Domain

Simulating a disabled domain's phenomena through another's rules ("we disabled Economy but shopkeepers use a barter hack in Culture"). Vacant territory stays vacant; if the world needs trade, enable trade.

### Protagonist Clauses

Any syntax that lets a rule name an individual. The day it exists, every design conversation becomes a negotiation about who deserves one.

### Balance by Binary

Fixing a tuning problem in engine code because editing the package felt slow. The number now exists twice, and the package's copy is a lie.

### The Godlike Default

Engine-supplied fallback values for undeclared rules. A missing rule is a validation error, not an invitation for the engine to have opinions. (Explicit package-side defaults, declared in a dependency pack, are fine — they are still package.)

### Rules as Content

Burying laws in content declarations ("this species definition includes the global hunger rate"). Rules and content have different lifecycles and different blast radii; mixing them makes every content edit a potential physics change.

---

# 2.6 Future Evolution

Future versions of Cardinal may introduce:

- rule schemas with declared units and valid ranges
- sensitivity tooling: which rules most affect which phenomena
- rule presets and inheritance across package families
- staged rule changes (a law of physics that shifts mid-history, as a recorded event)
- interactive tuning against reference worlds

Each makes rules easier to wield.

None makes them less binding.

---

## Preparing for the Next Chapter

The constitution is set. Now the world needs nouns.

The next chapter covers **Content** — data packs, identity, cross-reference integrity, modular composition, and the pipeline that turns an author's declarations into validated runtime fact templates.

---

# END OF CHAPTER 2
