# Cardinal Architecture Specification
# Volume V
# Reference Architecture
## Chapter 9
# AI Integration

> *The model may speak for the world. It may never speak to it.*

---

# Chapter Overview

Cardinal was conceived alongside large language models, and Volume I drew the line on day one: simulation before narration; the narrator describes committed state and never creates it.

This chapter is that line as engineering. Three integration points admit AI into the architecture, each with its own wall:

**The narrator** — rendering committed reality as prose.
**The interpreter** — parsing free-form player input into candidate actions.
**Agent minds** — optionally, letting models participate in NPC decisions.

The common law across all three, before the specifics: a model is a *frontend-layer component* (Ch. 1). It consumes the observation plane and submits through the input door like every other consumer (Ch. 6), and it is architecturally incapable — not merely forbidden, *incapable* — of writing a fact. The walls of this chapter are the reason "the AI hallucinated" can never be a sentence about world state.

---

# 9.1 The Narrator

## Position

The narrator subscribes to a *perspective-filtered* observation stream (Ch. 6's entitlement): committed deltas and events, scoped to what the bound perspective could perceive (Vol. II Ch. 4). It renders prose from exactly this feed.

```text
observation plane ──► perception filter ──► narration context ──► model ──► prose
     (committed)        (entitlement,          (curated,            │
                         Vol. II Ch. 4)         bounded)            ▼
                                                              player's screen
                                                          (and NOWHERE else)
```

The output arrow terminates at presentation. Prose is never parsed back into state, never cached as fact, never consulted by any system. The narrator could claim the moon is cheese; the moon's facts would not notice.

## Grounding Discipline

Hallucination cannot corrupt the world, but it can corrupt the *account* of it — so the reference design grounds hard:

- **Context is committed state only.** The narration context contains the perception-filtered deltas, relevant standing facts, and vocabulary (Vol. IV Ch. 1) — assembled deterministically. No prior prose is authoritative context (style memory is fine; *fact* memory comes from the world each time).
- **The rules ride in the system prompt.** The narrator's charter — describe, never invent; perception limits; the world's vocabulary and tone — is the cached system prompt (the current engine's pattern, validated in production).
- **Contradiction is a rendering bug.** Tooling may lint prose against the context (names, numbers, deaths). A narrator that says the dead man spoke has a defect *in the frontend* — filed, fixed, and never load-bearing, because nothing downstream consumed it.

## Degradation Is a Contract

The current engine's hardest-won pattern, canonized: **the narrator must fail closed to a deterministic renderer.** Missing package, missing credentials, timeout, refusal, malformed output — any failure drops the session to the plain template narrator, permanently for that session, without a simulation hiccup. The simulation loop *never* awaits a model on its critical path; narration is asynchronous to the tick, and a world must run headless forever (every soak of Chapter 8 does exactly that). LLM output is never a dependency of state computation — Volume I's rule, now with a failure-mode specification.

---

# 9.2 The Interpreter

Free-form input ("I shove the table over and dive behind it") must become the pipeline's INTERPRET output: typed candidate actions.

```text
player text ──► model ──► candidate Action(s) ──► VALIDATE (Vol. II pipeline)
                              (typed, schema-      │
                               checked, from the   ├─ valid   → COST → …
                               engine's action     └─ invalid → refused, narrated
                               vocabulary)
```

The wall here has a precise location: **the model proposes candidates; VALIDATE decides.** The interpreter's model may misread wildly — propose shoving a table that does not exist — and validation refuses it against committed state exactly as it would refuse a typo'd CLI command. The model holds no authority; it is a parser with better manners. Consequences:

- The action schema is the contract: interpreter output is schema-checked *before* validation even sees it; free text never reaches the simulation.
- Ambiguity resolves by asking, not guessing: multiple plausible candidates go back to the player as a clarification, because the interpreter deciding "what you really meant" is authority leaking toward the model.
- The deterministic command grammar (the CLI's verbs) remains first-class forever — the interpreter is sugar over the same door, and headless operation uses the grammar alone.

---

# 9.3 Agent Minds

May a model decide what an NPC does? The architecture's answer: yes, in exactly one seat, under exactly one condition.

**The seat.** Decision systems consume beliefs and produce intended actions (Vol. III passim; Vol. II Ch. 4's pipeline). A model may serve as a decision *policy*: belief-state in, intended action out — subject to the same walls as the player's interpreter (typed candidates, VALIDATE decides, perception-bounded context only — an LLM-backed mind sees its *beliefs*, never reality; omniscient AI is one context-assembly bug away, and the entitlement filter is the guard).

**The condition.** Determinism (Ch. 4) does not bend. A sampled model is a nondeterminism door, so an LLM-in-the-loop mind must either (a) be *recorded*: its outputs captured as inputs in the chronicle, making replay exact (the model is then formally an input source, like a player — which is the honest description); or (b) run in study/companion contexts explicitly outside the deterministic guarantee, never in reference worlds or tested saves. There is no third mode; "mostly deterministic" is Chapter 4's tolerated flake wearing a mind.

**The default.** Utility-based deterministic decision systems (the current engine's agents) remain the reference for the population at large — cheap, testable, tunable by rules. Model-backed minds are a *garnish for depth* (a named character's dialogue-driven choices, recorded as inputs), not the crowd's machinery. A thousand LLM wolves is a cost model and a replay problem; one LLM chancellor, recorded, is a feature.

---

# Designer Note
## The Stochastic Guest in the Deterministic House

The tension this chapter manages is real and permanent: Cardinal's value *is* its lawfulness — replay, audit, conservation, proof — and language models are magnificent, unlawful guests. The resolution is neither exclusion nor surrender. It is *doors*.

Every admitted integration sits at a door the architecture already had: the narrator at the observation plane's exit (speaking *about* the world), the interpreter and the recorded mind at the input door (speaking *as a participant*, validated like one). Notice what is absent: any door into the middle. No model in a system's evaluation. No model in resolution. No prose consulted by a rule. The tick pipeline contains zero stochastic components, and every chapter of this volume conspired to keep it that way.

Hold the doors, and the guest is transformative: worlds narrated with genuine literary texture, players speaking naturally, a chancellor who converses. Lose one door — one system that "just asks the model when uncertain" — and five volumes of law dissolve into vibes. The house rule is short: *models at the doors, never in the walls.*

---

# 9.4 Common Queries

- What context, exactly, produced this narration? (grounding audit)
- What candidates did the interpreter propose for this input, and what did VALIDATE rule?
- Which agent decisions this session were model-backed, and where are they recorded?
- What is the narrator's current mode (model / degraded), and what tripped degradation?
- What did model integration cost this session (calls, latency, tokens), per seat?
- Does this save replay identically with all recorded model outputs? (Ch. 4 gate)

---

# 9.5 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Models are frontend-layer components; no model call exists in kernel, domains, or services.
2. The tick's critical path never awaits any model; headless operation is complete and permanent.
3. Narration consumes perspective-filtered committed state and terminates at presentation.
4. Prose is never parsed into, cached as, or consulted as world state.
5. Narrator failure degrades closed to the deterministic renderer, per session, without simulation impact.
6. Interpreter output is schema-typed candidates; VALIDATE holds sole authority.
7. Ambiguous interpretation returns clarification; models never resolve intent by fiat.
8. Model-backed agent minds see belief state only, bounded by entitlement.
9. In deterministic contexts, model outputs are recorded inputs in the chronicle; otherwise the context is explicitly non-reference.
10. The deterministic grammar and utility agents remain complete without any model present.

---

# 9.6 Anti-Patterns

### The Oracle System

A domain system that consults a model mid-evaluation ("ask the LLM if the treaty seems fair"). This is the one absolute taboo: a stochastic call inside the tick voids determinism, hermeticity, and audit in a single line.

### Narration Feedback

Parsing the narrator's flourishes into facts ("it said the tavern was crowded — spawn patrons"). The arrow terminates at the screen. A world listening to its own storyteller is Volume I's first directive, inverted.

### The Omniscient Familiar

Assembling agent-mind context from reality instead of beliefs "because the filter is expensive." Vol. III Ch. 4's omniscient-AI mistake returns wearing a transformer; the entitlement filter is not optional garnish.

### Await-the-Muse

Blocking the tick on narration for output quality. The world does not wait for its chronicler; buffer, degrade, or drop — the simulation's cadence belongs to the simulation.

### Unrecorded Genius

Model-backed decisions in worlds that claim replay. Every unrecorded sample is a fork in history that no bisection can find; record at the door or stay outside reference worlds.

---

# 9.7 Future Evolution

Future versions may explore:

- local models for interpretation and degraded-mode narration
- style memory systems (voice consistency without fact authority)
- model-assisted authoring at build time (content suggestion into the validated pipeline — a *tool* seat, offline, outside this chapter's runtime walls)
- recorded-mind libraries: shareable, replayable character performances

New seats will be proposed. Each gets the same two questions: *which door*, and *what wall* — and any seat that cannot answer both stays outside.

---

## Preparing for the Final Chapter

The house is built, instrumented, and guarded. What remains is how it grows old well:

**Extension and Evolution** — plugins, the performance philosophy, and the roadmap discipline that lets Cardinal outlive every choice this volume made.

---

# END OF CHAPTER 9
