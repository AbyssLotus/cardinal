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

---

## A-10 — Fatigue, health, and death (2026-10)

**Edits:**
- Vol. III Ch. 2 §2.4 (*Needs*)
- Appendix A (Ruling 15)
- Vol. II Ch. 4 (perceiving one's own needs)
- Vol. V Ch. 9 §9.3 (minds that rest)

**Change.**
- **Fatigue.** Living Systems tracks fatigue. It rises while awake and falls while resting, at
  rates the world declares.
- **Resting** is an intent any decider may propose. Living carries it out only for a still body.
- **Health** falls under harm from:
  - cold, as body heat below the world's line;
  - falls, beyond a safe drop.

  It recovers slowly when unharmed.
- **Death** is health reaching zero, chronicled with its cause. Metabolism, fatigue, and healing
  stop. The senses go and travel is cancelled, so perception and thought stop too.
- **The information layer** lets a mind feel its fatigue and its health.
- **Minds rest.** A mind with nowhere it wants to go rests when tired, and wakes when rested.
  Cold wakes it. So does a routine, once it is no longer tired.

**Rationale.** The sweep's D7 (body heat had no consequences) and the agents plan's phase 4
(competing needs). Vol. III Ch. 2 asks that organisms die of cold through the same architecture
as wounds, and that life be maintained state, never a flag. So death is an event on a continuous
measure, not a stored truth.

---

## A-11 — Needs that arise, and love (2026-10)

**Edits:**
- Vol. III Ch. 2 §2.4 (*Needs that arise*)
- Vol. III Ch. 5 §5.4 (*Lovers*)
- Vol. II Ch. 4 (*Absence can be seen*; *Fondness is an opinion*)
- Appendix A (temperament, needs that arise, bonds)

**Change.**
- **Needs that arise** (Living Systems).
  - A need may come into being from what happens, as a fact about a pair: the holder, and what
    the need is about.
  - A world declares kinds of need from a closed set of mechanisms. A kind arises from a bond or
    a dependence, is met by presence or a dose, and has rates of growth and easing, and a harm.
  - A need ends when what made it arise is gone.
- **Temperament** (minds). Three traits, declared per mind. Compatibility between two minds is
  100% less the mean difference of their traits.
- **Fondness** (information).
  - It grows while the other is in sight, at the world's rate times their compatibility, and never
    past their compatibility. It fades while they are apart.
  - Absence can be seen: in a lit place, a mind lets go of its belief that a person or movable
    thing is there when it cannot see it.
  - To know its place is lit, a mind's view now includes the place it stands in when that place
    is lit (Physical Reality).
- **Lovers** (Society). A bond forms when fondness is mutual past the world's line. It ends when
  both have cooled below a lower one.
- **Minds** feel their arisen needs and are moved by them on the same scale as warmth and routines.
  A need met by presence takes a mind to where it *believes* its object is.

**Rationale.** The project owner asked that new needs be able to arise, for instance wanting to be
with a lover, and that love grow from time together, limited by compatibility so not everyone
falls for everyone (`docs/audits/needs-that-arise.md`).

---

## A-12 — Items: taking, carrying, dropping, consuming (2026-10)

**Edits:**
- Vol. III Ch. 1 *Constraints Made Concrete* (*Handling things*)
- Appendix A (Ruling 16)
- Vol. II Ch. 4 (believing what things are made of)

**Change.**
- **Intents.** Take, drop, and consume are intents any decider may propose. Physical Reality
  carries them out within reach, as it does opening a door.
- **Carrying.** A taken thing is held, contained in its holder.
- **Weight.** It is size times the densest material the thing is made of, against a carrying
  limit the world declares. A thing whose weight the world does not know is not taken.
- **Dropping** puts the thing where the holder stands.
- **Consuming.** Only something held and made entirely of materials a body can take in (a new
  material property, *edible*) can be consumed. The thing leaves the world: its placement is
  cleared, and its identity and composition remain. Physical reports what the body consumed.
- **Perception** believes what seen things are made of.

**Rationale.** The project owner asked for drugs and addiction, and chose to build items first so
that what is consumed truly leaves the world (Vol. III Ch. 2: consumption is conservative). These
are also the sweep's M6 and M7, the core verbs of a text game.

---

## A-13 — Substances and addiction (2026-10)

**Edits:**
- Vol. III Ch. 2 §2.4 (*Dependence*)
- Vol. III Ch. 1 (*Potency and habit*)
- Vol. V Ch. 9 §9.3 (minds that seek a dose)

**Change.**
- **Material properties.** Two new ones: *potency* and *habit*.
- **Dependence.** When a body consumes something, Living Systems judges the dose from
  Physical's report. For each habit-forming material in it, the body's dependence on that
  material (a fact about a pair) rises by the material's habit. Dependence fades at a daily rate
  the world declares.
- **Craving.** A need kind that arises from dependence has a line:
  - past the line, the need comes into being;
  - it grows at the kind's rate, scaled by how deep the dependence is;
  - a dose of the material eases it by the kind's measure;
  - it harms while unmet, as the kind declares;
  - it ends when dependence falls below the line.
- **Starting state.** A world may declare the dependences its people start with.
- **Minds.** A mind that craves looks for things it believes are made of the substance. It goes to
  where it believes they are, takes one, and consumes it. A mind feels when what it held is gone.

**Rationale.** The project owner asked that needs be able to arise from taking a drug and becoming
addicted (`docs/audits/needs-that-arise.md`, step 3), built on real items (A-12) so each dose leaves
the world.

---

## A-14 — Everyone acts, unless directed (2026-10)

**Edits:** Vol. V Ch. 9 §9.3 (*Direction*); Appendix A (direction, under decision systems).

**Change.**
- **Every organism may have a mind,** and acts on its own.
- **Direction.** A controller (a player, a recorded model, a test) may *direct* an entity: a mind
  fact anyone may set.
- **A directed mind stands aside.** It proposes nothing, so the controller's intents are the
  entity's only ones. The entity still perceives, its needs still run, and the world still judges
  its acts.
- **Timing.** A controller first directs, then commands on the following tick, so a mind and its
  controller never propose at once.
- **Release.** A released mind decides for itself again, from what it now believes.

**Rationale.** The project owner asked that agents act autonomously on their needs, wants, and jobs,
and that everyone be one (`docs/audits/autonomous-agents.md`, phase 1). Direction is how the
player character, and tests that need someone to do something, coexist with a city of minds.

---

## A-15 — Things come into being (2026-10)

**Edits:**
- Vol. V Ch. 2 §2.1 (*New identities*)
- Appendix A (Ruling 16, arriving, leaving, picking)
- Vol. III Ch. 3 §3.4 (deposits that regrow; picking)
- Vol. II Ch. 4 (believing a deposit's stock)

**Change.**
- **New ids.** The kernel issues them to systems. Each is `floor + tick·2²⁴ + slot·2¹⁴ + n`: the
  slot is the system's place among all systems sorted by id, and `n` counts its requests this
  tick. That makes ids deterministic, disjoint, and permanent.
- **Arriving and leaving.** Physical Reality carries out arrival (in a place, or in a hand) and
  leaving (out of the world) on request. *(A-18: or at a fixture, set down just in front of it,
  in its place, where one can walk up to it.)*
- **Picking.** Physical carries out picking within reach of a deposit, and reports it.
- **Deposits** (Resources). A deposit's stock regrows at a declared rate up to a cap. A reported
  pick takes one unit and brings a new item into the picker's hand.
- **Perception** believes how much a seen deposit holds.

**Rationale.** The project owner chose that production create real things
(`docs/audits/autonomous-agents.md`, phase 2). This is the sweep's M7, which also opens the way
to births and building.

---

## A-16 — Hunger (2026-10)

**Edits:**
- Vol. III Ch. 2 §2.4 (*Hunger*)
- Vol. III Ch. 1 (nutrition, a material property)
- Vol. II Ch. 4 (believing what is food, and what a deposit yields)
- Vol. V Ch. 9 §9.3 (minds that eat)
- Vol. IV Ch. 4 (tick zero is lit as the sun stands)

**Change.**
- **Hunger** (Living Systems). It rises hourly at the world's rate. Each judged consumption lowers
  it by the nutrition of what was eaten. Past the world's starvation line it harms health.
- **Nutrition** is a material property.
- **Perception** believes:
  - one's own hunger;
  - the nutrition of materials seen in things;
  - what a seen deposit yields.
- **Minds.** Hungry past the world's line, a mind weighs eating on the same scale as everything
  else. It considers food in hand, food it believes lies somewhere it knows a way to, and deposits
  it believes bear food. It goes, takes or picks, and eats.
- **Generalised.** The fetch-and-consume plan of A-13 now serves any need met by consuming.
- **A world begins with its light.** Tick zero is a complete initial reality (Vol. IV Ch. 4), and
  that includes the light it is seen by. The loader seeds each region's
  illumination as the sun stands at the first moment, from the same rule the day/night cycle
  follows. Before this, the first tick had no light committed, and unlit counts as lit, so a world
  that opened at midnight was seen whole for one tick.
- **Starting knowledge sees deposits.** Deposits are seeded before minds' starting knowledge, so
  a mind that knows a tree knows what it bears and how much.

**Rationale.** `docs/audits/autonomous-agents.md`, phase 3: the first need met by things that come
into being.


---

## A-17 — Making things (2026-10)

**Edits:**
- Vol. III Ch. 4 §4.4 (*Production*: recipes, the make intent, time in the making)
- Appendix A (Ruling 17, making)

**Change.**
- **Recipes** (Economy) are package data. A recipe names:
  - what it needs: so many things of each material;
  - what it makes: a material, and the size of the thing;
  - where it is made: a workplace, such as a hearth;
  - how long the thing is in the making.
- **Making.** A decider proposes that a maker *make* a recipe. Economy judges it:
  - The maker must stand in the place where the workplace is.
  - The maker must carry what the recipe needs, and must have nothing else in the making.
  - If so, the inputs are used up: Economy asks Physical Reality for them to leave the world.
  - When the time in the making has passed, the product arrives: in the maker's hand if they
    stand where the workplace is, and otherwise at the workplace, set down in front of it. The
    maker need not wait.
  - Otherwise the request is refused, and the refusal is recorded for the decider to notice.
- **Conservation.** Nothing is made without its inputs, and the inputs are gone before the product
  exists (Vol. III Ch. 4, invariant 6).

**Rationale.** `docs/audits/autonomous-agents.md`, phase 4. The cook's job (A-18) needs a way to
turn the orchard's apples into something more. A pie is worth more to the hungry than the apples
and flour it took.

---

## A-18 — Jobs (2026-10)

**Edits:**
- Vol. III Ch. 5 §5.4 (*Role*: job kinds, and who holds them)
- Vol. II Ch. 4 (knowing one's own work; knowing a recipe)
- Vol. V Ch. 9 §9.3 (minds that work)

**Change.**
- **Job kinds** (Society) are package data, built from a closed set of mechanisms:
  - *carry*: bring things of a material from a source (a deposit, or a place) to a store;
  - *make*: keep a store supplied with what a recipe makes.

  Each job names its store, how many of its goods to keep there, and its hours.
- **Roles** (Society). A role says who holds which job. Others can plan around it.
- **Knowing one's work** (Information). A person knows the job they hold, and what it asks, as
  they know their own body. A mind told of a recipe knows what it needs, what it makes, and where.
- **Working** (Minds). In its hours, a mind weighs its work on the same scale as everything else,
  at a worth the world declares. The work is wanted while the store, as the mind believes it,
  holds fewer goods than the job keeps there, and the mind knows where to get what the work
  needs. Its plan is reactive, recomputed from beliefs at every step:
  - *carry*: gather goods from the source until the store would be stocked, take them to the
    store, and put them down there, one at a time;
  - *make*: put down any product it carries in the store; otherwise get what the recipe needs
    that it is not carrying, nearest first; then make the recipe at its workplace.

  Hunger, cold, longing and tiredness still outrank work when they press hard enough.
- **Sleep.** In the world's sleeping hours, a mind with nothing better to do lies down once at
  all tired, and stays down until they end unless something calls it. A day's waking and a
  night's rest then balance as the world's rates intend, instead of everyone tiring at noon.
- **Giving up.** A mind that tries to get a thing and cannot (the act refused, or the way to it
  blocked) gives up on it until it learns something newer of where it is. In the dark, a pie
  someone else has eaten is not sought all night.
- **The hungry prefer what fills them.** A food's worth to a hungry mind includes how much it is
  believed to feed, so a pie a room away beats an apple at hand.
- **Perception, by touch and by eye.**
  - One knows where one put something down, even in the dark.
  - One feels how much is left on what one has just picked from.
  - Standing in a lit place, one notices that a thing one could carry off is no longer there.

**Rationale.** `docs/audits/autonomous-agents.md`, phase 5. The project owner chose
farm-and-stock and cook as the first jobs. Roles are Society's (Appendix A). The minds read only
beliefs (Ruling 14), so a person's job reaches its mind the way its hunger does: through what it
knows of itself.

---

## A-19 — Wants (2026-10)

**Edits:**
- Vol. III Ch. 4 §4.4 (*Holding*: owners, and claiming what no one owns)
- Appendix A (Ruling 18, claiming)
- Vol. II Ch. 4 (believing who owns a thing)
- Vol. V Ch. 9 §9.3 (minds that want)

**Change.**
- **Curiosity** (Minds, a disposition). A curious mind is drawn to places it knows a way to but
  has never stood in. A place it has stood in, or was told of at the start, it knows the feel of,
  and is no longer curious about. Each mind's curiosity is declared by the world; most have
  none.
- **Owners** (Economy). A thing may have an owner. The world declares who owns what at the start.
- **Claiming.** A decider proposes that someone *claim* a thing. Economy judges it: the claimant
  must carry the thing, and no one may own it already. Then the claimant owns it. Otherwise the
  claim is refused, and the refusal recorded.
- **Believing who owns a thing** (Information). Seen, a thing's owner is believed, as its material
  is.
- **Likes and home** (Minds, dispositions). A mind may like some materials, each with a worth, and
  may have a home: a place.
- **Wanting** (Minds). When nothing presses, a mind:
  - carries home what it owns and puts it down there;
  - fetches what it owns that lies elsewhere;
  - picks up a thing no one owns, made of something it likes, and claims it.

  A thing's worth to it is how much it likes what the thing is made of. Wants score low, so
  they fill idle hours.
- **What is someone else's** is never taken, by any want, hunger, or work: a mind does not reach
  for a thing it believes another owns.

**Rationale.** `docs/audits/autonomous-agents.md`, phase 6. The project owner chose curiosity and
possessions as the first wants. Holdings are Economy's (Appendix A); likes and a home are a mind's
own, as its temperament is (A-11).
