# Autonomous Agents: Lives That Run Themselves

> **Status (2026-10-06): all seven phases built** (A-14 to A-19). The Ashford gates in §3 pass,
> recast where the world disagreed with the plan: the courier, curious, goes to see the house (the
> Hill lies beyond the known limit below), and Erin, not Ida, keeps the glass bottle (Southfen is
> beyond reach of anything in Ashford). Built on the way, each recorded in its amendment:
> - A world begins with its light (Vol. IV Ch. 4, *Completeness*).
> - Minds sleep in the world's sleeping hours, and Ashford tires at 4.5% an hour, so a day's work
>   fits between them.
> - A mind gives up on what it cannot reach until it learns something newer.
> - Perception knows where one put a thing down, how much is left on what one picks, and that a
>   portable thing has gone.
> - Ashford grew food for everyone the orchard is beyond: manor, cellar, cart, ship, quay, farm,
>   mill, wood, moor, fen, and the Hill.
>
> **Known limit:** travel between open places that share a parent but have no opening between them
> (Old Town and the Hill) is still blocked; the planner routes only through openings.

*2026-10-06, on branch `feat/agents`. The agents plan (`docs/audits/agents.md`) gave Ashford minds
that act on what they believe. `docs/audits/needs-that-arise.md` gave them love, items, and
addiction. This plan finishes the job: every person and animal acts on its own, on its needs,
its wants, and its work, with nobody directing it. Every step starts with its spec amendment and
ends in an Ashford gate.*

---

## 1. Bottom line

An agent's day should come from the world, not a script.
- **Needs.** It eats when hungry, sleeps when tired, comes in from the cold, longs for its lover.
- **Work.** It does its job in its hours: the farmer picks the orchard and stocks the larder; the
  cook bakes pies at the hearth.
- **Wants.** When nothing presses, it goes to look at places it has heard of but never seen, and
  gathers things it likes to keep at home.

All of it goes through the same doors a player uses (intents), and Physical, Living, Resources,
and Economy decide what happens.

The project owner chose:
- **Everyone acts, unless directed.** Every organism has a mind. A player, or a test, can direct
  an entity, and while directed its own mind stands aside. The player character will be a
  directed person, nothing more.
- **Things come into being.** A deterministic id allocator (the sweep's M7) lets an orchard grow
  apples and a hearth bake pies as real items: carried, eaten, owned.
- **The first jobs are farm-and-stock and cook.**
- **The first wants are curiosity and possessions.**

---

## 2. Who owns what

| Fact or act | Owner | Spec |
|---|---|---|
| Whether a mind is being directed | Decision systems (minds) | Vol. V Ch. 9 (the player is a recorded input source) |
| New entity ids | Kernel: deterministic, never reused | Vol. V Ch. 2 §2.1, clause 4 |
| A thing coming into the world, or leaving it | Physical Reality, on request (Ruling 16 extended) | Vol. III Ch. 1 |
| An orchard's stock of fruit, and its regrowth | Resources (deposit, stock, regeneration) | Vol. III Ch. 3 |
| Picking: an extraction that yields an item | Resources judges the yield; Physical places the item in the picker's hand | Vol. III Ch. 3 §3.4 |
| Hunger, and what eating does | Living Systems (a need; nutrition is a material property) | Vol. III Ch. 2 |
| Recipes, and making things from them | Economy (production); Physical retires inputs and places outputs | Vol. III Ch. 4 §4.4 |
| Who owns a thing | Economy (holdings) | Vol. III Ch. 4 §4.4 |
| Jobs: who does what, where, and when | Society (roles), from job kinds the world declares | Appendix A: roles are Society's |
| Likes, home | Decision systems (a mind's dispositions) | A-11's temperament precedent |

---

## 3. Build order

| Phase | Amendment | Builds | Gate in Ashford |
|---|---|---|---|
| 1. **Everyone acts** | A-14 | Every organism in Ashford gets a mind. A mind under direction stands aside. The test harness directs whoever a test commands, and offers a quiet Ashford, without minds, for tests of physics and perception. | Bob, directed up the stairs, goes up, and his mind does nothing to stop him. Released, he decides for himself again. A night in Ashford with nobody directed: everyone ends it somewhere they chose, for a reason in their trace. |
| 2. **Things come into being** | A-15 | Kernel: new ids, deterministic and never reused. Physical: things arrive in a place or a hand, and leave the world, on request. Resources: a deposit's stock regrows at its declared rate up to a cap, and picking takes one from the stock and yields a new item in the picker's hand. | The orchard ripens an apple every few hours; picked, an apple is a new thing in the picker's hand. Same seed, same ids. |
| 3. **Hunger** | A-16 | Living: hunger rises, eating lowers it by the food's nutrition, and starving harms. Minds fetch and eat food they believe in. The craving machinery becomes general: anything consumed to meet a need. | Hungry, Erin goes to the larder she knows and eats. A world without food starves; with an orchard, nobody does. |
| 4. **Making things** | A-17 | Economy: recipes (inputs, product, workplace) as package data, and a make intent carried out at a workplace. Physical retires the inputs and places the product in the maker's hand. | At the hearth, apples and flour become a pie. Without the flour, nothing. |
| 5. **Jobs** | A-18 | Society: roles from job kinds the world declares, from a closed set of mechanisms: *carry* (move things of a kind from one place to another) and *make* (keep a store supplied by a recipe). Minds work their hours when nothing more pressing calls. | The farmer picks the orchard and stocks the larder. The cook bakes pies from what is there. The hungry eat the pies. |
| 6. **Wants** | A-19 | **Curiosity:** visit places one knows of but has never stood in. **Possessions:** Economy holdings; a mind likes certain materials, claims unowned things made of them, and keeps them at home. It never takes what it believes is someone else's. | Idle, the courier goes to see the Hill. Ida gathers glass things and keeps them at home, and leaves Bob's lamp alone. |
| 7. **A week in Ashford** | — | No new rules: the whole city, undirected. | Same seed, same week. People eat, work, sleep, and want. Nobody starves. The trace explains every day. |

Each phase is built on the ones before it.

---

## 4. Design notes

- **Reactive plans, not scripts.** Each kind of goal has a *next step* function: given what the
  mind believes now, what single act brings it closer?
  - *Go:* the next opening, or the place itself.
  - *Fetch:* go to the thing, take it.
  - *Eat:* consume what is in hand.
  - *Carry:* take, go, drop.
  - *Make:* fetch the inputs, go to the workplace, make.

  The plan is recomputed from beliefs each time, so a closed door, a moved apple, or an empty
  shelf is met by the next thought, not by a stale script.
- **One scale for everything.**
  - Needs score by urgency.
  - Jobs score by a declared worth, during their hours.
  - Wants score low, so they fill idle time.

  A choice is kept until done, unwanted, impossible, or clearly beaten (A-9).
- **Direction is control, not a bypass.** A directed entity still perceives, needs, and is judged
  by Physical and Living. Only its mind is silent. The player's commands will arrive the same way
  (Vol. V Ch. 9: the player is an input source).
- **Nothing is free.**
  - The orchard regrows at a declared rate, up to a cap.
  - A pie consumes real apples and real flour.
  - Eating removes the food from the world.
  - Weeks of plenty are earned by the farmer's walks and the cook's work.
