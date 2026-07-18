# frontends/

Consumers of the world, and the only place input originates. Frontends receive
entitled observation streams (Vol. V Ch. 6 §6.2.3 — what may this consumer see is
computed at the plane, per perspective) and submit intent through the one front door:
INTERPRET → VALIDATE → the tick (Vol. II §1.2; Vol. V Ch. 6 §6.2).

**Governing spec:** Vol. V Ch. 1 (layer law), Ch. 6 (plane + front door), Ch. 9
(AI integration — models at the doors, never in the walls).

| Directory | Duty | Spec |
|---|---|---|
| `cli/` | the deterministic command grammar — first-class forever, complete without any model | Vol. V Ch. 9 §9.5.10 |
| `narrator/` | prose from perspective-filtered committed state; fails closed to the deterministic renderer | Vol. V Ch. 9 §9.1 |
| `tools/` | inspectors, dashboards, historians' queries — read-only without exception | Vol. V Ch. 8; Vol. III Ch. 11 §11.4 |

**Law:** no frontend concern leaks into domains (Vol. V Ch. 1 §1.6 — a domain that
knows about prose will someday lie for it); prose is never parsed into, cached as, or
consulted as world state (Ch. 9 §9.5.4).
