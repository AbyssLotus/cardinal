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

---

## A-3 — Bodies, facing, and motion (2026-10)

**Edits:** Vol. III Ch. 1 (new *Bodies, Facing, and Motion* section after §1.8).

**Change.** Position is one three-component fact (the body's base, in its container's frame).
Bodies may declare a size (half-width, half-depth, height) and a heading (compass bearing); a
container's heading orients its contents' frame. Motion is a single straight segment (target,
departure tick, arrival tick) from which position at any tick is derived; facts change only
when motion changes, and speed and direction are derived queries. The kernel gains a
three-component integer value so a position or target is one atomic fact.

**Rationale.** The audit (§4.3) measured motion written as per-tick position facts: every
mover wrote two or three facts and chronicle entries every tick, about 48 MB of chronicle per
second for 50,000 movers at 10 Hz. Position split across three facts also made each move three
writes that could, in principle, be proposed inconsistently. Without size there was no
"does it fit" or "what is it on"; without facing, no "to your left". A text world needs all of
these answered precisely — where the player stands, which way they face, what the sword is
lying on.

---

## A-4 — Constraints, ground, sight, and travel (2026-10)

**Edits:** Vol. III Ch. 1 §1.11 (new *Constraints Made Concrete* section); Appendix A
(new Ruling 13 — Travel).

**Change.** Bodies may be solid (block movement) and opaque (block sight); regions may be
enclosed (crossed only through portals); portals may be closed and have linked faces; a body
fits a portal only if no wider and no taller than it. Ground is a place's terrain heightfield
or its level floor; mobile bodies without support fall, kinematically, under the world's
gravity. Line of sight is a defined query. Travel is an intent a decider proposes and Physical
Reality carries out — routing through open, fitting portals and around solid bodies — with
blocked travel reported as a fact (Ruling 13).

**Rationale.** §1.11 listed what constraints *are* ("walls block vision", "closed doors prevent
movement") without making any of them a fact or a query, and nothing moved bodies at all: the
3D-readiness audit's building test had to use a stand-in walker that could teleport anywhere.
A world that responds to its own actions needs the world itself — not each decider — to say
whether a way is open, what can be seen, and what happens when a body steps off a ledge.
