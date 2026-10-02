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
