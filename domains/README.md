# domains/

Volume III made executable: one module per domain, each containing its fact-type schemas,
its systems, and its composition validators (Vol. V Ch. 1 §1.1).

**Governing spec:** Vol. III (all chapters + Appendix A); Vol. V Ch. 1 (registration),
Ch. 3 (systems as hermetic transformations).

## The domains

| Directory | Volume III | Owns (per Appendix A) |
|---|---|---|
| `physical/` | Ch. 1 | space, topology, regions, containment, materials, environment — **mandatory in every world** (Vol. IV Ch. 2 §2.1) |
| `living/` | Ch. 2 | vital state, metabolism, lifecycle, capability, inheritance, death |
| `resources/` | Ch. 3 | definitions, deposits, grades, non-living stocks, regeneration |
| `economy/` | Ch. 4 | holdings, transfers, exchanges, prices-as-events, money, markets |
| `society/` | Ch. 5 | kinship, households, settlements, roles, reputation, membership |
| `culture/` | Ch. 6 | values, norms, customs, language, lore, transmission |
| `knowledge/` | Ch. 7 | techniques, knowing, proficiency, records, discovery |
| `conflict/` | Ch. 8 | grievances, hostility, engagements, morale, terms |
| `institutions/` | Ch. 9 | offices, laws, jurisdiction, legitimacy, treaties, succession |
| `ecology/` | Ch. 10 | populations, food webs, carrying capacity, migration, individuation |

Each domain directory follows the same internal shape (Vol. V Ch. 1 §1.3):

```text
<domain>/
├── schema/       — fact-type declarations this domain owns
├── systems/      — hermetic transformations, with declared read/write sets & cadence
├── composition/  — proposal-composition validators for owned fact types (Vol. IV Ch. 2)
└── README.md     — the domain's charter: owns / never owns / spec references
```

## Law binding this directory

- **Domains never import domains.** Cross-domain effect is committed reality — proposals
  and events, never calls (Vol. III Ch. 12 §12.1; Vol. V Ch. 1 §1.1 rule 2, build-enforced).
- **One fact, one owner.** Every fact type here appears exactly once, in its owner's
  schema, per [Appendix A](../docs/design-spec/volume-3-domains-of-reality/APPENDIX_A_Ownership_Matrix.md).
  Boundary disputes are settled in Appendix A *before* implementation — argue from its
  twelve rulings as precedent.
- **Registration is data.** Domains register schemas, systems, and validators at bootstrap
  step 2 (Vol. V Ch. 1 §1.2); domain selection is a world-package decision (Vol. IV Ch. 2),
  and every domain except `physical/` must be cleanly absent when disabled — Pelagia is
  watching (Vol. IV Ch. 8 §8.3).
- **No phenomenon modules.** No `famine/`, no `war_generator/` — implement causes, never
  outcomes (Vol. III Ch. 11 §11.3).
