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

---

## A-20 — Levels: quantities that move at a steady rate (2026-10)

**Edits:**
- Vol. V Ch. 2 §2.2 (*Levels*, beside motion segments)
- Vol. III Ch. 2 §2.4 (hunger and fatigue are levels)
- Vol. II Ch. 4 (feeling one's own levels)

**Change.**
- **A level** is a quantity that moves at a steady rate between events: hunger between meals,
  fatigue between lying down and getting up. It is one three-component fact, `[value, rate per
  hour, since tick]`: where it stood, how fast it moves, and since when. Its value at any later
  tick is derived exactly from the simulated time elapsed, held within the quantity's bounds. It is
  never stored per tick.
- **Writing a level.** A level is written only when its rate or its value jumps:
  - eating lowers hunger by the food's nutrition, from wherever it had reached;
  - lying down, getting up, or moving changes the rate of fatigue;
  - death stops both.

  Between such events nothing is written and nothing is chronicled.
- **Reading a level.** Any domain reads a level the same way, through the kernel's one
  definition, so every reader agrees on its value at every tick.
- **Feeling one's levels** (Information). One knows one's own hunger and fatigue as they are: the
  felt level is the body's level, copied when it changes.
- **Exact at any tick length.** The value at a moment of simulated time is the same whether the
  world ticks by the second or by the hour (A-1), with no rounding carried from tick to tick.
- **A change of course holds from the committed tick.** A meal, lying down, or getting up
  changes a level from the tick whose state showed it, as a step's rate always held over the
  step after the state it read.
- **Tiredness has a margin** (Minds). A felt level is exact, where it used to be noticed only in
  steps of the world's resolution, and those steps had given a mind getting up for its routine
  some slack before it felt tired again. That slack is now a world rule: a mind already up and
  about its routine or work carries on until it is `tired_margin` past the tired line.

**Rationale.** `docs/audits/world-scale.md`, phase 1. Rewriting hunger and fatigue for every
organism every tick was about a third of everything the chronicle recorded. A city of a million
cannot afford a write per person per tick for numbers anyone could compute. Motion already works
this way (A-3).

---

## A-21 — What changed, and what it cost (2026-10)

**Edits:**
- Vol. V Ch. 2 §2.1 (the store answers what changed)
- Vol. V Ch. 3 §3.1 (the tick meters its systems)
- Vol. V Ch. 8 (per-system meters)

**Change.**
- **What changed.** The committed view answers one more question: *which entities had facts of
  this type written in the ticks after `t`?*
  - The answer is sorted, and costs time in proportion to its size.
  - A store may remember only a window of recent ticks. Asked about something older, it answers
    *unknown*, never a guess.
  - A system given *unknown* does all the work it would do without the answer. A system's first
    evaluation always does all of it, since nothing it computed before can be standing yet.
  - So no committed fact may depend on how far back a store remembers. That is tested by running a
    world with the window and without it, and comparing every tick.
  - A system reads changes only to fact types it declares it reads.
- **What it cost.** The tick records, for each system it evaluates, how long it took and how many
  proposals it made. The figures sit outside the simulation: no system reads them and no fact
  depends on them, so they cannot touch determinism. They are the first of Vol. V Ch. 8's meters.

**Rationale.** `docs/audits/world-scale.md`, phase 1, and §7: work done only for what changed is
how a world of a million does a city's work, not a million people's. Finding where a tick's time
goes took an outside profiler; the engine should say.

---

## A-22 — The agenda (2026-10)

**Edits:**
- Vol. V Ch. 2 §2.1 (the store answers who is due)
- Vol. V Ch. 3 §3.2 (scheduling by what is due)
- Vol. I Ch. 4 §7.2 (advance to the next due boundary: built)

**Change.**
- **Agenda facts.** A domain may declare fact types whose value is a tick: the tick at which an
  entity is next due for that domain's attention.
  - A resident's plan step ends then.
  - A level reaches its line then.
  - A making is ready then.
- **Who is due.** The committed view answers *which entities are due, by this fact type, at or
  before tick t?*, in ascending order of id.
  - A store may keep an index of agenda facts, so the answer costs time in proportion to its
    size, not to how many entities hold the fact.
  - The answer must be the same with the index or without it; it is tested both ways.
- **Using it.** A system that finds everything it must do through what is due (and, A-21, what
  changed) does no work for entities with nothing due. The world goes on for them: their levels
  move, their plans run (A-20), and what is not computed until it matters is computed exactly
  then.

**Rationale.** `docs/audits/world-scale.md` §8: the city tier's million residents must cost nothing
until something is due. The spec has asked for this since Vol. I ("the scheduler should advance
directly to the next due boundary"); this is its kernel primitive.

---

## A-23 — The city tier: residents (2026-10)

**Edits:**
- Appendix A (Ruling 19, one person, one tier at a time)
- Vol. III Ch. 5 (residents, households, and the rhythm of a settlement)
- Vol. I Ch. 4 §7.3 (Tier 1, individual simulation for loaded settlements: built)

**Change.**
- **Residents** (Society). At the city tier a person is a resident record, and nothing else:
  - home and workplace, both places;
  - what they are doing (sleeping, at home, walking, working, eating);
  - where, and until which tick, an agenda fact (A-22);
  - how hungry and how tired they are, as levels (A-20).

  A household's larder, a count of meals, belongs to its home.
- **The rhythm** (Society). A resident is attended to only when the thing they are doing ends.
  Then the next thing is chosen from the hour and their state:
  - **at night,** sleep at home, resting;
  - **hungry, with food at home,** eat a meal: the larder gives one, hunger falls by its
    nutrition;
  - **in working hours,** walk to work, taking the time the distance takes at the world's walking
    pace, and work until the hours end;
  - **otherwise,** home, until the next of these is due.

  Every rule and rate comes from the package.
- **Everyone keeps the hours a little their own way.** Each resident's night and working hours
  are the settlement's, shifted by a personal offset of up to the world's `hours_spread_minutes`
  either side. The offset is fixed for each resident, derived from who they are. A city does not
  all wake in the same minute, and its work is spread across the hour as people are.
- **Earning the keep.** A finished shift adds the world's meals per shift to the worker's
  household larder: an explicit boundary process (Vol. I, *Conservation*), standing in for the
  economy the city tier does not simulate in detail.
- **Cost.** A resident with nothing ending costs nothing. A day is a handful of segments, each a
  few writes.

**Not yet** (the next slice): promotion into the scene and demotion back across the seam; harm and
death at the city tier; buying and selling.

**Rationale.** `docs/audits/world-scale.md` §8, step 2: a city of a million on a desktop, every
resident an individual who costs nothing until something is due.

---

## A-24 — Crossing the seam: promotion (2026-10)

**Edits:**
- Appendix A (Ruling 19: how a person crosses; Ruling 14: a mind is made from a record)
- Vol. V Ch. 5 §5.2 (the residency ladder, per place: built for promotion)
- Vol. V Ch. 2 (who holds a value: the agenda indexes entities too)
- Vol. I, Law 14 (scale must not change truth: the promotion side, tested)

**Change.**
- **Residency** (Society) is a place's rung: *city* or *scene*.
  - A place takes the rung of the nearest place holding it that declares one. With none
    declared, it is in the scene.
  - Residency is policy: whoever directs attention may set it (a player's frontend, a tool, a
    test). It changes what is computed, never what is true.
  - A package declares the rungs it starts with. A resident's home must be at the city tier at
    load: a person the world means to be in the scene is declared as an organism.
- **Marking a crossing** (Society). A resident in the scene is marked *crossing*, naming the place
  they cross into. There are two ways in:
  - **Attention arrives.** When a place's rung becomes *scene*, everyone not walking who is at
    that place, or at a place within it, is marked. A place within it that keeps the city tier
    keeps its people.
  - **Walking in.** A resident's place changes only by walking. When a walk ends at a place in the
    scene, the rhythm marks them instead of choosing their next segment.
- **Crossing.** On the next tick every domain acts on the mark at once. Each reads the same
  committed record, so the person changes tier in a single commit:
  - **Physical Reality** gives them a body of the world's resident size, able to move, in the
    place they cross into;
  - **Living Systems** copies their hunger and fatigue exactly, levels and all (A-20). It gives
    them full health, body heat at the world's set point, the world's resident sight, and rest
    if they were asleep;
  - **the information layer** gives them what a resident knows: where they are, where their home
    and work are, the openings of each and into each from where it lies, and their hunger and
    fatigue as felt;
  - **decision systems** give them the residents' walking pace, their home, and lying down if
    they were asleep. One with work gets two routines in their own hours: to work in working
    hours, home for the rest. These are the same personal hours the city tier kept (A-23);
  - **Society** retires the record's state and the mark. It keeps only who they are: home and
    work.
- **Never both.** No tick holds both a resident's state and an organism's: Ruling 19 holds through
  the crossing. Hunger and fatigue are continuous across it.
- **Who is at a place** (kernel). An agenda fact (A-22) whose value is an entity is indexed by the
  entity it names, so `holding(fact, value)` finds who is at a place in proportion to how many
  are. Attention arriving at a district costs what its residents cost, not what the city costs.
  Without an index the answer comes from a scan, and is the same.
- **A fixed trait from an id** (kernel). `EntityId::jitter(by)` gives an offset within `±by`
  derived from the id alone. Society and decision systems both compute a resident's hours with
  it, so the two tiers agree without either importing the other.

**Measured** (`examples/city.rs`, minute ticks; Ryzen-class desktop figures are the target, these
are from the development machine): a district of 3,000 residents crosses in about 50 ms, the
same at a city of 100,000 as at a city of a million. A million residents still average 28 ms a tick,
and peak at 2.7 GB.

**Not yet** (the next slice):
- demotion back to the city tier, which must retire facts other systems may be writing in the
  same tick;
- carrying held things across;
- a household's larder becoming food one can take in the scene. Until then, a resident in the
  scene grows hungry as anyone does, and eats only what they find.

**Rationale.** `docs/audits/world-scale.md` §9: the player turns into a street of the city, and its
people are there, as themselves.

---

## A-25 — Crossing back: demotion, and retirement (2026-10)

**Edits:**
- Appendix A (Ruling 19: how a person leaves the scene)
- Vol. V Ch. 3 §3.1 (Resolve: retirement)
- Vol. V Ch. 5 §5.2 (the residency ladder: both directions built; the round trip tested)
- Vol. I, Law 14 (scale must not change truth: tested to the bit)

**Change.**
- **Retirement** (kernel). A new kind of change, `Retire`. It is proposed on any fact of an
  entity, and retires the entity from the keeping of that fact type's owner:
  - every fact the entity holds whose type that owner owns ends this tick, those about a pair
    included;
  - every other change proposed this tick to such a fact is moot, and is dropped before
    resolution;
  - a fact the owner keeps for its own systems (Amendment A-5) is retired only by one of them;
  - each fact retired is chronicled under the retirement's cause;
  - facts other owners keep for the entity, and facts others hold *about* it, are untouched.

  The kernel carries it out; an owner's composition never sees it. This is how a domain lets go
  of a person at once, whatever its other systems were about to write about them that tick,
  without every system having to know that people leave.
- **Leaving the scene** (Society). A resident in the scene who stands in a place at the city tier,
  alive and carrying nothing, is marked *leaving*. On the next tick:
  - Physical Reality, Living Systems, the information layer, and decision systems each retire
    them;
  - Society takes up their record again. Their hunger and fatigue are their body's, exactly. If
    they were lying down at home in their own night, they are asleep until their own morning, as
    the rhythm would have had them. At home or at work they have otherwise just arrived, and the
    rhythm takes them up on the next tick. Anywhere else they walk on from where they stand: to
    work in their working hours, otherwise home, taking the time the distance takes.

  A resident in the scene is recorded as such (`society.resident.in_scene`), so finding who might
  leave costs what the scene costs.
- **What the city tier keeps and does not.** It keeps who a person is (home and work), how hungry
  and tired they are, and where they are going. It keeps no memory, needs that arose, or
  dependence: these end at the seam. A person who returns to the scene knows what a resident
  knows.
- **Away from home and work, in working hours** (the rhythm). A resident who is at neither,
  having just come out of the scene, walks to work in working hours. Before this amendment no
  resident could be anywhere else, so the rule changes nothing that was.
- **Knowing the way across a city** (the information layer). A resident crossing in knows the
  openings of their home, their work, and where they stand, and of every place holding each,
  between it and the next out: the door onto the street, and the street's gate onto the city. So
  one who lives in one district and works in another can walk between them in the scene.

**Law 14, tested.** Ashford's Lanes brought into the scene at three in the morning and let go at
half past leave the whole world's state hash identical, a day later, to a run where no one looked
(`crossing.rs`).

**A day of crossings** (`commute.rs`). In a generated town of four districts, with attention on
two, people cross the seam all day as they commute:
- at every tick each resident is exactly one thing, a record or an organism;
- at every crossing, either way, their hunger is the same to the unit;
- at eleven, every worker is at work, at whichever tier their workshop is;
- at day's end, everyone is home.

**The scene, cheaper and exact.** Generated districts showed the scene's systems doing work that
grew with the city, not the scene. Each fix keeps every answer the same.
- **Clear frames** (sight). Each evaluation works out once which frames hold nothing that could
  block a line: no ground, and nothing walled or opaque anywhere within. A line through such a
  frame is never tested inside it, and such a place is never scanned as an obstacle. A frame is
  judged by looking over at most 16,384 things within it. One holding more — a city's open
  ground — is looked into near each line instead, and the frames within it finished on the way
  stay known.
- **Bare places near a line** (sight). The places with no body of their own that a line could meet
  are found by one search of the index, widened by the farthest any of them reaches, not by
  scanning every such place in the frame for every line.
- **Once per evaluation** (sight). Each thing's chain of containers, where it stands in each
  frame, and each place's light are worked out once and shared by every line.
- **Which branches a search enters** (kernel). The spatial index answers which subframes of a
  frame could hold something within a box (`subframes_meeting`). Each frame keeps a bound on its
  subframes' reach, so the answer is one search of the index widened by it, not a look at every
  subframe. "What is within 30 m", asked by someone on a city's open ground, no longer looks at
  every district in the city. Without the index, a scan gives the same answer.
- **Routes once per tick** (travel). The way from one region to another, through openings a body
  of a given size can use, is searched once per tick and shared: a crowd leaving one street for
  another searches the city's openings once, not once each.
- **Known places once per decision** (minds). Where a mind believes its openings lead is gathered
  once per decision, not once per thing it knows of.

In the commuting town, sight fell from 9.0 to 3.3 ms a tick. In the village bench, a tick at a
thousand agents fell from 51 to 40 ms.

**A gate for every district.** The city bench (`examples/city.rs`) now builds its city with the same
generator as the test, with attention on its first districts and people crossing all day. Its first
run found a gap: people in the scene who worked in a district without doors never left. Every
generated district now has its gate onto the city's ground; only districts meant for the scene
have doors on every home. A mind that knows no door into its workshop heads for the district it
is in, and leaves the scene at the gate.

**Not yet:**
- carrying held things across (one carrying something stays in the scene until they set it down);
- households' larders as food in the scene;
- the economy's and resources' facts for a person crossing.

**Rationale.** `docs/audits/world-scale.md` §10: the player walks out of the district, and its
people go back to being the city's, costing nothing; walks back in, and there they are.

---

## A-26 — Larders in the scene: pantries (2026-10)

**Edits:**
- Appendix A (Ruling 19: a household's larder crosses the seam as its people do)
- Vol. III Ch. 3 (deposits: deliveries)
- Vol. III Ch. 5 (households: the larder in the scene; work in the scene earns)
- Vol. I, *Conservation* (the larder's meals are counted across the seam, both ways)

**Change.**
- **A pantry** (Society, declared by the package). A household's larder may have a pantry: an
  entity, declared with the larder (`home = meals, pantry:id`), that the larder becomes in the
  scene. At the city tier it has no facts but its owner (Economy): the household, named by its
  home.
- **Setting the larder out.** When attention brings a home into the scene, its larder crosses as
  its people do (A-24):
  - Society marks the pantry crossing;
  - on the next tick Physical Reality stands it in the home, at the world's pantry size;
  - Resources makes it a deposit holding the larder's meals. It does not regrow. Each meal is the
    world's meal material, which must feed exactly what a meal feeds at the city tier (Law 14,
    validated);
  - the rhythm lets the count go.
- **Counting it back.** When the home is at the city tier again, the pantry is marked leaving. On the
  next tick Physical Reality and Resources retire it (A-25). The rhythm counts the larder back:
  the pantry's stock, plus any delivery not yet taken in.
- **One book-keeper.** The rhythm keeps every larder's count across the seam. It lets the count go,
  sends what is earned meanwhile to the pantry, and counts the pantry back in. No two systems ever
  write one larder in one tick.
- **Deliveries** (Resources). `resources.act.restock`, an intent open to anyone, carries whole units
  to a deposit. Deliveries in one tick add up, and the deposit takes them in on the next.
- **Work in the scene earns.** A resident in the scene standing at their work as their own hours
  end earns their household its meals, as a finished shift does at the city tier. With the pantry
  out, the meals are delivered to it.
- **Knowing one's pantry.** A resident crossing in knows their household's pantry: where it is,
  that it is theirs, how many meals it holds, and how filling a meal is. Those who are hungry fetch
  a meal from it and eat it, as anyone eats from a tree they know (A-16).
- **A household's things are its own.** A mind does not count as someone else's what its own
  household owns (A-19). People in the scene, who can see into their neighbours' homes, leave
  their neighbours' pantries alone.

**Measured.**
- Ashford's Lanes, a day in the scene: people eat from their pantries, a shift restocks one, and
  nobody passes 90% hungry. When attention leaves, every larder is counted back exactly
  (`crossing.rs`).
- In the generated town, those living in the scene districts eat at home in the scene
  (`commute.rs`).

**Not yet:**
- a meal picked but not eaten when its pantry leaves is not counted back;
- one carrying something stays in the scene until they set it down (A-25).

**Rationale.** Without this, a household whose home is in the scene could eat only at the city tier,
at home, where its people never are while the home is in the scene. Within two days the scene
would starve the people the city tier kept fed.

---

## A-27 — Integrity: things that break (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 (materials: what a structure bears) and §1.11 (bodies: integrity, breaking)
- Vol. III Ch. 2 (capability: strength)
- Appendix A (Ruling 9, built: damage proposed by anyone, applied by Physical Reality; Ruling 13:
  breaking is Physical Reality's)

**Change.**
- **Integrity** (Physical). How whole a thing or a walled place is, in hundredths of a percent.
  - Absent means whole, so only what has been damaged carries it.
  - Changes add up: damage and repair proposed in one tick both apply (Vol. III Ch. 12). The
    result is held within nothing and whole; whole again, it is no longer stored.
  - Open to any proposer (Ruling 9): a blow, a fire, a storm, a battle.
- **Breaking** (Physical). When a thing's integrity reaches nothing, Physical Reality breaks it in the
  next tick. Being broken is recorded, and only Physical Reality's own systems may record it.
  - **A thing becomes its wreckage.** It is the same entity, still its materials and still its
    owner's (Conservation). It lies flat, no taller than the world's wreckage height. It is no
    longer solid or opaque. What it held spills where it stood.
  - **A walled place becomes a ruin.** It is the same place, with its walls down. It is open to
    sight and to the sky (no longer sheltered), and its doors are broken open.
  - **A door breaks open:** both its faces broken, open, and no longer opaque.
  - What stood on a broken thing is no longer held up. A thing that cannot move by itself (a
    bottle on a table) falls as the thing breaks, taking the time gravity takes. A body that can
    move by itself is gravity's on the next tick (A-4). What stood on it is found by looking
    over the thing's place, so the answer is the same with or without the spatial index.
- **Strength** (Living). An organism's physical capability, 0–100% (Vol. III Ch. 2: "strength,
  speed, stamina as biological facts"). Declared per organism, or the world's default for its
  people. A resident crossing into the scene is given the default.
- **Striking** (Physical; Ruling 13). A decider proposes that a body strike a thing (`act.strike`).
  Physical Reality carries out the blow if the thing is within reach, and is made of something with
  a known weight. How hard it lands:
  - **Bare-handed:** the body's strength, times the world's `hand_strike_percent`.
  - **With a tool in hand:** a tool adds the world's `tool_strike_percent_per_kg` of strength for
    each kilogram it weighs. It never strikes harder than its own weakest material: a wooden
    club does not dent iron. The best thing in hand is used.
  - **What it does:** a blow no harder than the target's weakest material leaves no mark, and is
    refused, so the striker can notice. A harder blow costs the target integrity: how much
    harder, times the world's `strike_scale`, over its weight in kilograms. One blow is one
    action, the same at any tick length.

**In Ashford:**
- A packing crate stands in the yard with an apple in it, and a two-kilogram hammer lies by it.
- People are 40% strong; Carol, who hauls the harvest, 70%.
- Erin's fists cannot break the crate. Carol's can, in a few blows. Erin with the hammer can, in
  two. The broken crate lies flat and timber in the yard, and the apple rolls out.

**Not yet:**
- rebuilding what is broken;
- harm to those inside a ruin as it falls;
- breaking a body (harm to the living is Living's);
- deciders that choose to strike (A-30).

**Rationale.** `docs/audits/mutable-world.md` §3.1: the foundation fire, wind, and conflict all
propose to.

---

## A-28 — Fire (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 ("fire spreads because nearby materials satisfy ignition conditions": built)
  and §1.10 (environment: rain on what is open to the sky)
- Vol. III Ch. 2 §2.4 (harm: burns)
- Appendix A (Ruling 9: Physical Reality reports heat, Living Systems judges burns)

**Change.**
- **Burning** (Physical; only its own systems write it). How fiercely a thing or a walled place
  burns. Absent means not burning. On the world's fire cadence:
  - **Its course.** It grows toward what the thing's most flammable material allows, at the
    world's rate. It eats the thing's integrity: at full fierceness, the world's `burn_per_hour`
    from a hundred kilograms of fuel, more slowly by the cube root of how much more there is. A
    walled place's fuel is its walls, floor, and roof, at the world's `wall_thickness_cm`. With
    nothing left the fire goes out, and what burned is broken (A-27): wreckage, or a ruin. In the
    open, while humidity is at the world's `rain_above` or more, rain puts it out. A fire the
    world declares *kept*, a tended hearth's, neither grows, nor eats, nor dies down.
  - **Its heat.** It heats everything within its reach that it has a line to. Its reach is the
    world's `fire_reach_cm` at full fierceness, beyond its own extent, measured to the nearest
    edge of what it heats. It gives more the nearer and the fiercer it is. The line is drawn from
    the fire's middle (heat radiates from its whole body), and walls, opaque bodies, and rising
    ground stop it as they stop a gaze. Walls keep heat out until they burn themselves.
  - **What it holds and what holds it.** A fire inside a walled place heats its walls, as near as
    it is to the nearest of them or the ceiling. A walled place on fire heats everything inside it.
  - **Catching.** A thing that can burn catches when the heat it is given, times its flammability,
    reaches the world's `ignition_heat`. It catches at the world's `ignite_intensity`. Fires set to
    one thing in one tick burn as fiercely as the fiercest.
- **Heat exposure** (Physical, a report). How much heat each body that moves by itself is taking,
  written only where it changed.
- **Burns** (Living). Health lost at the world's `burn_harm_per_hour`, in proportion to heat
  exposure. Death by fire is chronicled as such.
- **Firelight** (Physical: sight). What burns can be seen, and so can everything in the place it
  burns in, whatever the daylight.
- **Setting fire** (Physical; Ruling 13). A decider proposes that a body set fire to a thing
  (`act.ignite`). It is carried out if the body has a flame to hand (something burning that it
  holds, or within its reach), and the thing is in hand or in reach and can burn. Otherwise it is
  refused.
- **Fires the world declares.** `[fires] thing = fierceness[, kept]`.

**In Ashford** (`tests/reference/tests/fire.rs`):
- The hay wagon is straw. Set alight against Alice's house, it takes the timber kitchen and the
  bedroom over it, which burn down to ruins open to the sky; the granite cellar does not burn.
  Bob, who has no mind of his own in that test to flee with, is burned.
- A kept fire in the hearth lights the kitchen at midnight, so Bob can see the table, and in half
  an hour sets nothing near it alight.
- Erin cannot set the crate alight with nothing burning to hand. She lights the torch at the
  hearth and sets the crate alight with it. The crate burns down to wreckage; its apple, too wet
  to burn, rolls out.
- Under rain, the burning hay wagon goes out; the kitchen table burning under the roof does not.

**Not yet:**
- smoke;
- fire warming the air (it reports heat to bodies; the climate's temperature is untouched);
- minds that flee fire or put it out;
- fire at the city tier.

**Rationale.** `docs/audits/mutable-world.md` §3.2.

---

## A-29 — Wind (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 (a structure fails because its material cannot bear the stress: built for
  wind) and §1.10 (storms the world declares)
- Vol. IV (the world package: `[storms]`)

**Change.**
- **Storms** (Physical; world data). A world may declare storms over its regions:
  `[storms] region = from hour, hours, wind cm/s`. While one blows, the region's wind is at
  least the storm's. Otherwise wind is the pressure gradient's, as before.
- **Wind load** (Physical). On the environment's cadence, every structure in the open bears the
  wind of the region it stands in. A structure is a thing made of something, or a walled place's
  outer walls. "In the open" means:
  - it stands in a place, not inside another thing that shelters it (an apple in a crate, a
    hammer in a hand); a ruin counts as a place;
  - no walled place is around it.

  The wind presses with its speed squared, over the world's `wind_force_divisor`, in the units of
  hardness. Where that exceeds what the structure's weakest material bears, the structure loses
  integrity: the world's `wind_damage_scale` per hour for each unit it exceeds by, more slowly by
  the cube root of how much of it there is, as fire eats (A-28). Breaking is as any breaking
  (A-27): a shed to its wreckage, a house to a ruin. What the house sheltered is in the open from
  then on.
- In calm weather nothing is looked at.

**In Ashford** (`tests/reference/tests/wind.rs`):
- A 40 m/s gale over the Vale presses at 50%. In four hours it brings down the timber shed and
  Alice's timber house, and breaks the packing crate. The granite manor, the iron hammer, and the
  glass bottle stand untouched.
- The kitchen table stands out the gale behind the kitchen's walls, and bears it only once they
  fall.
- A 25 m/s wind presses at about 20%: it scatters the straw of the hay wagon and leaves the shed
  without a mark.

**Not yet:**
- light, loose things blown about;
- wind fanning a fire and carrying it downwind;
- harm to people caught out in a storm;
- weather that brews its own storms.

**Rationale.** `docs/audits/mutable-world.md` §3.3: the user's "destroyed by winds".

---

## A-30 — Open ground (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.5 (movement between places: built for places without walls)
- Vol. IV (the world package: the loader marks places)

**Change.**
- **Place** (Physical; world data). That an entity is a place one can be in, rather than a thing:
  a climate region, a declared place, a room. The loader marks every one.
- **Open ground.** A place without walls (a garden plot, a street, a ruin) is walked into and out
  of anywhere along its edge, not only through its openings:
  - **Into it**, from the place around it: one walks to the nearest point of its footprint, or
    stands where one is if already over it. For a place with no size, one walks to where it is.
  - **Out of it**, into the place around it: one walks to the nearest point of its edge, or, for a
    place with no size, from its middle.
  - Each is chronicled as a crossing (`entered`, `left`), as a doorway is.
- **The way between places** is found breadth first, as before, now over three kinds of step:
  through an opening, into an open place, out of one. Openings come before edges, and openings in
  ascending id, so the answer is deterministic. Edges are taken only along the two lines of
  containers, up from where one is and down toward where one is going, so a search across a
  city's open ground does not wander into every street in it. Routes are remembered for the
  tick.
- A walled place is still entered only by its doors. A ruin (A-27) is open ground: what was a
  house is walked into where its wall stood.
- Before this, travel crossed only openings: two open places side by side, with no opening
  between them, had no way between them.

**In Ashford** (`tests/reference/tests/open_ground.rs`):
- Erin walks from the yard into a 4 m garden plot across its edge, stands within it, and walks out
  again.
- She walks to a spot in the yard with no size, and is in it.
- With the kitchen's front door shut and its walls standing, she has no way in.
- With its walls brought down, she walks into the ruin over the rubble, not through its doorway
  (narrowed for the test), and up to the table inside.

**Rationale.** `docs/audits/mutable-world.md` §3, slice 5.

---

## A-31 — Collapse and mending (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 (structures fail: what falls strikes; what is broken can be made whole)
- Vol. III Ch. 2 §2.4 (harm: struck by what falls)
- Appendix A (Ruling 9: damage and repair compose on one integrity)

**Change.**
- **Impact** (Physical; a report). When a walled place breaks into a ruin, every body inside it that
  moves by itself is struck, as by a fall from as high as the walls stood. The report carries the
  tick it happened. Living Systems judges it as it judges a fall (A-10), once per impact.
- **What it was** (Physical; only its own systems write it). Breaking (A-27) records what a thing
  or place was: its height, and whether it was solid, opaque, and walled.
- **Mending** (Physical; Ruling 13). A decider proposes that a body mend a thing (`act.mend`). It is
  carried out if all three hold:
  - the thing is damaged;
  - it is within reach, or it is the place the body stands in, or, for an opening, one of its faces
    is within reach;
  - the body holds a tool no softer than the thing's weakest material.

  Otherwise it is refused. Each go restores the world's `mend_scale` from a hundred kilograms,
  less, the more there is to mend (a walled place's walls, floor, and roof, as fire counts them).
  Mending and damage to one thing in one tick both apply: integrity is additive (A-27).
- **Made whole.** A broken thing whose integrity comes back whole is what it was again: as tall,
  as solid, as opaque, and walled if it was walled. A ruin made whole has the doors and windows
  that broke with it whole again, opaque, and able to be shut. Found among what changed, as
  breaking is.

**In Ashford** (`tests/reference/tests/mending.rs`):
- The kitchen comes down on Bob. He is struck as by a 2.8 m fall, and hurt. Dave, upstairs, is
  not in the kitchen and is untouched.
- Carol smashes the crate. With bare hands she mends nothing. With the iron hammer, a few goes
  make it whole: 40 cm tall and solid again, with nothing left to mend.
- Bob, in the kitchen ruin with the hammer, rebuilds it (with a quicker `mend_scale`). It is
  walled again, its front door is whole, and he can shut it.

**Rationale.** `docs/audits/mutable-world.md` §1 (composition: "fire consuming a wall that
soldiers are repairing").

---

## A-32 — Fire in the air (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 (environment: fire warms the air, fills it with smoke, and is fanned by wind)
- Vol. III Ch. 2 §2.4 (harm: smoke)

**Change.**
- **Warmth.** A fire inside a walled place warms its air: the world's `fire_warmth_per_hour`
  centidegrees at full fierceness for a hundred cubic metres, less in a bigger place. A kept
  fire (A-28) warms as any other.
- **Smoke** (Physical; only its own systems write it). How thick the smoke is in a walled place:
  - It rises from every fire burning there that is not kept, by the world's `smoke_per_hour` at
    full fierceness for a hundred cubic metres. A kept fire's smoke goes up its chimney.
  - It clears by the world's `smoke_clear_per_hour` share of itself, and as much again for each of
    the place's openings that stands open.
  - In the open, or once a place's walls fall, there is none.
- **Smoke and sight.** In a smoky place one sees only as far as the smoke lets one: at full smoke,
  not at all.
- **Smoke and breath** (Living). Health is lost at the world's `smoke_harm_per_hour`, in
  proportion to the smoke where one stands, counted with burns.
- **Fanning.** In the open, the wind fans a fire: at the world's `fire_fan_wind_cm_s`, to twice
  what its fuel alone allows (never past full), and it carries the fire's heat twice as far.

**In Ashford** (`tests/reference/tests/fire_air.rs`):
- Six hours of a kept hearth warm the kitchen by more than 3 °C, with no smoke.
- The kitchen table, set alight, fills the kitchen with smoke, thicker with the doors and window
  shut than open.
- At midnight in the shut-up kitchen, Bob sees the lamp by firelight, then loses it in the smoke,
  and is hurt breathing it.
- The burning hay wagon stays short of full in the Vale's breeze. A 10 m/s wind fans it to full.

**Not yet:**
- warmth carried from room to room, and smoke drifting through open doors to the next room.

**Rationale.** A-28's "not yet": smoke, and fire warming the air.

---

## A-33 — Wind that moves things, and gales (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 (loose things blown about) and §1.10 (storms the weather makes)
- Appendix A (Ruling 13: a push is anyone's to give)

**Change.**
- **A push** (Physical; Ruling 13). A proposal that a thing move by `[x, y]` centimetres in its
  place's frame (`act.push`). It is open to anyone (the wind, a shove), and pushes in one tick add
  up. Physical Reality carries it out last of all it does in a tick, then clears it. A thing is
  not pushed if, that tick:
  - someone took hold of it;
  - it broke or fell;
  - it is already moving (it finishes that move first).

  In a place with a size, a push stops at the place's edge.
- **Carried off by the wind.** On the environment's cadence, a loose thing (one that is not solid)
  standing in the open is carried downwind if the wind's pressure exceeds what its weight holds
  (the world's `wind_hold_per_kg` for each kilogram). It is carried the world's
  `wind_carry_cm_per_hour` per hour, scaled by how far the wind overwhelms its hold:
  - downwind means toward the region the wind blows to, where both have a place in a common
    frame;
  - otherwise it goes a gust's way, drawn from the thing's own random stream.
- **Gales from deep lows** (Physical; world rules). Where a region's pressure falls more than the
  world's `gale_below` decapascals under normal, its wind rises by the world's
  `gale_per_decapascal` cm/s for each decapascal deeper. It is the same as a declared storm's
  (A-29): the wind is at least the gale's.

**In Ashford** (`tests/reference/tests/wind.rs`):
- In a 40 m/s gale the torch is carried more than 10 m across the yard, and stays in it. The crate,
  solid, does not move.
- A shove given the bottle moves it exactly as far as it was pushed, and the push is spent.
- With weather that swings hard and a low gale threshold, a gale brews within two days with no
  storm declared. With Ashford's own weather, in the same two days, none does.

**Not yet:**
- people pushed or knocked down by the wind;
- fire carried downwind as embers;
- storms and fire at the city tier.

**Rationale.** A-29's "not yet": light, loose things blown about, and weather that brews its own
storms.

---

## A-34 — The calendar: seasons, the sun's year, and the moon (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 (*Time and Change*: sunrise, sunset, and seasons built; the moon)
- Vol. IV Ch. 2 §2.2 (world rules: the year, latitude, tilt, seasons, the moon)
- Vol. IV (the world package: `[latitude]`)

**Change.**
- **The year** (world rules). A world declares how many days its year lasts (`year_days`; zero
  means no year) and how many days after the spring equinox it begins (`start_day`). The sun's
  declination follows the year as a sine of the world's axial tilt.
- **Latitude** (Physical; world data). How far north a place lies, in hundredths of a degree. A
  place declares one in `[latitude]` or inherits it down the nesting; else it lies at the world's.
- **The sun.** Its height is `sin h = sin φ sin δ + cos φ cos δ cos(hour angle)`. Open ground is
  lit at the world's peak times that height while the sun is up. So days lengthen toward
  midsummer and shorten toward midwinter, noon stands lower in winter, and the far north has
  midnight sun and polar night. With no year, at latitude zero, this is exactly the day the
  engine had before.
- **The seasons' warmth** (Physical). Every climate open to the sky warms toward midsummer and
  cools toward midwinter by the world's `seasonal_amplitude_centi_c`, warmest
  `seasonal_lag_days` after midsummer. It is reversed south of the equator, and damped by exposure
  and thermal mass as the day's swing is. Like the day's swing, each step proposes the change in
  the seasonal level, so it is exact at any tick length. A region's declared temperature is its
  air on the world's first day.
- **The moon.** A month of `lunar_month_seconds`; zero means no moon. It begins at
  `moon_phase_at_start`. Its light at night is the world's `moonlight_peak` times how much of it
  is lit times its height. It rises later each night, and a full moon stands opposite the sun.

**In Ashford** (52° north, a 365-day year, beginning a month after the spring equinox, at new
moon; `tests/reference/tests/climate.rs`):
- The noon sun stands 49.6° high and lights open ground at 76% of its peak; the rooms by their
  openings, in proportion.
- The first day has fourteen hours of daylight; midsummer, two months on, sixteen and a half.
- A year of seasons warms the Vale 7 °C by three months in and cools it 7 °C below its start by
  nine; granite Highmoor answers less; a year on, both are back where they began. The same at
  hour ticks and ten-minute ticks, exactly.
- Under a full moon at midnight Erin can make out the apple tree across the yard; under the new
  moon she cannot.
- Dawn comes just before five: the hungry courier sets off for the tree an hour earlier than he
  did when every day was an equinox.

**Found:** a body lying down still sees. Before, dawn and the end of the sleeping hours fell
together; now a late-April dawn shows a resting mind what it wants, and it gets up. That belongs
with perception and is left to it. *(Settled by A-40: a body at rest has its eyes shut.)*

**Rationale.** `docs/audits/environment.md` §4, slice 1.

---

## A-35 — The sky: the day's curve, cloud, rain and snow, and fog (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 (environmental state: cloud, precipitation, fog; the day's temperature)
- Vol. III Ch. 1 §1.11 ("thick fog limits visibility": built)
- Vol. IV Ch. 2 §2.2 (world rules), Vol. IV (the world package: `[humidity]`)

**Change.**
- **The day's curve.** A world declares when in the day its air is coldest and when warmest
  (`diurnal_coldest_seconds`, `diurnal_warmest_seconds`). The swing rises straight from the one to
  the other and falls straight back. At midnight and noon it is the curve the engine had.
  A climate's declared temperature is the coldest of its day, as before. It begins as far above
  that as the curve stands at the first moment.
- **Humidity normals** (world data). A climate may declare its normal humidity in `[humidity]`;
  else it is the world's baseline. Its weather departs from that normal as before. This is *the
  air's* humidity.
- **Cloud** (Physical; written by the sky). Cloud forms as the air's humidity passes the world's
  `cloud_above`, and the sky is overcast by `rain_above`.
  - It keeps out `cloud_shade` of the light: of the region's own sky, and of a room's, through its
    openings.
  - It holds back `cloud_damp` of the day's swing: cooler afternoons, milder nights. The sky
    proposes the change in that share between its step's start and end, against the cloud it
    committed last, so the share telescopes exactly however cloud comes and goes.
- **Rain and snow** (Physical; written by the sky). Rain falls from air at `rain_above` or more:
  `rain_per_hour` with the air saturated, less in proportion below that. At `snow_below_centi_c`
  or colder, it falls as snow. Both are rates in hundredths of a millimetre an hour, absent when
  nothing falls. Fire in the open is put out by either (A-28 now reads them, not the humidity).
- **Humidity near the ground.** The humidity fact is now the air's humidity, raised by
  `humidity_per_degree` for each degree the air stands below its day's mean, and lowered for each
  degree above it. The day's mean takes cloud's share into account. So dawns are damp and
  afternoons dry.
- **Fog** (Physical; written by the sky). Fog forms where the humidity near the ground passes
  `fog_above` in air stiller than `fog_wind_below_cm_s`: thicker toward saturation, thinner as
  the wind rises. It lies over open ground, not in walled rooms.
- **Thick air and sight.** How clear the air is in a place is what its smoke leaves (A-32) times
  what its fog leaves. One sees only as far as the air about one lets one, and what is in thick
  air is seen only from as near.

**In Ashford** (coldest at five, warmest at three; Highmoor 75% humid, Southfen 80%;
`tests/reference/tests/sky.rs`):
- At 85% the Vale's sky is three-quarters cloud, and noon is 44% as bright as a clear noon.
- Under a full overcast the day swings 2 °C, not 5.
- With cloud coming and going over ten days, the Vale's air is at every step exactly its start,
  plus the day's swing since, less the share of it the cloud then holds back.
- At 95% it rains 5 mm an hour; declared at five below, the same air snows.
- At 86%, still Southfen lies under fog at dawn, lifted by three.
- At six in a damp dawn Erin cannot see the apple tree across the yard for fog, though it is
  light enough; by noon she can.
- With the coldest hour at dawn, Erin now feels the cold toward dawn, not in the late evening,
  and by then it is light enough to see the way in.

**Rationale.** `docs/audits/environment.md` §4, slice 2.

---

## A-36 — Wind and fronts: weather between climates (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 ("wind flows": wind from the weather; weather carried between climates)
- Vol. IV Ch. 2 §2.2 (world rules: `wind_per_decapascal` replaces `wind_gradient_divisor`;
  `climate_mixing_seconds`, `front_distance_cm`)

**Change.**
- **Wind from pressure at sea level.** Before, wind blew down the difference in neighbouring
  climates' raw pressure. Pressure falls with height, so the wind always blew uphill, toward the
  moor, however the weather stood. Now each climate's pressure is brought to the datum (its
  elevation times the world's pressure fall per metre, added back) before neighbours are
  compared. Height alone moves no air; the wind follows the weather, toward the lowest
  neighbour. Its speed is the world's `wind_per_decapascal` for each decapascal of difference.
  Declared storms and brewed gales (A-29, A-33) still set a floor.
- **Fronts.** Each climate's weather (its departures from normal temperature, humidity, and
  pressure) is mixed, before each step of its weather, with its neighbouring climates'. Each
  neighbour weighs in:
  - by the step's share of `climate_mixing_seconds`: weather everywhere slowly mixes;
  - and, if the neighbour's wind blows toward this climate, by how far that wind travels in the
    step, against `front_distance_cm`: weather rides the wind downwind.

  The mix is a weighted mean, the climate's own weather at weight one, so it is stable at any
  step length. Only departures cross: each climate keeps its own normal, so a moor stays colder
  than a valley, but a cold spell on the moor is felt downwind.

**In Ashford** (3 cm/s per decapascal; weather mixing over two days; neighbours 20 km apart;
`tests/reference/tests/fronts.rs`, `climate.rs`):
- With steady weather, pressure still falls with height, Southfen to the Vale to Highmoor, and
  the air is still everywhere.
- A low 200 decapascals deep over Highmoor draws the Vale's air toward it at about 6 m/s, and the
  moor is calm. A high blows off the moor into the Vale instead.
- A spell 6 °C cold over the moor under a high rides the wind down into the Vale, taking more than
  2 °C off its weather within half a day, and reaches the fens beyond, less. Under a low the wind
  blows the other way, and the Vale feels less than 1 °C of it, by slow mixing alone. With
  fronts turned off, neither feels any.
- With Ashford's own weather the wind is now a breeze of a few metres a second that comes and
  goes, not a steady 35 cm/s up toward the moor. Tests that need still air declare steady
  weather.

**Rationale.** `docs/audits/environment.md` §4, slice 3; and the wind's old flaw, found in the
building.

---

## A-37 — Air between places: warmth and smoke through openings (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 ("smoke diffuses": built between places; air quality outdoors)
- Vol. IV Ch. 2 §2.2 (world rules: `opening_air_speed_cm_s`, `draught_percent`,
  `open_air_height_cm`)

**Change.**
- **Air through openings.** Air passes both ways through every opening that stands open (a
  broken door stands open), as fast as the opening's area times the speed air moves through it:
  the world's natural draught (`opening_air_speed_cm_s`), and, where the opening gives onto open
  air, `draught_percent` of the wind outside. An opening's two faces are one way.
- **What holds air.** A walled place holds its floor area times its height. Open ground with a
  size (a yard, a ruin) holds its area to the world's mixing height (`open_air_height_cm`). Anything
  else is open air: too much to change, and it gives back only its own air.
- **Warmth.** A walled room's air follows the outside through its walls, as before (A-5). It now
  also trades with whatever lies beyond each open opening: the next room's air, or the open air's.
  This is solved as a weighted mean in the room's own air, stable at any step length. An open
  door lets the night in; a hearth's warmth climbs an open stair.
- **Smoke.**
  - It rises from what burns, except a kept fire (A-32), into the walled place around it, or into
    the open ground the fire stands on if that holds air.
  - It leaks away at `smoke_clear_per_hour`, now through walls and roof only. Over open ground the
    wind also carries it off, at the wind's speed across the place.
  - It trades through every open opening: the difference in smoke, times the air that passes,
    shared out by how much air each side holds. So it drifts up a stair, out of a door, and in at
    one from a smoky yard.
  - Open ground's smoke shortens sight and harms breath, as a room's does (A-32, A-35).
- Before this, an open opening only cleared a room's smoke faster, as if onto clean air, and
  carried no warmth at all.

**In Ashford** (5 cm/s draught; a tenth of the wind; smoke over open ground mixing ten metres
up; the yard now declares its size, 40 m × 30 m; `tests/reference/tests/air.rs`):
- Alice's kitchen begins the night at 20 °C. With its front door and window open, two hours on it
  is much nearer the yard's air than when shut up behind its timber walls.
- The house shut up, a kept fire in the hearth warms the bedroom up the stair and the cellar down
  it.
- The burning table's smoke drifts up the stair to the bedroom, thinner than in the kitchen.
- The packing crate burning in the still yard smokes the air over it, and the smoke drifts in at
  the kitchen's open door. Shut up, the house keeps it out. A 5 m/s wind carries most of it off.
- Erin, making for the kitchen she remembers warm, finds its door has let the night in, and
  climbs to the bedroom, where the warmth went.
- With the doors shut, the kitchen's and the hall's swing is smaller than before, and later: each
  also trades air with heavier rooms up and down its stairs.

**Not yet:**
- smoke production by how much is burning, not only how fiercely: a burning house smokes no more
  than a torch at the same fierceness;
- a plume: smoke over open ground is well mixed across the place, not thick downwind of the fire
  and thin upwind.

**Rationale.** `docs/audits/environment.md` §4, slice 4; A-32's "not yet".

---

## A-38 — Wet and snow (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 (environmental state: soil moisture; snow cover)
- Vol. III Ch. 1 §1.9 (fire: wet things burn less readily)
- Vol. III Ch. 2 (body heat: wind and wet hurry a body toward the air's temperature)
- Vol. IV Ch. 2 §2.2 (world rules: `soak_per_mm`, `ground_drying_per_hour`,
  `snow_melt_mm_per_degree_hour`, `wetting_per_hour`, `drying_per_hour`, `dry_by_heat_per_hour`;
  Living's `chill_wind_cm_s`, `wet_chill_percent`)

**Change.**
- **Ground water** (Physical). How wet a climate's ground is. Rain soaks in at `soak_per_mm` for
  each millimetre, and so does melting snow. It dries at `ground_drying_per_hour` times how dry
  the air is, twice as fast in full sun, faster in wind. It does not dry under snow.
- **Snow cover** (Physical). Snow builds ten times as deep as the water in it, and melts at
  `snow_melt_mm_per_degree_hour` for each degree above freezing, into the ground.
- **Wetness** (Physical; open to anyone, as a bucket may be thrown). How wet a thing or a body
  is. Whatever stands in the open while rain or snow falls soaks, at `wetting_per_hour` in the
  world's hardest rain, less in lighter. Everything wet dries as the ground does, slowly indoors
  in a room's still, dim air.
- **Wet things burn less readily.** A thing burns as its most flammable material, less as much as
  it is wet. It catches from heat only by that, and a fire on it grows only to that. A flame
  lights only what will burn at least as fiercely as a fire begins. A fire's heat dries what it
  reaches, at `dry_by_heat_per_hour` at full heat, so a soaked thing beside a fire holds out
  until it is dried.
- **Chill** (Living). A body is pulled toward the air's temperature faster:
  - by one again for every `chill_wind_cm_s` of wind on it. Living finds that wind as it finds the
    air: walking up what encloses the body to its climate's wind, none once a wall stands
    between, and only the declared share through a walled place open to the sky. (A per-body
    wind report was tried first and dropped: the breeze changes every step, and a write for
    every body outdoors every step is churn the world does not need.)
  - by `wet_chill_percent` when soaked through, in proportion as it is wet.
- Ground water, snow depth, and wetness are levels that vanish at zero. Any number of systems
  may add to them or take from them in one tick: fire dries, rain wets.

**In Ashford** (`tests/reference/tests/wet.rs`):
- Three hours of 5 mm rain wet the Vale's ground by about 45%. On a dry spring day, half-wet
  ground dries even at night, and faster under the sun.
- Declared five below, saturated air lays 10 cm of snow in two hours, and the frozen ground takes
  none of it. Twenty centimetres on the spring ground melt by noon, into the ground.
- An hour of rain soaks the packing crate in the yard. The kitchen table stays dry.
- Dry, the crate catches from the torch burning beside it. Soaked, it is still drying in the
  torch's heat forty minutes on.
- Three hours in a 20 m/s gale leave Erin's body colder than in still air, and colder still in
  the gale and rain. Bob, in the kitchen with the house shut up, feels no wind. (With the front
  door open he would be colder too, for the draught it lets in: A-37.)
- Ashford's ordinary breezes of a few metres a second chill a body only a little: a gale of
  20 m/s doubles how fast it takes the air's cold.
- That little is enough to move people, through what they believe of the house's rooms (A-37).
  Gwen, cold at dawn in a kitchen whose open door let the night in, goes down to the cellar she
  remembers warmer; the love test keeps her ignorant of the cellar so she stays with Finn.
  Erin, cold in the bedroom on her second morning, goes out into the yard the sun has warmed
  first.

**Not yet:**
- deep snow slowing those who walk through it;
- rivers, ponds, and flooding (A-39).

**Rationale.** `docs/audits/environment.md` §4, slice 5.

---

## A-39 — Water: ponds, rivers, the sea, floods, and ice (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.10 (water level, rivers that flood, tides: built)
- Vol. III Ch. 1 §1.11 ("water flows downhill": built for the world's declared watercourses)
- Vol. IV Ch. 2 §2.2 (world rules: `tide_amplitude_cm`, `tidal_period_seconds`,
  `ice_mm_per_degree_hour`, `wade_depth_cm`, `ice_bears_mm`)
- Vol. IV (the world package: `[water]`, `[drains]`, `[floods]`)

**Change.**
- **Water** (world data). A place may be water:
  `[water] place = pond|river|sea, depth cm, bank cm, land drained m², drain seconds`. These
  give its depth at rest, the depth at which it tops its banks, the land whose rain runs into
  it, and the time constant at which what stands above its rest flows on. `[drains] a = b`: a
  pond or river flows on into `b`. `[floods] a = places`: the low ground it covers over its banks.
- **Water level** (Physical), on the environment's cadence:
  - The sea stands at its depth plus the tide: `tide_amplitude_cm` over `tidal_period_seconds`,
    all of it at new and full moon, half at the quarters. What runs into the sea is lost in it.
  - A pond or river takes in the rain on its own face, all of it. It takes in the rain and
    melt-water on its land in proportion to how wet the land already is (A-38): dry ground
    drinks it.
  - It takes in whatever flows on into it, upstream first, within the same step. What stands
    above its rest flows on downstream at its drain time.
  - Each level is solved at the step's own end, so a river stands as high at any tick length.
- **Floods** (Physical). Water above its banks covers the low ground it floods, as deep as it
  stands above them. The flood rises and drains as fast as the water drains.
- **Ice** (Physical). Ice forms on water at `ice_mm_per_degree_hour` for each degree of frost,
  and melts at the same rate for each degree of thaw.
- **Water stands in the way.** No one walks into water, or onto flooded ground, deeper than
  `wade_depth_cm`, unless ice at least `ice_bears_mm` thick bears them. The way is found around
  it if there is one, and reported blocked if not.
- **What stands in water is soaked** (A-38): in water not frozen over, or on flooded ground.

**In Ashford** (a pond in the manor grounds, a metre deep; the River Brack past the mill, knee
deep, 300 hectares draining into it, running to the sea; the sea at the harbour, five metres at
mean tide; `tests/reference/tests/water.rs`):
- At new moon the sea rises and falls three metres, high again 12 h 25 min after the first high.
  At the first quarter it rises and falls half as much.
- An hour of the hardest rain on land already soaked swells the Brack over its banks and floods
  the far field more than half a metre deep. On dry land the ground drinks it, and the Brack
  stays in its banks. The same at minute and ten-minute ticks.
- Erin, setting out for the far field a kilometre off, finds no way into it while it is flooded.
  Dry, she gets there.
- The manor's pond is too deep to wade. In a hard frost it freezes thick enough by midday that
  Erin walks out onto it.
- The hammer, dropped in the Brack on a dry day, is wet through at once.

**Not yet:**
- people and things carried off by floodwater, or drowning;
- boats that float on the water's level;
- walking slowed by deep snow and shallow water.

**Rationale.** `docs/audits/environment.md` §4, slice 6.

---

## A-40 — Shut eyes: what gets a sleeper up (2026-10)

**Edits:**
- Vol. III Ch. 1 (*What is in view*: a body at rest has its eyes shut; a place seen whole)
- Appendix A (Ruling 15: Physical Reality reads the rest intent too)
- Vol. II Ch. 4 (*The Layer as Built*: a sleeper knows only what it feels and holds; *Absence can
  be seen* only in a place seen whole)
- Vol. V Ch. 9 §9.3 (minds that sleep: what gets a sleeper up)

**Change.**
- **A body at rest has its eyes shut** (Physical). Sight reads Living Systems' rest intent,
  `living.act.rest`, by its id, as it already reads the sight range. A body asked to rest that is
  still sees nothing. *Still* is the test Living uses to judge a rest (Ruling 15): not moving,
  and not setting off. Only what the body holds stays in its view, known by touch (A-13). When it
  gets up, or is moved, its eyes open and it sees again at the next sight step. In this engine,
  resting and sleeping are one state.
- **What it saw becomes memory** (Information). Nothing new is needed here. As a sleeper's view
  empties, each belief about what it saw is stamped with the moment it was last seen, as for
  anything lost from sight. It grows no fonder of those it cannot see, though a longing is still
  eased by lying in the same place as its object. It still feels itself: the cold, hunger,
  fatigue, its needs, and what it holds.
- **Light alone wakes no one.** No felt fact of light is added, and first light gets no one up.
  The world's sleeping hours (A-18) say when its people rise. A sleeper woken by the light would
  be up before four at Ashford's midsummer, two hours before the town's hours end. A winter
  morning is no different: a sleeper gets up at the end of the hours, in the dark. Out of the
  sleeping hours, a sleeper gets up once rested, as before.
- **In the sleeping hours, only what presses gets a sleeper up** (Minds). That is the cold,
  hunger, a longing or a craving, a routine, or work, each scored as always against staying put.
  A want or curiosity does not: they fill the hours when nothing presses (A-19). This holds even
  for a want the sleeper already knows of. Out of the sleeping hours, anything worth more than
  staying put gets a sleeper up, as before. Lying down is unchanged: a want may still keep a mind
  up past its bedtime.
- **A place seen whole** (Physical). With what is in view, Sight now publishes the place a body
  took in whole, if any: `physical.sense.seen_whole`, written only when it changes. A body takes
  in the place it stands in when:
  - the place is lit;
  - every corner of its floor is within the body's sight, as far as the air there lets one see;
  - nothing in it could hide anything: no wall, nothing opaque, no rising ground.

  A place with no size has no corners to reach, and is never seen whole. Eyes shut, a body takes
  in nothing. The place stays in the body's view when lit (A-11), as before.
- **Absence needs a place seen whole** (Information). A mind concludes that a thing it believed
  here is not here (A-11) only when the place Physical says it took in whole is the place it
  stands in. The marker names the place the view was taken in, so the first view after a mind
  walks somewhere new, still taken from where it came from, decides nothing.

**In Ashford** (`tests/reference/tests/needs.rs`, `perception.rs`):
- Erin lies down in the yard in the small hours of a late-April night, sleeps through a sunrise
  just before five, and gets up at six, rested. The test runs on Ashford's own calendar again,
  not at the equinox.
- The green bottle lies five metres from her in the dawn light. It is glass, which she likes.
  Asleep, she has nothing in view, and she sleeps on. Up at six, she goes in from the cold, then
  comes back out for the bottle. (Before, it got her up at twenty to six.)
- Put into her hand as she sleeps, the bottle is known by touch: glass, and no one's. Still she
  sleeps till six, and then claims it.
- At noon Erin sees the bottle, goes into the kitchen, and comes back out to the doorstep, where
  the house hides it. She still believes it lies in the yard. In the kitchen, which she takes in
  whole, Dave still sees that Bob is gone.

**Found:**
- **Absence ignored what stands in a place, and when the view was taken.** A-11 let a mind
  conclude a thing was gone whenever its place was lit and in view. The yard has had the house
  standing in it since A-37. Erin, back on the doorstep for the bottle she saw at dawn, could not
  see it round the house, and let it go. She never went for it again. Worse, the first view after
  any step through a door was the one taken from the far side: from the kitchen she had seen the
  yard through the doorway, so the yard counted as in view. Before this amendment, Erin claimed
  the bottle at dawn from where she lay, and neither fault showed. The wants test caught it.
- **Love takes a day longer.** Finn and Gwen used to watch each other for an hour from where
  they lay, from first light until the sleeping hours ended. That no longer counts as time
  together, and they become lovers on the third day, not the second (`love.rs`).

**Not yet:**
- noise, pain, smoke or fire waking a sleeper; nothing yet sends a mind away from a fire;
- waking with the sun, for a world whose people do;
- lying awake: resting with open eyes;
- absence on open ground or in a big hall: a mind there never concludes a thing is gone, even when
  the spot where it lay is in plain view. Beliefs hold places, not spots.

**Rationale.** A-34 found that a body lying down still saw. When the calendar moved Ashford's
dawn before the end of its sleeping hours, the first light showed a resting mind something it
wanted, and it got up for a bottle.

---

## A-41 — Taking from deposits: tools, effort, grade, and renewal (2026-10)

**Edits:**
- Vol. III Ch. 3 §3.4 (yield as a function of grade, capability, and method; the four renewal
  classes: built)
- Vol. III Ch. 1 §1.9 (a tool no softer than what it works)
- Vol. IV Ch. 2 §2.2 (world rules: `bare_hand_hardness`), Vol. IV (deposits' `renewal`, `floor`,
  `grade`, `difficulty`, `grows`; `[ground_water]`)

**Change.**
- **One way to take from a deposit.** A go at a deposit (`act.pick`, A-15) needs, within reach, a
  hand or a tool fit for what the deposit yields. A bare hand works only what is no harder than
  the world's `bare_hand_hardness`: fruit, berries, wheat. Anything harder needs a tool no softer
  than it: a hammer for ore, an edge for timber. With nothing fit, the go is refused.
- **Effort** (Physical; a report). Each go's effort is reported with it: the hardest blow the body
  lands with what is fit, as a strike is judged (A-27), by strength and the tool's weight, never
  harder than the tool.
- **Work** (Resources). A deposit's `difficulty` is the work each unit takes at full grade. Its
  `grade` divides that, so a seam half as rich takes twice the work. Each go's effort goes toward
  the next unit, whoever's go it is. The go that completes it takes one unit from the stock into
  that hand. One go yields one unit at most. A deposit with no difficulty yields a unit to every
  go, as before.
- **Renewal** (Resources; world data per deposit):
  - *nonrenewable*: never regrows;
  - *renewable* (the default, as before): regrows at its daily rate up to its cap;
  - *threshold*: as renewable, until it is taken below its `floor`; then it has collapsed, and
    never regrows again;
  - *flow*: refills to its cap at each renewal, and cannot be stockpiled at the source.
- **Growing things** (Resources). A renewable or threshold deposit that `grows` regrows only as
  warm as it stands between its two temperatures, and only as much as its climate's ground holds
  the water it needs (A-38).
- **Ground water at the start** (world data): `[ground_water] climate = level`. Ground that
  began bone dry would grow nothing until it rained.

**In Ashford** (`tests/reference/tests/extract.rs`):
- An iron seam (grade 60%) and a poorer copper seam in the mine's deep gallery, nonrenewable;
  a stand of timber in the wood (threshold: floor ten logs; it grows); wheat on the farm (it
  grows); iron ore, copper ore, and wheat as materials. Bare hands work what is no harder than
  soil. The Vale's ground begins at 40% wet, Highmoor's 60%, Southfen's 80%.
- Carol cannot work the iron seam with her hands. With the hammer, a few blows bring a lump of
  ore away, and the seam holds one fewer.
- Carol works it in fewer blows than Erin; at half the grade, it takes her about twice as many.
- Three days on, the seam has not regrown.
- Felled to its floor, the stand regrows a log a day. Felled one below it, it never regrows.
- In three spring days the wheat ripens by half again. In frost, or on dry ground, it does not
  grow at all.
- Brambles declared to flow are back to their full eight at the next renewal.

**Not yet:** digging the ground (A-42).

**Rationale.** `docs/audits/resources.md`, slice R2 (built first: digging is taking from a pit).

---

## A-42 — Ground and quarries (2026-10)

**Edits:**
- Vol. III Ch. 1 §1.9 (what the ground is made of)
- Vol. III Ch. 3 §3.4 (deposits opened by digging)
- Vol. IV Ch. 2 §2.2 (world rules: `pit_half_width_cm`), Vol. IV (`[ground]`, `[quarry]`, the
  `paved` flag)

**Change.**
- **Ground** (Physical; world data). `[ground] place = material`: what a place's ground is made
  of, inherited down the nesting as climate is. The Hill's mine mouth stands on the Hill's
  granite.
- **Paved** (Physical; world data). A place flagged `paved` is not to be dug: cobbles, floors.
- **Digging** (`act.dig`, open to any decider; Physical carries it out). A body digs the ground
  where it stands, if there is ground to dig. There is none:
  - inside a thing, or in a walled place;
  - in water, on flooded ground, on ice;
  - on paved ground;
  - where nothing up the nesting declares a ground.

  It digs at a pit within its reach that is not dug bare, the lowest first. If there is none, it
  breaks a new pit where it stands: a thing in the place, `pit_half_width_cm` to either side,
  made of the ground's material. Either way the dig is a go at that pit, as a pick is (A-41): a
  hand or a tool fit for the ground, and its effort reported.
- **Quarries** (Resources). A pit not yet a deposit becomes one. It is nonrenewable, holding
  what the world declares for a pit in that ground: `[quarry] material = units, unit size, work
  per unit`. The dig that breaks ground yields nothing of itself. When a pit is dug bare it stays
  where it is, and the next dig beside it breaks a new one.

**In Ashford** (the Vale and Southfen on clay, Highmoor and the Hill on granite; granite pits hold
5,000 blocks of ten kilos at hard work; clay pits a hundred thousand spadefuls by hand; the
Lanes cobbled; `tests/reference/tests/quarry.rs`):
- At the mine mouth, Carol's first blow with the hammer breaks ground, and the pit gives a granite
  block every few blows after. The pit holds one fewer.
- Without a tool she cannot break the granite, and no pit is dug. Erin, in the yard, digs clay by
  hand: the first dig breaks ground, the second brings it up.
- Nothing is dug in the kitchen, on the Lanes' cobbles, or in the Brack.
- In pits that hold two spadefuls, the third spadeful comes from a second pit beside the first.

**Rationale.** `docs/audits/resources.md`, slice R1: "stone anywhere the ground is stone."
