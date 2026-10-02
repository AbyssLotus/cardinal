# worlds/

World packages: content, never code (Vol. IV Ch. 1 — packages are data: they declare,
and cannot execute). Nothing in kernel/, domains/, services/, or frontends/ imports
from here; packages arrive through services/packages/ as validated, sealed data or not
at all (Vol. V Ch. 1 §1.1 rule 3 — the POC's oldest hard rule, now build law).

Each package follows Vol. IV Ch. 1 §1.2:

```text
worlds/<name>/
├── manifest      — id, version, engine range, vocabulary, domain selection
├── rules         — every tunable number and law (Vol. IV Ch. 2)
├── content/      — data packs (Vol. IV Ch. 3)
├── generation/   — the layer recipe (Vol. IV Ch. 4)
├── scenarios/    — named starting situations (Vol. IV Ch. 5)
└── validation/   — package invariants, reference seeds + signatures, probes
                    (Vol. IV Ch. 7 §7.3)
```

## ashford.world — the working reference city

Ashford is the fictional city every integration test runs in (`tests/reference/`). New
features are tested by what they do *here* — Alice's house, the manor, the cottage on the
Hill, the Heron in the Harbour, Millside's farm and mill — rather than in a small world built
for one test. When a feature needs something Ashford lacks, Ashford grows it.

```text
the Reach                          (no climate; the root)
├── the Vale        14 °C, 50 m    ─┐
│   └── Ashford                     │ adjacent climates: wind runs
│       ├── Old Town                │ down the pressure gradient
│       │   ├── Alice's yard, house (cellar, kitchen, bedroom), shed, hay wagon, cart
│       │   └── the manor grounds, manor house (hall, undercroft, gallery, sealed vault)
│       ├── the Hill    terrain: a ridge with a cliff and a pass; the cottage (kitchen, loft)
│       │               and the mine (mouth, cabin, deep gallery — by declared exposure)
│       ├── the Harbour the Heron (a ship whose deck turns with her), gate, well, stables
│       └── Millside    farm, farmhouse, wood, mill — and overlapping regions over them
├── Highmoor        6 °C, 400 m, granite
└── Southfen       18 °C, 5 m
```

Ashford is written in the engine's current single-file format: one `.world` file of
`[sections]` (manifest, clock, rules, places, containment, positions, portals, bodies,
facing, flags, motion, terrain, materials, organisms, regions). The directory layout above
is where packages are headed as content packs, generation, and scenarios arrive.

## Destined residents

Destined residents (Vol. IV Ch. 8, the standing falsification targets): thornwall/
(medieval baseline), meridian/ (modern city), pelagia/ (ocean — five domains disabled),
kepler-station/ (closed loops), sundered-march/ (fantasy exotica). The POC's aincrad
package is archived with its engine at archive/poc-v0.1/worlds/.
