# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 5
# Society

> *A society is not a collection of people. It is a collection of relationships that outlive the moments that created them.*

---

# Chapter Overview

The domains so far describe a world that could exist without anyone caring about anyone else.

Organisms eat, resources deplete, goods change hands — and every interaction could, in principle, be anonymous.

Society is the domain that ends anonymity.

It governs the persistent social relationships between persons: kinship, household, settlement, role, class, and reputation. It is the reason two strangers meeting on a road are not blank slates to one another — they are a miller's daughter and a disgraced soldier, from villages with a history, carrying names that arrived before they did.

Society transforms a population into a people.

---

# 5.1 Purpose

The purpose of the Society domain is to make social structure a simulated fact rather than a narrative flavor.

Where organisms cluster, structure forms:

Families pool resources and raise young.

Settlements concentrate shelter, defense, and exchange.

Roles divide labor.

Reputation accumulates as a social memory of conduct.

Each of these is a pattern of relationships that persists beyond any single interaction — and because it persists, it can be stored, queried, inherited, damaged, and repaired.

Society exists so that *who you are to others* is part of reality.

---

# 5.2 Responsibilities

The Society domain owns:

- Kinship — parentage, marriage, lineage, and the relationship graph of family
- Households — co-residing groups that pool holdings and labor
- Settlements — persistent communities bound to place: hamlets, villages, towns, cities
- Membership — which persons belong to which communities and groups
- Roles — socially recognized positions: smith, elder, midwife, guard
- Social standing — class, status, and stratification within a community
- Reputation — the aggregate social memory of an entity's conduct, per community
- Demographics as structure — the composition of communities (not the vital events themselves, which Living Systems owns)

---

# 5.3 Non-Responsibilities

The Society domain does not own:

- **The persons themselves.** A person is an organism (Living Systems) with knowledge (Information) and holdings (Economy). Society layers relationships onto them.
- **Formal power.** A village elder's *respect* lives here. A magistrate's *office* — with defined authority and succession — belongs to Institutions.
- **Shared meaning.** Why a community mourns in white or shuns moneylending is Culture. Society records that the group exists; Culture records what it believes together.
- **Individual belief.** One person's opinion of another is information. Reputation is the *social aggregate* of many such opinions, and only that aggregate lives here.
- **The buildings.** A settlement's houses and walls are Physical Reality. The settlement is the community, not the masonry — a city razed to the ground and rebuilt is the same city.

---

# 5.4 Canonical Concepts

## Person

An organism that participates in social relationships.

Nothing about the engine distinguishes persons from other organisms except the relationships attached to them. A world package decides which species are persons — and may decide that dragons, ships' AIs, or ancestral spirits qualify.

## Kinship

The relationship graph of family:

```text
parent_of
married_to
sibling_of
descends_from
```

Kinship is permanent history. A marriage may end; *having been married* never does. Lineage makes inheritance, feuds, dynasties, and genealogy possible.

## Household

The smallest economic-social unit: persons who dwell together and pool holdings.

```text
Household #83
Members: Bren, Sela, two children
Dwelling: house #1204, Milbrook
Holdings: pooled
```

The household is where Society and Economy meet. Most production, consumption, and inheritance in a pre-industrial world clears through households, not individuals.

## Settlement

A persistent community bound to a location.

A settlement has members, a place, internal structure, and a name that travels. It is an entity with identity: it can grow, shrink, split, absorb neighbors, and die — and its death (abandonment) is an event the chronicle records.

## Role

A socially recognized position filled by a person: the village has *a* smith, and Bren *is* him.

Roles create expectation. Others plan around the role's existence, which is precisely what makes a role-holder's death a social event and not merely a biological one.

## Reputation

A community's aggregate memory of an entity's conduct:

```text
Reputation(Kell, Milbrook) = +0.62   (trusted)
Reputation(Kell, Tolbana)  = -0.31   (suspected)
```

Reputation is local, like price. It is earned where deeds are witnessed and spreads only as information spreads — a person can be a hero in one valley and a stranger in the next.

---

# Designer Note
## The Village Remembers

An individual's memory dies with them.

A society's does not.

When Kell cheats a merchant in Milbrook, the merchant's memory is information — fallible, decaying, personal. But as the story is told, retold, and absorbed into how Milbrook *treats* Kell, it becomes reputation: a social fact that persists even after every original witness is dead.

This is the architectural reason reputation lives in Society rather than in individual memory. A community is a memory institution. Its grudges and gratitudes are among the longest-lived facts in the simulation — and the richest source of emergent story.

---

# 5.5 Domain Interactions

## Consumes

- **Living Systems:** births, deaths, and maturation — the vital events that reshape families and settlements
- **Physical Reality:** habitable geography, water, defensibility — the logic of where settlements form
- **Resources:** the local stocks that make a place worth settling
- **Economy:** wealth distributions that stratify; markets that draw communities together
- **Information:** the spread of deeds that feeds reputation
- **Conflict:** the violence that displaces populations and hardens group boundaries

## Provides

- **Economy:** households as economic units; settlements as market sites; trust as the substrate of credit
- **Institutions:** the communities over which formal authority is claimed; the notables from whom officials are drawn
- **Culture:** the groups within which shared meaning forms and transmits
- **Conflict:** the sides — kin groups, settlements, and peoples with loyalties worth fighting for
- **Knowledge:** apprenticeship structures through which skills pass between generations

---

# 5.6 Common Queries

- What settlements exist in this region, and how large is each?
- Who belongs to this household, and what does it hold?
- How is this person related to that person?
- Who fills this role in this settlement?
- What is this entity's reputation with this community?
- Which families hold standing in this town?
- Who are this person's heirs?
- Which settlements have ties — kinship, trade, feud — to this one?

---

# 5.7 Architectural Contracts

1. Social relationships are first-class facts with provenance and history.
2. Kinship, once established, is permanent history; only its present status changes.
3. Settlements and households are entities with identity independent of their members and their buildings.
4. Membership is explicit — no person belongs to a community by implication.
5. Reputation is per-community, derived from witnessed and transmitted deeds, and decays deterministically.
6. Roles are relationships between a person and a community, not properties of the person.
7. No social structure is hardcoded — the world package defines which structures exist and how they form.

---

# 5.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Persons are ordinary organisms plus relationships — never a privileged type.
2. Kinship history is append-only.
3. Settlement identity survives change of members, place, and buildings.
4. Households pool holdings only through recorded membership.
5. Reputation is local to a community, never global.
6. Reputation changes only through information that actually reached the community.
7. Roles bind persons to communities and are vacated by recorded events.
8. Social stratification derives from held facts, never authored labels.
9. Group boundaries (member / stranger) are queryable facts.
10. The death of a person is a social event wherever that person held relationships.

---

# 5.9 Anti-Patterns

### The Global Faction Meter

One reputation number per faction, shared by every member. A person should be able to be loved by a guild's dockworkers and hated by its masters; aggregating past the community level destroys the texture that makes social play possible.

### Cardboard Settlements

Towns as static scenery with respawning shopkeepers. A settlement must be a living roster — when the smith dies childless, the village *has no smith*, and that absence should ripple.

### Instant Reputation

Deeds updating opinion worldwide at the moment they occur. Reputation must travel as information travels: by road, at the speed of gossip, distorting as it goes.

### Kinship Amnesia

Discarding family graphs to save space. Lineage is among the cheapest data and the richest story fuel the simulation owns — inheritance disputes, blood feuds, and dynasties are all queries over the kinship graph.

### Society as Quest Dispenser

Modeling communities only as sources of tasks for a player. Settlements must have full lives — marriages, harvests, funerals, rivalries — whether or not anyone is watching.

---

# 5.10 Future Evolution

Future versions of Cardinal may introduce:

- migration modeling and settlement fission
- social network analysis as a first-class query layer
- naming systems and genealogical records as in-world artifacts
- class mobility dynamics
- honor and shame economies
- diaspora communities retaining ties across distance

Each deepens social texture without altering the domain's charge:

Society keeps the relationships that outlive the moment.

---

## Preparing for the Next Domain

A society knows *who* its members are.

It does not yet know *what they hold sacred*.

The next chapter introduces **Culture** — the domain of shared meaning: values, customs, language, and tradition, the invisible inheritance every community passes to its children.

---

# END OF CHAPTER 5
