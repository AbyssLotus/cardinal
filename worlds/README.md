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

Destined residents (Vol. IV Ch. 8, the standing falsification targets): thornwall/
(medieval baseline), meridian/ (modern city), pelagia/ (ocean — five domains disabled),
kepler-station/ (closed loops), sundered-march/ (fantasy exotica). The POC's aincrad
package is archived with its engine at archive/poc-v0.1/worlds/.
