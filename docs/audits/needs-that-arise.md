# Needs That Arise: Love, Items, and Addiction

> **Status (2026-10-06): all three steps are built** — A-11, A-12, A-13; gates in
> `tests/reference/tests/{love,items,addiction}.rs`. See §5 and §6 for what building them showed.

*2026-10-05, on branch `feat/agents`. A plan for needs that are not there from the start but
come into being from what happens:*
- *someone falls in love and then wants to be with them;*
- *someone takes a drug, becomes dependent, and then craves it.*

*It follows the agents plan (`docs/audits/agents.md`), phases 1–4. Every step starts with its
spec amendment, and every gate is a scenario in Ashford.*

---

## 1. Bottom line

**A need that arises is a fact about a pair** (A-7): it has a holder and something it is
about.
- **Longing:** "Gwen longs for Finn" is `(Gwen, need, about Finn)`.
- **Craving:** "Bob craves poppy" is `(Bob, need, about poppy)`.

**The engine knows a small, closed set of mechanisms. The world declares the kinds of need
built from them.** This is the same split as materials, which have properties (Vol. III Ch. 1).
- **What makes a need arise:** a bond to someone, or a dependence on something.
- **What meets it:** that someone's presence, or a dose of that something.
- **What it costs:** how fast it grows, how fast it eases, and what it does to health when unmet.

A world can declare "longing", "grief", "craving", or "homesickness" with no engine change, as
long as it is built from mechanisms the engine has.

The project owner chose:
- **Love grows from time together.** Affection builds while two people spend time in each
  other's sight, faster the more compatible they are. Lovers form only when it is mutual and
  they are compatible enough, so not everyone falls for everyone.
- **Items first.** Taking, carrying, dropping, and consuming things come before drugs, with what
  is consumed leaving the world (Vol. III Ch. 2: consumption is conservative).

---

## 2. Who owns what

| Fact | Owner | Why |
|---|---|---|
| Temperament: a few traits per mind | Decision systems (minds) | A mind's disposition, authored like routines. |
| Affection: how fond A is of B (a pair) | Information layer | One person's opinion of another is information: personal, fallible, fading (Appendix A, Ruling 2). |
| Bond: A and B are lovers (a pair, both ways) | Society | A relationship that outlives the moments that made it (Vol. III Ch. 5). Kinship already includes marriage. |
| Need kinds: what makes one, what meets it, its rates | World package data, read by Living Systems | Needs are Living's (Appendix A); a world chooses which ones exist. |
| A need held: its level and its kind (a pair) | Living Systems | Vital state. A need is a measurement, never a behaviour (Vol. III Ch. 2). |
| Taking, carrying, dropping, consuming | Physical Reality, on request (a new ruling like Ruling 13) | Where a thing is, and whether it is anywhere at all, is Physical's. |
| What a consumed thing does to a body | Living Systems, from Physical's report | The same pattern as falls (Ruling 9): Physical reports, Living judges. |
| Dependence on a substance (a pair) | Living Systems | Physiology. |
| Substance properties: potency, how habit-forming | Physical Reality (material properties) | Properties over names: the engine never knows "poppy". |
| What a mind knows of its needs, and what things are made of | Information layer | A mind feels its craving and believes the vial is poppy. It never reads either. |

---

## 3. Build order

| Step | Amendment | Builds | Gate in Ashford |
|---|---|---|---|
| **1. Needs that arise, and love** | A-11 | **Need kinds** as package data, with mechanisms from a closed set. **Temperament** (minds). **Affection** (information): grows while two people are in each other's sight, faster with compatibility, and fades apart. **Bonds** (Society): lovers form when affection is mutual and they are compatible enough. **Longing** (Living): arises from the bond; grows while apart and eases in each other's presence. **Minds** go to where they *believe* their beloved is, and perception notices when someone is not where they were thought to be. | Finn and Gwen, side by side on the doorstep and well matched, fall for each other within days. The courier stands in the same yard but is ill-matched, and stays a stranger. When Finn is sent to the harbour, Gwen's longing grows and she goes after him to where she last saw him. |
| **2. Items** | A-12 | **Intents** to take, drop, and consume, carried out by Physical only within reach (a ruling like 13). **Carrying** is containment in the carrier, up to a mass limit: density × size, a world rule. **Consuming** removes the thing from the world, keeps its identity, and is reported to Living. **Perception** believes what seen things are made of. | Bob takes the lamp upstairs and puts it down by Alice. The courier cannot lift the wardrobe. A tincture drunk is nowhere afterwards, but its history remains. |
| **3. Substances and addiction** | A-13 | **Material properties** potency and habit (Physical). **Dependence** rises with each dose of a habit-forming substance and fades with abstinence (Living). **Craving** arises past a line (a need kind). It grows, eases with a dose, and harms health unmet (withdrawal). **Minds** crave, look for things they believe are made of the substance, and take them. | Bob, given the apothecary's poppy tinctures, comes back for more as dependence builds. When the shelf is empty, withdrawal harms him. Weeks without, the craving goes. Erin, who never took one, never wants one. |

Talking and rumour, the agents plan's phase 5, follows these.

---

## 4. Design notes

- **Compatibility** comes from three temperament traits (0–100% each): 100% minus their mean
  difference. The world declares how compatible lovers must be, and how fast affection grows and
  fades.
- **Affection is an opinion, so it is per mind and one-way.** Finn can be fond of Gwen while
  Gwen is indifferent. Society forms a bond only when both are past the line. It ends the bond
  when both fall below a lower line, so love does not flicker.
- **Absence is observable.** Standing in a lit place, a mind that believes someone is there and
  cannot see them concludes they are not, and drops the belief. Then it does not wait for ever
  where its beloved used to be.
  - For this, Physical's *in view* now includes the place one stands in, when it is lit. Seeing
    the room is how a mind knows its view is good enough to judge absence.
- **Generic satisfaction.** A mind does not know "longing" or "craving". It knows a need of some
  kind, about something, met by *presence* or by a *dose*:
  - **Presence:** it goes to where it believes the object is.
  - **Dose:** it goes to things it believes are made of the object, and takes and consumes one.
  - The urgency is the need's level above the world's line. It competes with warmth, rest, and
    routines on the same scale.

---

## 5. What building step 1 showed

- **They fall in love on day 2.** Finn and Gwen, 97% compatible, grow fond by day and cool a
  little in the dark, and become lovers at about 14:30.
- **The courier stays a stranger.** He sees Finn every day, but his fondness stops at 48%,
  their compatibility. Love is limited by fit, not just by time together.
- **Being in view counts as being together.** With Gwen in the cellar and Finn in the kitchen,
  they can see each other up the stairwell, and longing does not grow until dusk takes the light.
- **Two minds can miss each other in the dark.** With both lovers able to act, Finn went down to
  find Gwen as she came up to find him. In the dark neither could see the other was gone, and
  each waited where they last saw the other. Daylight resolves it: absence can be seen, and so
  can the other one.
  - Nothing prevents this crossing. The gate keeps one lover still so its story is about the
    other.
  - A future refinement could let minds hear each other on the stairs, or remember which way
    someone went.
- **Cold and love compete on one scale.** A cold mind with a warm room in mind and a lover
  outside weighs both. Once the longing is strong enough, love wins.

---

## 6. What building steps 2 and 3 showed

- **A latent sight bug.** Line of sight panicked when one end contains the other: a person
  looking at what they hold, or a rider at the cart. The search for containers between the two
  ends ran backwards when the common container was one of the ends. It has been there since A-4.
  Holding things exposed it. Fixed, and covered by a test in `sight.rs`.
- **What one holds, one knows by touch.** Physical's view of a body now always includes what it
  holds, light or dark. Without that, Ned, taking a vial in an unlit undercroft, would never
  learn he had it.
- **Feeling a thing go from one's hand.** When something one held leaves one's view (drunk, or
  put down), the belief that it is in hand goes. It is not stamped "last seen", as other things
  are.
- **Craving accelerates.** Each dose of poppy builds 25% dependence, and craving grows faster the
  deeper the dependence. So Ned's cravings come 9 h 50 min, then 7 h 20 min, then 6 h 10 min
  apart: nobody wrote "addiction gets worse".
- **Withdrawal, then recovery.** With the shelf bare by noon on day 1, the craving reaches 100%
  by day 2. Withdrawal takes Ned from full health to about half over four days. At 15% a day, the
  dependence falls back below the line on about day 6, and the craving ends.
- **Rules a world sets:** how habit-forming each substance is, how fast dependence fades, and
  how harsh withdrawal is. They live in `[materials]`, `[rules.living]`, and `[need_kinds]`.
  Ashford's are tuned so Ned suffers and recovers. A harsher world could kill him.

