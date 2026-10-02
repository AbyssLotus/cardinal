# Specification Amendments

The amendment record required by Vol. V Ch. 10 §10.3: every change to an invariant, a
ruling, or a contract is written here, with its rationale, *before* the implementation that
depends on it. Each entry names the chapters it edits; the edited passages carry the
amendment's id inline.

---

## A-1 — Time has units (2026-10)

**Edits:** Vol. II Ch. 2 (new *Simulated Duration* section; invariant 11), Vol. V Ch. 3 §3.2
(cadence in simulated time).

**Change.** A tick is a declared length of simulated time (a world clock rule). Every rate is
declared per unit of simulated time; cadences are declared in simulated time and converted to
ticks by the clock. Tick length becomes a resolution choice that changes no rate. Stochastic
rules are declared by their statistics (spread and memory) and must revert toward a normal.

**Rationale.** Volume II defined time as `Year → Day → Tick` with no physical unit, so every
rule was written per tick. The 3D-readiness audit (`docs/audits/3d-world-readiness.md` §4.1)
measured the consequence: at one-minute ticks a sheltered region lost its day/night swing to
integer rounding; at one-second ticks a day's weather drift grew from 1.6 °C to 136 °C; and at
the shipped hourly ticks, temperature — an unbounded random walk — spread regions that started
at 15 °C to between −55 °C and +74 °C within a simulated year. A world that needs fine ticks
(anything where people walk through rooms) could not keep its climate, and no world could keep
it for long.

---

## A-2 — Spatial queries through a derived index (2026-10)

**Edits:** Vol. V Ch. 2 §2.1 (spatial index under clause 5; conformance and no-opinions rules),
§2.4 (common queries); Vol. III Ch. 1 §1.12 (Physical Reality's proximity queries).

**Change.** The store may keep a spatial index, registered at bootstrap by Physical Reality
as a placement rule (entity → frame + bounding box, from the entity's own facts) and
maintained by `apply()`. The index answers candidate questions; Physical Reality answers
exact ones. Indexed and scanned answers must be identical (conformance), and the index is
never a second source of truth (no opinions; readers declare the watched facts).

**Rationale.** Vol. V Ch. 2 invariant 8 made spatial queries contract surface but the
contract offered only per-fact reads and a by-type roster, so "who is within 10 m" was a
scan of everyone against everyone: 375 ms for 1,000 agents and 3.8 s for 3,000 (audit §4.2),
growing with the square of the agent count. A world of people in rooms needs that question
answered thousands of times a tick.
