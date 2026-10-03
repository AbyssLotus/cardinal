# Agents: What the Spec Requires, What Exists, and a Build Plan

> **Status (2026-10-02): phases 1–3 are built** — A-7, A-8, A-9; gates in
> `tests/reference/tests/{perception,minds}.rs` and `kernel/tests/pairs.rs`. See §7 for what
> building them changed in this plan.

*2026-10-02, on branch `feat/agents` (based on `feat/overlapping-regions`, PR #22). This plan
covers minds that act on what they believe: the sweep's M13 (decision systems) and M14 (the
information layer). The sweep is `docs/audits/system-sweep.md`. Every step starts with its spec
amendment, and every gate is a scenario in Ashford, the reference city.*

---

## 1. Bottom line

**The spec already decides almost everything about agents. What is missing is the
machinery.** Five chapters agree on one pipeline:

```text
Reality ─► Observation ─► Belief / Memory ─► Decision ─► Intent ─► Reality
 (Physical,   (what one      (what one          (a mind's     (Ruling 13: the world
  Living)      could sense)   holds, aging)      choice)       decides what happens)
```

No stage may be skipped:
- **Ruling 4:** biology never decides.
- **Vol. II Ch. 4:** decisions operate on information, never on reality.
- **Vol. V Ch. 9:** a mind sees its beliefs and nothing else.

The engine already owns the last arrow. Intents, travel, doors, reach, and refusal all work,
and the kernel refuses anyone who writes around them. It has none of the middle: nothing
perceives, nothing believes, nothing decides.

The plan builds the middle in three steps:
1. **Pairwise facts,** so a belief can be stored at all.
2. **Perception and belief,** the information layer.
3. **Minds:** deterministic, utility-based decision systems that read beliefs only.

The read-beliefs-only rule is kernel-enforced, not left to convention.

---

## 2. What the spec requires

| Source | Requirement |
|---|---|
| Vol. II Ch. 4 (Information), invariants 1–10 | Reality and information are separate layers. Observation reads reality and never changes it. Knowledge is per observer. Memory goes stale. Beliefs may be wrong. **Decisions operate on information, not reality.** Every piece of information records where it came from. Information spreads only through explicit mechanisms. |
| Vol. I Ch. 2 §8, §13–15 | A decision belongs to the actor's epistemic world: needs, goals, beliefs, predicted outcomes, risk. An action is attempted; the world resolves it. A belief record carries a value, confidence, source, acquisition time, and method. |
| Vol. I Ch. 4 §10–11 | Perception: eligibility, then signal quality, then observation, then belief update. Cognition is layered: needs and pressures, goals, candidates, evaluation against beliefs, commitment (interruptions cost), and a compact **decision trace** (pressures, candidates, decisive factor, choice, triggering belief). |
| Vol. III Ch. 2 §2.4, §2.8 | Needs are measurements, never behaviours; "a system elsewhere may translate a critical need into a goal". Sensory capability bounds observation but never grants knowledge. |
| Appendix A, Rulings 4 and 13 | The need is Living's, the knowledge Information's, the decision a decision system's, the travel Physical's. A decider proposes intents; Physical alone moves bodies and doors. |
| Appendix A, Part 1 | "Observation, knowledge, memory, belief (individual)" is owned by the Information layer. Decision systems appear only as *consumers*; nothing owns goals, plans, or traces yet. |
| Vol. V Ch. 9 §9.3 | The reference mind is **utility-based and deterministic**. A language-model mind may sit in the same seat only as a *recorded input*, sees beliefs only, and is never the crowd's machinery. |
| Prototype v0.1 (`archive/poc-v0.1/engine/systems/agents.py`) | "No shadow systems": an agent's effects go through exactly the doors a player's do. Its agents broke the newer rules: they read reality directly and wrote state directly. |

---

## 3. What exists, and what's missing

**Exists:**
- **Intents, and refusals the agent can notice.** Travel (`TRAVEL_TO`, `TRAVEL_SPEED`), open,
  shut, and face, carried out by Physical Reality only. The world reports back when it can't:
  travel blocked (`TRAVEL_BLOCKED`), or an act refused (`ACT_REFUSED`). Authority is
  kernel-enforced (A-5).
- **Geometric sight:** `line_of_sight` and `visible` through walls, glass, doors, curtains, and
  terrain.
- **Illumination** per place, including indoor daylight through openings.
- **Body heat** per organism, the one need.
- **Declared reads,** enforced by the kernel: a system that reads a fact type it did not declare
  aborts the tick. This is the hook that makes "minds read beliefs only" enforceable.

**Missing:**

| Gap | Why it blocks agents |
|---|---|
| **No way to store a belief** | A belief is about a pair: Erin's belief about *where Bob is*. A fact is addressed by one entity and one type, and values are a number, a flag, an entity, or a triple. "Erin believes Bob is in the kitchen" has nowhere to live. |
| **No perception** | Sight is a query, not a process: nothing records who saw what, and light doesn't affect it. |
| **No information layer** | No beliefs, no memory, no aging, no starting knowledge. |
| **No decision systems** | No needs-to-goals translation, no utility, no commitment, no traces. |
| **No owner for goals and traces** | Appendix A lists decision systems as consumers only. |

---

## 4. Design decisions

### 4.1 Beliefs are pairwise facts *(kernel; recommended)*

**Change:** a fact may be addressed by a *pair*: holder, fact type, and the entity it is about.
- Erin's belief about Bob's whereabouts is `(Erin, info.belief.place_of, about Bob) = the kitchen`.
- The provenance already on every fact gives the belief its acquisition tick, its source system,
  and its method (the cause: *seen*, *felt*, *told*, *known from the start*).
- Confidence is computed from age and method (§4.4), so it needs no stored field.

This is general, not belief-specific. The same shape is what the spec needs next for:
- **opinions:** Ruling 2, one person's trust in another;
- **multidimensional relationships:** Vol. I Ch. 4 §12;
- **debts and obligations.**

The kernel stays ignorant of all of them: it learns only that a key may carry a second entity.

**Alternatives:**
- **Each belief its own entity** (holder, subject, value, … as ordinary facts). This is the most
  flexible: fields can be added one fact type at a time. But it needs the runtime id allocator
  (P8), which does not exist. It also turns every glance into new entities, and lookup "what
  does Erin believe about Bob" needs an index of its own.
- **A compound value type** (a record or tuple). This teaches the kernel record shapes. Value
  would also stop being `Copy`, which ripples through every system.

**Cost:** `FactKey` gains an optional third field, which goes into ordering, hashing, and
persistence. The store gains one range read: "all of Erin's beliefs of this type".

### 4.2 Two new owners, kept apart: `information` and `minds`

| Crate | Owns | Systems | May read |
|---|---|---|---|
| `domains/information` (Vol. II Ch. 4) | Observations, beliefs, memory: every `info.*` fact | Perception (turns what Physical says is in view, and what a body feels, into beliefs), aging | Reality, through Physical's and Living's published facts. Observation reads reality. |
| `domains/minds` (decision systems) | Goals, commitments, decision traces: every `mind.*` fact | Deciding: needs to goals to candidates to choice to intents | **Information and its own facts only.** Its writes are intents, which anyone may propose (Ruling 13). |

The wall between the two is the spec's central rule, and the kernel already enforces it.
- **The kernel check:** a decision system that reads `POSITION`, `CONTAINED_IN`, or `BODY_HEAT`
  without declaring them aborts the tick (hermeticity).
- **The `minds` check:** the crate refuses to declare them. A test holds every `minds` system's
  read set to `info.*` ∪ `mind.*`.

**Self-knowledge is perception too.** A mind learns where it is, how cold it is, and that its
way was blocked from beliefs about itself, written by perception. It never reads the facts
themselves.

**Amendments this needs:**
- **Appendix A:** add rows for goals, commitments, and decision traces.
- **Vol. V Ch. 1 (the layer diagram):** add the information layer and decision systems to the
  domain layer, still never importing one another.

**Alternative:** one crate for both. This is cheaper, but the wall becomes a convention inside
a crate rather than a boundary visible in the directory listing (Vol. V Ch. 1,
invariant 10: the law should be discoverable by inspection).

### 4.3 Physical publishes what is in view; Information decides what is observed

Domains never import each other, so the information layer cannot call `line_of_sight`.
Visibility is already Physical's in the matrix ("blocking, visibility, passability"). So:
- **Physical publishes** an `IN_VIEW` relation on each sighted body: what it could see this
  step. It is light-aware (nothing is seen in a dark room) and bounded by the body's sight range,
  which is Living's sensory capability. Sight range is a world rule until Living has senses.
- **Information reads it** and decides what becomes belief (Vol. I Ch. 4 §10.1: eligibility,
  then signal, then observation, then interpretation).

Phase 1 perceives:
- **Seen:** where things are (place and position); whether an opening is open, and where it
  leads.
- **Felt:** the warmth of the place one stands in; one's own body heat.
- **Told by the world:** that one's travel was blocked, or an act refused.

Hearing and smell come later, as more channels into the same layer.

### 4.4 Memory is belief that has not been refreshed

- **Beliefs persist** until perception replaces them.
- **They go stale.** If Dave saw Bob in the kitchen and Bob left while Dave was upstairs, Dave
  still believes Bob is in the kitchen.
- **Confidence is a deterministic function of age:** a belief is trusted `H / (H + age)`, half
  trusted at an age `H` the world declares (Vol. II Ch. 4 "Information Decay"). How much to trust
  old news is the mind's judgement, so `H` is a rule of the minds. This keeps confidence exact
  under replay, at any tick length, and free to store.
- **Starting knowledge is package data.** A `[knows]` section seeds what each agent knows at the
  start: Erin knows her way round Alice's house; the courier knows only the yard. This is a
  scenario concern (Vol. IV Ch. 5), and it is how two agents in the same reality come to choose
  differently.

### 4.5 Minds are deterministic utility deciders that act through a player's doors

Each mind steps on a cadence. Thinking every few simulated seconds is a world rule, staggered by
id so a crowd does not think in lockstep. Each step:

1. **Pressures** come from believed needs: "I am cold". The only need today is warmth.
2. **Goals** come from pressures and from routines the world declares, such as "the guard keeps
   the north gate by day".
   - Routines are authored content, owned by `minds` until Society owns roles.
   - A mind may know the time of day: time is the clock, not a fact about the world.
3. **Candidates** come from beliefs only: the places I know, how warm I remember them, the ways I
   believe lead there.
4. **Evaluation** scores each candidate as urgency × believed gain − believed cost, with ties
   broken by id. No candidate is drawn from reality, so a mind never walks to a warm room it has
   never seen.
5. **Commitment** with hysteresis: a goal is kept until done, impossible, or clearly outscored, so
   minds don't flip-flop.
6. **Acting** is by the intents a player would use (travel, open, face). Physical decides the
   outcome; the refusal comes back as a belief.
7. **The trace** is a few facts per decision (goal, choice, decisive score, triggering belief),
   so "why did Erin go in?" has an answer (Vol. I Ch. 4 §11.6).

Language-model minds stay out (Vol. V Ch. 9): later, recorded at the input door, for a named
character or two.

---

## 5. Build plan

Each phase is one amendment, one gate in Ashford, and nothing beyond it.

| Phase | Amendment | Builds | Gate in Ashford |
|---|---|---|---|
| **1. Pairwise facts** | A-7: Vol. V Ch. 2 §2.1 (the fact model) | `FactKey` gains an optional *about*. Range reads by holder and type; ordering, hash, index unchanged for unpaired facts. | Kernel contract tests: a fact about a pair is its own fact; hashing and replay hold; the spatial index ignores it. The whole suite passes unchanged. |
| **2. Perception and belief** | A-8: Vol. II Ch. 4; Vol. III Ch. 1 (visibility published); Appendix A; Vol. V Ch. 1 | Physical: `IN_VIEW` (light-aware, range-bounded). `domains/information`: perception, beliefs with method and age-derived confidence, `[knows]` starting knowledge. | Dave sees Erin from the window and believes she is in the yard. Bob leaves the kitchen while Dave is upstairs, and Dave still believes he is there. At midnight Dave cannot see Erin. Drawn curtains hide the yard. The courier has never seen the kitchen and holds no belief about it. Replays are bit-identical. |
| **3. Minds** | A-9: Appendix A (goals, commitments, traces); Vol. V Ch. 9 (the reference mind) | `domains/minds`: pressures, routines, candidates, utility, commitment, intents, traces. Read sets limited to beliefs. | **Erin**, cold in the yard at night, goes into the kitchen she remembers as warm. **The courier**, who doesn't know it, stays put. **Shut out:** blocked by the shut door, the courier waits, and when he sees Bob open it, goes in. **The guard** keeps his routine. **Same seed,** same choices, and each trace explains its choice. **A mind that declares** `POSITION` is refused. |
| 4. Needs beyond warmth | Vol. III Ch. 2 | Fatigue and rest, and the cold having consequences (D7, with M10's health). Gives minds competing pressures. | Bob, tired and cold, chooses between bed and hearth, and the trace says why. |
| 5. Talking | Vol. II Ch. 4 "Sharing Information" | Telling a belief passes it on with its source ("told by Bob") and lower confidence; rumours; lies. | Bob tells the courier the kitchen is warm; the courier now goes in, citing Bob. |
| 6. Model-backed minds (optional) | Vol. V Ch. 9 | One named character's choices proposed by a model, recorded as inputs. | Replays exactly from the recording; headless runs need no model. |

**Order:** 1 → 2 → 3 makes Ashford's people *agents*: they notice, remember, want, choose, and
are refused. Phases 4–5 make them interesting.

**Hunger and fetching things** depend on other work in the sweep's order:
- **M5 (kinds):** to want "food" or "a bed", a mind needs things to have kinds.
- **M6 and M7 (interaction verbs and runtime entities):** eating takes and consumes something.

**Building perception now also serves the player.** The player is an ordinary entity (Law 10),
and the narrator should describe what the player perceives. Both read the same information
layer.

---

## 6. Decisions taken

The project owner chose, on 2026-10-02:

1. **Pairwise facts in the kernel** (§4.1), not belief entities.
2. **Two crates,** `information` and `minds` (§4.2).
3. **Warmth and routines** as the first minds' goals (§4.5).

---

## 7. What building phases 1–3 changed

- **Night had to become dark.** Daylight was a triangle wave from midnight to noon. That left
  Ashford above the sight threshold for all but about fourteen minutes of the night, so light
  could not bound sight. Daylight now follows the sun's height: zero from sunset to sunrise (A-8).
- **Coming back into sight refreshes a memory.** A belief is stamped *seen* whenever its subject
  comes into view, even if nothing about it changed. Otherwise a thing glimpsed, lost in the
  dark, and seen again at dawn would keep its old *lost sight* stamp.
- **Confidence is rational, not exponential.** Trust is `H / (H + age)`, half at age `H`. It is
  exact in integers and a rule of the minds, not the information layer.
- **A fresh intent outlives the clearing of the old** (Physical's composition rule for intents,
  A-9). A mind may ask for something new in the tick Physical clears an intent it fulfilled.
- **The shut-door story is Erin's, not the courier's.**
  - The courier knows only the yard, so he never wants the kitchen.
  - It is Erin, cold and remembering the kitchen warm, who finds the door shut, walks to it,
    opens it, and goes in.
  - When her travel is reported blocked, a mind walks to the first opening it believes is on
    the way and opens it.
- **The kitchen needed a reason to be warm.** Without a heat source a walled room at night is
  only about 2 °C warmer than the yard. Erin's starting memory, the 14 °C air at midnight, gave
  her no reason to go in. Ashford's kitchen now starts at 20 °C, warm from last night's stove,
  and she remembers that.
  - **She decides at 22:50.** Her body heat is 22.7 °C, below the 23 °C comfort line. The yard
    feels 14.6 °C. Her 23-hour-old memory of the kitchen says 20 °C. The choice scores 92.
  - **Inside, she stays.** The kitchen then feels 16.9 °C, still warmer than the yard.
  - **Heat sources would make this honest.** Hearths and lamps (sweep M12) are the real fix.
- **Sight costs test time.** With sight refreshed every second for 26 people, the engine suite
  takes about 16 s in a debug build, up from 5 s. Refreshing only for bodies whose surroundings
  changed is the obvious next optimisation.

