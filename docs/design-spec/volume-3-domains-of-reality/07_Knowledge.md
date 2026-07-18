# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 7
# Knowledge

> *Every skill a civilization possesses is one unbroken chain of teachers away from being lost forever.*

---

# Chapter Overview

Volume II, Chapter 4 established the information architecture: observation, knowledge, memory, and belief as the personal, fallible counterpart to objective reality.

That chapter answered how a single observer comes to know anything.

This chapter answers a larger question:

**How does a civilization know things?**

How does the technique of smelting iron exist in a world — not in a menu, but in the heads of particular smiths in particular towns? How is it taught, hoarded, stolen, written down, and lost? How does a better way of doing something come to exist at all?

The Knowledge domain governs *capability knowledge*: skills, techniques, recipes, discoveries, and the living structures — apprenticeships, guilds, libraries, schools — through which they survive the deaths of the minds that carry them.

Where Volume II's information layer is about knowing *facts*, this domain is about knowing *how*.

---

# 7.1 Purpose

The purpose of the Knowledge domain is to make capability distributed, local, and mortal.

In most games, technology is global: once "Iron Working" is researched, every settlement can work iron, forever, automatically.

Cardinal rejects this.

In Cardinal, the ability to work iron exists only where a person who can work iron exists. It arrived in that town in someone's hands. It will leave the same way — by travel, by teaching, or by a funeral.

This single commitment generates, for free:

- towns famous for a craft
- guilds that guard their methods
- the strategic kidnapping of artisans
- libraries as treasuries
- dark ages, when chains of transmission break
- renaissances, when lost writings are found

Technology stops being a tree and becomes a *geography*.

---

# 7.2 Responsibilities

The Knowledge domain owns:

- Techniques — identified units of know-how: smelting, crop rotation, double-entry ledgers, fireball casting
- Skill — an individual's practiced proficiency in a technique or craft
- Knowing — the fact that a particular mind holds a particular technique
- Learning and teaching — the deterministic transfer of techniques between minds
- Practice — proficiency growth through validated use
- Discovery — the processes by which genuinely new techniques enter the world
- Recorded knowledge — books, scrolls, schematics: techniques fixed into artifacts
- Loss — the decay of unpracticed skill and the death of untransmitted knowledge

---

# 7.3 Non-Responsibilities

The Knowledge domain does not own:

- **Situational information.** "The bridge is out" is Volume II information. "How to build a bridge" is Knowledge. The test: information describes the world's state; knowledge describes a capability that survives state changes.
- **The performance itself.** Swinging the hammer is an action resolved against Living Systems (strength, stamina) and Physical Reality. Knowledge determines *whether and how well* the technique can be attempted.
- **Recipes' material truth.** What smelting physically consumes and yields is declared in Resources/Economy data. Knowledge governs who *can* perform it.
- **The institutions of learning.** A university is an Institution; a guild is an Institution. This domain provides what they traffic in.
- **Culture's inherited assumptions.** Culture transmits automatically through upbringing; knowledge must be deliberately taught or learned. The distinction is effort.
- **The physical book.** The paper is matter; the schematic inscribed on it is recorded knowledge. Burn the paper and the knowledge dies with it — unless it lives elsewhere.

---

# 7.4 Canonical Concepts

## Technique

An identified unit of transmissible capability:

```text
tech.iron_smelting
Prerequisites: tech.charcoal_burning
Complexity: 0.7
Teachable: yes
Recordable: yes (with literacy)
```

Techniques are world-package data. Their prerequisite graph is that world's "technology tree" — but the tree describes *dependency*, never *inevitability*. Nothing is discovered on schedule.

## Knowing

The atomic fact of the domain:

```text
Knows(Bren, tech.iron_smelting) = true
Proficiency(Bren, tech.iron_smelting) = 0.68
```

Knowledge is always attached to a mind or a record. There is no free-floating "the kingdom knows smelting" — that phrase is shorthand for a query: *does any accessible carrier in the kingdom know it?*

## Skill

Practiced proficiency, grown through validated use and decayed through neglect:

```text
Practice event → Proficiency 0.68 → 0.69
Ten idle years → Proficiency 0.69 → 0.51
```

Skill growth is deterministic, diminishing against inferior challenges, and bounded by the technique's complexity and the practitioner's capabilities.

## Transmission

The deliberate transfer of a technique:

```text
Teaching:  teacher + student + time + shared language → Knows(student)
Study:     record + literacy + time → Knows(reader)
Imitation: observation + practice + time → Knows (degraded fidelity)
```

Every channel costs time, requires access, and can fail. Fidelity varies: a master's teaching transmits more than a stolen glance at her workshop.

## Discovery

The entry of a new technique into the world. World packages declare discovery processes — deliberate research, accidental insight during practice, recombination of held prerequisites — all deterministic given state and seed.

Discovery is rare, local, and singular: it happens to *someone, somewhere*, and then must spread by transmission like everything else.

## Record

Knowledge fixed into an artifact: a book, scroll, tablet, or schematic.

Records decouple knowledge from mortal carriers — at the price of requiring literacy, language, and physical survival of the medium. A library is a civilization's backup; fire is its corruption.

---

# Designer Note
## The Last Smith of Milbrook

Milbrook's smith dies in an autumn plague. His apprentice is fourteen, half-trained: proficiency 0.3, missing the quenching technique entirely.

No system called "dark age" runs. But Milbrook's tools now break faster than they are replaced. Plows dull; harvests thin; the town buys blades from Tolbana at Tolbana's prices. Twenty years later, a Milbrook boy walks to Tolbana to apprentice, and forty years later Milbrook has a forge again — with Tolbana's methods, Tolbana's style, and a debt of gratitude that shapes two towns' politics.

Every step is just the architecture: a death (Living Systems), an unfinished transmission (Knowledge), a price shift (Economy), a journey (Physical Reality), a new social tie (Society).

Knowledge loss is not a penalty mechanic.

It is what mortality does to capability.

---

# 7.5 Domain Interactions

## Consumes

- **Living Systems:** minds to carry knowledge; deaths that end carriers; aptitude bounds on learning
- **Society:** apprenticeship and kinship channels; the settlements where specialists cluster
- **Culture:** language (transmission requires mutual intelligibility); literacy customs; lore that encourages or resists a discovery
- **Economy:** the prosperity that funds teaching time; demand that makes a craft worth learning
- **Information (Vol. II):** the observations from which imitation learning and some discoveries proceed
- **Physical Reality:** the survival conditions of records; the distances teachers travel

## Provides

- **Economy:** the capability side of all production — who can execute which recipes, and how well
- **Conflict:** military technique, fortification knowledge, and the strategic value of captured experts
- **Institutions:** what guilds monopolize, schools teach, and censors suppress
- **Society:** the master–apprentice bond; the prestige of expertise
- **Ecology (indirectly):** extraction techniques that change humanity's pressure on stocks

---

# 7.6 Common Queries

- Who in this settlement knows this technique?
- How proficient is this person at this craft?
- Can this teacher teach this student (language, access, time)?
- What records exist of this technique, and where?
- What techniques are prerequisites of this one?
- Is this technique extinct — no living carrier and no readable record?
- Where is the nearest carrier of this capability?
- What could this person plausibly discover, given what they know?

---

# 7.7 Architectural Contracts

1. All knowledge attaches to carriers: minds or records. No global technology state exists.
2. Transmission is explicit, costly, channel-based, and deterministic.
3. Proficiency grows only through validated practice and decays through neglect, at declared rates.
4. Discovery is a deterministic, seeded, local event — never a global unlock.
5. Records require declared access conditions (literacy, language, possession) to yield knowledge.
6. Technique definitions and prerequisite graphs are world-package data.
7. The death of a carrier removes their knowledge from the world's accessible stock.
8. Capability queries ("can X be done here?") reduce to carrier queries.

---

# 7.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. No knowledge exists without a carrier.
2. Knowing and proficiency are per-mind facts.
3. Transmission consumes time and requires access; it never occurs implicitly.
4. Teaching fidelity exceeds imitation fidelity by declared margins.
5. Skill decays without practice.
6. Discovery is singular, located, and seeded — never scheduled.
7. Records are artifacts subject to Physical Reality in full.
8. A technique with no carriers and no accessible records is extinct until rediscovered.
9. Prerequisite graphs constrain learning and discovery deterministically.
10. Engine code never names a technique.

---

# 7.9 Anti-Patterns

### The Research Bar

Civilization-wide progress meters that unlock capabilities everywhere at once. Deletes geography, transmission, espionage, and loss — the entire drama of knowing.

### Immortal Knowledge

Worlds where nothing learnable is ever lost. Without loss there are no dark ages, no lost arts, no value in libraries, and no reason the old master matters.

### Skill Osmosis

Proficiency gained from time passing rather than validated practice. The swordsman who never draws his blade should rust.

### The Universal Tongue

Ignoring language in transmission. If knowledge crosses every border freely, borders stop meaning anything to capability.

### Recipe Books as Menus

Treating records as instant unlocks on pickup. A stolen schematic should demand literacy, study time, and often prerequisite techniques — otherwise theft replaces education.

---

# 7.10 Future Evolution

Future versions of Cardinal may introduce:

- schools of thought — technique variants with distinct lineages
- espionage models for knowledge theft
- pedagogy quality (some masters teach better than they practice)
- translation and commentary chains for records
- collaborative discovery requiring co-located specialists
- knowledge institutions' internal politics (what gets taught, what gets buried)

Each sharpens the same truth the domain exists to enforce:

Capability is carried, taught, and mortal.

---

## Preparing for the Next Domain

Knowledge arms its carriers.

Scarcity gives them reasons.

The next chapter confronts what happens when reasons collide: **Conflict** — the domain of contested goals, violence, and the destruction that reshapes every other layer of the world.

---

# END OF CHAPTER 7
