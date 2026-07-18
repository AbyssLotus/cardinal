# Cardinal Architecture Specification
# Volume IV
# World Packages
## Chapter 3
# Content

> *Content does not put things in the world. It declares what kinds of things the world can hold.*

---

# Chapter Overview

Rules said how Thornwall's universe behaves.

Content says what can exist in it: red deer and limestone, arming swords and drystone walls, the Marchfolk and their guest-right, the King's Road toll and the technique of smelting.

This chapter defines the content layer of a world package: what a content declaration is, how declarations reference each other, how packs compose into larger wholes, and the pipeline that carries an author's files into validated runtime form.

One distinction governs everything here, and it is worth stating before anything else:

**Content is definition, not existence.**

`species.red_deer` declares what a red deer *is* — its rates, its diet, its senses. It places no deer anywhere. Existence is generation's job (Chapter 4) and the simulation's thereafter. Content is the dictionary; the world is the text.

---

# 3.1 Data Packs

Content ships in *data packs*: named, versioned sets of declarations grouped by authorship and purpose.

```text
content/
├── core/            — the package's own base declarations
├── highlands/       — a regional expansion: new species, cultures, sites
└── (dependency) pack.temperate_wildlife v1.2
                     — a shared pack this package imports
```

A pack contains declarations across any content categories:

```text
Categories (per owning domain, Vol. III):
  materials, items, structures, devices     → Physical Reality
  species                                    → Living Systems / Ecology
  resources                                  → Resources
  goods, recipes, currencies                 → Economy
  cultures, languages, norm sets             → Culture
  techniques, records                        → Knowledge
  institution charters, law codes, offices   → Institutions
  region templates, settlement templates     → generation inputs (Ch. 4)
```

Every declaration is a *fact template*: the set of initial facts and rates an instance of this kind receives at creation, expressed in the owning domain's schema. The engine's loader knows category schemas; it has never heard of deer.

## Declarations Reference by Id

Content is a web, not a list:

```text
species.red_deer
  diet: [res.browse, res.grasses]
  predators-of-note: —            # not declared here; the food web is
                                  # declared once, ecology-side, not per species

recipe.venison_stew
  consumes: [item.venison, item.root_veg, res.freshwater]
  requires: tech.cooking >= 0.2

culture.marchfolk
  language: lang.march_tongue
  norms: [norm.guest_right, norm.grave_tree_taboo]
```

Every reference resolves by id, across packs, at validation. A dangling reference is a build failure, not a runtime surprise. And each fact is declared *once, in its owner's territory* — the food web edge deer→wolf lives in ecology content, not duplicated inside both species. The ownership matrix (Volume III, Appendix A) governs authors exactly as it governs systems.

---

# 3.2 Modularity

Packs compose. The mechanisms are three, and deliberately few:

## Dependency

A pack declares the packs it builds on, with versions:

```text
pack.thornwall_core
  requires: pack.temperate_wildlife >= 1.2
```

Dependencies form an acyclic graph, resolved before validation. The wildlife pack's `species.red_deer` is now referenceable by Thornwall's recipes and region templates — shared content, authored once, used by many worlds.

## Extension

A pack may add to another pack's declarations where the category schema marks the field extensible:

```text
# highlands pack extends the core culture:
extend culture.marchfolk
  norms: +[norm.highland_hospitality]
```

Extension is additive only. It cannot remove or alter what it extends — the core pack's declaration remains true everywhere the highlands pack is absent.

## Override

A root package — and only a root package — may override values in its dependencies:

```text
override species.wolf
  rates.aggression: 0.8      # Thornwall's wolves are bolder
```

Override is the sharpest tool and the most restricted: dependencies may never override each other (no action at a distance between peer packs), overrides are declared in one auditable place, and validation reports every override so an author can see exactly how their world diverges from its imports.

## What Composition Never Does

There is no "patching" — no pack may reach into another and rewrite arbitrary structure, no load-order magic decides silent winners, no two packs may declare the same id. Conflicting ids across packs are a validation failure demanding an explicit resolution (rename, or root override), never a coin flip.

---

# Designer Note
## Author Once, Diverge Deliberately

The composition rules encode a authorship philosophy.

Sharing should be easy: five worlds needing temperate wildlife should import one pack, and a fix to elk migration rates should reach all five as a version bump.

Divergence should be *visible*: when Thornwall's wolves are bolder than the shared pack's, that fact is one declared override in one file — not a forked copy of the wildlife pack that silently stops receiving fixes.

The forbidden thing is *invisible* divergence: forked copies, load-order accidents, peer packs fighting over an id. Every mechanism in this chapter is chosen so that the answer to "why is this world's deer different?" is always one query away.

---

# 3.3 The Content Pipeline

Between an author's editor and a running simulation, content passes through a pipeline with defined stages:

```text
Authoring form            — human-friendly files (the package repository)
      │  parse
      ▼
Declaration graph         — typed declarations with unresolved references
      │  resolve (dependencies, extensions, overrides)
      ▼
Composed content          — one flattened truth per id, provenance retained
      │  validate (Ch. 7: schema, references, coherence)
      ▼
Runtime form              — the loader's fact templates, sealed and cached
```

Three properties are architectural; everything else is implementation:

1. **Determinism.** The same package sources compose to the same runtime form, always. The pipeline is a pure function.
2. **Provenance.** Every composed value remembers which pack and which mechanism (base, extension, override) produced it. Authors debugging a world query provenance, not diffs.
3. **Sealing.** The runtime form is immutable for a given package version. The simulation never reads authoring files; it reads the sealed output, which is what the save references (Chapter 6) and what validation certified.

Authoring formats, editors, importers from external tools — all welcome, all outside the specification. The pipeline's mouth is wide; its throat is narrow and typed.

---

# 3.4 Common Queries

- What does this id resolve to, after composition?
- Which pack, and which mechanism, produced this composed value?
- What references this declaration? What does it reference?
- Which declarations would break if this pack were removed or downgraded?
- Where does this root package override its dependencies?
- What content categories does this pack contribute to?

---

# 3.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Content declares kinds; it never instantiates.
2. Every declaration has one namespaced id; no two packs declare the same id.
3. All references resolve at composition; dangling references fail the build.
4. Each fact template lives in its owning domain's category, once.
5. Dependency graphs are acyclic and version-resolved before validation.
6. Extension is additive; only root packages override; peers never touch each other.
7. Composition is deterministic and order-independent.
8. Composed values retain full provenance.
9. The runtime form is sealed per package version; simulation reads only sealed content.
10. The engine's loader knows category schemas and no category members.

---

# 3.6 Anti-Patterns

### Content That Spawns

Declarations with side effects ("this species definition places 200 deer in the north"). Placement is generation. A dictionary that inserts its words into your novel is not a dictionary.

### The Fork

Copying a shared pack into your package to change three values. Use dependency plus override; the fork stops receiving fixes the day it is made and diverges invisibly forever.

### Duplicate Truth Across Categories

Declaring the deer→wolf relation in both species. Volume III Appendix A applies to authors: one fact, one owning category, or the two copies *will* disagree by version 1.3.

### Load-Order Semantics

Any composition result that depends on the order packs are listed. Order-dependence is nondeterminism with a config file.

### Schema Smuggling

Encoding new semantics in free-form content fields ("we put the curse logic in the item's description string and parse it"). If the schema cannot express it, that is a domain or schema proposal — the escape-hatch anti-pattern of Chapter 1, wearing content's clothes.

---

# 3.7 Future Evolution

Future versions of Cardinal may introduce:

- shared pack registries with semantic versioning conventions
- content linting beyond validity (style, balance heuristics, coverage)
- visual authoring tools compiling to the same declaration graph
- localization packs for vocabulary and naming
- procedural content *declarations* (parameterized families of kinds)

Each widens the pipeline's mouth.

The sealed, validated, deterministic throat does not move.

---

## Preparing for the Next Chapter

The dictionary is complete: kinds, rules, and laws. Nothing exists yet.

The next chapter is where existence begins: **World Generation** — how a package's declarations become an initial reality of placed facts, and why a generated world must be indistinguishable, architecturally, from one that was always there.

---

# END OF CHAPTER 3
