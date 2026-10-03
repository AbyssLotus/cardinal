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

---

## A-5 — Authority, shelter, and ground-relative height (2026-10)

**Edits:** Vol. V Ch. 3 §3.1 (*Owners may refuse writers*); Appendix A Ruling 13 (operating
and turning are intents; restricted facts); Vol. III Ch. 1 *Constraints Made Concrete*
(*Shelter*; *Height above the ground*).

**Change.** An owner may restrict a fact type to writes from its own registered systems, and
the kernel enforces it, refusing duplicate system ids. Physical Reality restricts position,
containment, motion, facing, door state, and its derived reports; deciders open, shut, and
turn through intents that are carried out only within reach (a world rule), with both faces of
a door moving together. Enclosed regions are sheltered: no direct sun or sky weather unless
declared exposed, daylight through light-passing openings in proportion to their area, and a
temperature that follows the outside air with a declared lag. Height above the ground is
measured from the ground beneath, not from the world datum.

**Rationale.** The systems sweep (`docs/audits/system-sweep.md`) verified that any system could
teleport a body or open a door from anywhere (D2), that a walled kitchen got full noon sun and
its own outdoor weather (D4), and that terrain made a ground-level door on a hillside rate as
a 3 m fall (D3).

---

## A-6 — Thermal mass is heat stored per volume (2026-10)

**Edits:** Vol. III Ch. 1 *Materials* and *Constraints Made Concrete* (*Thermal mass*).

**Change.** How strongly a region's material resists changes of temperature — damping its
day/night swing and its weather, and lengthening an indoor room's lag behind the outside air —
is the heat its material stores per unit volume: density × specific heat, kJ/(m³·K). Before,
it was specific heat alone. As with the other composite aggregates, a composite uses its
dominant constituent; a material that declares no density has no thermal mass. The world rule
`thermal_mass_reference` is restated in the same unit: the thermal mass at which a swing is
halved.

**Rationale.** A wall holds heat in proportion to its volume, not its weight. Per kilogram,
timber (1700 J/(kg·K)) stores twice what granite (790) does; per cubic metre granite stores
nearly twice what timber does (2133 against 1190 kJ/(m³·K)). Measured per kilogram, the Ashford
reference world's timber kitchen lagged the weather *more* than its granite manor — the
opposite of A-5's promise that a stone cottage keeps the afternoon's warmth into the night.

---

## A-7 — A fact may be about a pair (2026-10)

**Edits:** Vol. V Ch. 2 §2.1 (*A fact may be about a pair*).

**Change.** A fact is addressed by an entity and a fact type, and — optionally — a second entity
it is *about*. `(Erin, belief.place_of, about Bob) = the kitchen` is one fact: owned, with
provenance and cardinality like any other, written only through `apply()`. The store reads one
such fact by its full address, and all of a holder's facts of one type in order of what they are
about. Facts without a second entity are unchanged, in meaning and in their committed digest.

**Rationale.** The agents plan (`docs/audits/agents.md`, §4.1) needs a place for beliefs, and a
belief is about a pair: who holds it and what it is about. The spec needs the same shape next for
one person's opinion of another (Appendix A, Ruling 2), relationships in several dimensions
(Vol. I Ch. 4 §12), and debts. The alternatives were a new entity per belief, which needs a
runtime id allocator and an index of its own, or a record-shaped value, which would teach the
kernel the shapes of domain data.

---

## A-8 — Perception and belief (2026-10)

**Edits:**
- Vol. II Ch. 4 (*The Layer as Built*)
- Vol. III Ch. 1 *Constraints Made Concrete* (*What is in view*)
- Vol. III Ch. 2 §2.2 (sight range)
- Appendix A (in view)
- Vol. V Ch. 1 §1.1 (the information layer in the domain layer)

**Change.**
- **Sight range.** Living Systems declares how far each organism can see.
- **In view.** On a cadence the world declares, Physical Reality publishes what each body with sight could see: within range, along a clear line, and lit above a threshold the world declares.
- **Night is dark.** Daylight follows the sun's height, zero from sunset to sunrise. It was a triangle wave from midnight to noon, which left Ashford above the sight threshold for all but about fourteen minutes of the night.
- **Perception.** The information layer is a new owner of facts with no imports. It turns what is in view into beliefs, held as facts about a pair (A-7):
  - where things were seen, and what openings were seen to lead to and whether they were open;
  - how warm the places one has stood in felt;
  - what one feels of oneself: one's place, one's body heat, where one is going, whether one's way was blocked, and what one was refused.
- **Memory.** A belief stays current while its subject is in sight. When the subject leaves sight, the belief is stamped with when it was last seen, and is otherwise left alone.
- **Starting knowledge.** What each mind knows at the start is package data (`[knows]`).

**Rationale.** The agents plan (`docs/audits/agents.md`, §4.3–4.4). Domains never import each other, and sight is Physical's geometry, so Physical publishes the geometric fact and the information layer makes the observation. Confidence derived from age keeps memory exact under replay and free to store.

---

## A-9 — Minds (2026-10)

**Edits:**
- Appendix A (goals, commitments, routines, traces; Ruling 14)
- Vol. V Ch. 9 §9.3 (*The reference mind, as built*)
- Vol. V Ch. 1 §1.1 (decision systems in the domain layer)

**Change.** Decision systems are a new owner of facts with no imports (`domains/minds`).
- **Reads.** A mind reads only the information layer's facts and its own commitments.
- **Choosing.** On a cadence the world declares, it scores each place it knows by:
  - the warmth it remembers there against the cold it feels (its body heat below the world's
    comfort line), trusted less the older the memory;
  - the routine the world gives it for the hour;
  - the number of openings it believes lie on the way.
- **Commitment.** It keeps a choice until it arrives, unless another beats it by a margin the
  world declares.
- **Acting.** It acts by travel and open intents. When its travel is blocked, it walks to the
  first opening on the way it believes in and opens it.
- **Tracing.** It records its goal, its reason, and the decisive score.
- **Package data.** Which entities have minds, how fast they walk, and their daily routines.
- **A fresh intent outlives the clearing of the old.** If a decider asks for a new destination,
  or a door opened or shut, in the tick Physical Reality clears the intent it just fulfilled,
  the new one stands. This is Physical's composition rule for intents. Two deciders asking for
  different things at once remains a conflict.

**Rationale.** The agents plan (`docs/audits/agents.md`, §4.5), and the project owner's choice
of warmth and routines as the first goals.

