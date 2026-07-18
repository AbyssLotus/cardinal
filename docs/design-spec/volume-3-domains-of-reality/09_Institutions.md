# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 9
# Institutions

> *An institution is a promise that outlives the people who made it.*

---

# Chapter Overview

Society gave the world informal structure: families, settlements, standing, reputation.

Institutions give it *formal* structure.

A village elder is respected; a magistrate holds an office. The difference is not the person — it is that the office exists independently of its holder, carries defined authority, survives succession, and can be occupied by someone nobody respects at all.

The Institutions domain governs formal organizations and formal rules: governments, courts, guilds, temples, charters, laws, offices, and treaties. It is the domain of authority made explicit — power that claims to bind people who never personally agreed to it.

Institutions are also the simulation's longest levers. A law changes the behavior of thousands. A tax reroutes an economy. A succession crisis turns one death into a civil war. No other domain converts single events into consequences at this scale.

---

# 9.1 Purpose

The purpose of the Institutions domain is to make formal authority a simulated fact with the same rigor as position or price.

That requires holding three things apart that most simulations blur:

**Claimed authority** — what an institution asserts the right to do.

**Effective authority** — what it can actually enforce, given its agents, reach, and resources.

**Legitimacy** — the degree to which those subject to it accept its claims.

A king may claim the forest (claimed), patrol only half of it (effective), and be obeyed in none of it (legitimacy). The gaps between the three are where most political history happens: smuggling lives in the enforcement gap, rebellion in the legitimacy gap, and empire-building in the campaign to close both.

An architecture that stores only "faction controls region" can represent none of this.

---

# 9.2 Responsibilities

The Institutions domain owns:

- Institutions — chartered organizations with purpose, structure, and persistence: crowns, councils, courts, guilds, temples, banks
- Offices — defined positions with authority, occupied by persons, surviving succession
- Laws and rules — explicit, recorded norms with declared scope, subjects, and sanctions
- Jurisdiction — the claimed scope of an institution's authority over places, persons, and activities
- Legitimacy — per-community acceptance of an institution's claims
- Enforcement capacity — the agents, reach, and resources available to make claims effective
- Adjudication — the formal resolution of disputes: verdicts, sentences, precedents
- Treaties and charters — formal agreements between institutions, as durable facts
- Succession — the recorded rules by which offices pass between holders

---

# 9.3 Non-Responsibilities

The Institutions domain does not own:

- **The people.** Officials are ordinary persons — with hungers, kin, grudges, and prices. The office is the domain's; the holder never is.
- **Informal norms.** Culture's unwritten expectations bind without offices. Law begins where a norm is *recorded, scoped, and sanctioned* by an institution.
- **The violence itself.** A sentence of execution is institutional; the killing is Conflict resolving against Living Systems. Institutions authorize; they do not resolve.
- **The treasury's coins.** Institutional holdings are ordinary Economy holdings whose owner is an institution. Taxation is a transfer with legal provenance — architecturally, a transfer like any other.
- **Reverence.** A temple's institutional hierarchy lives here; the faith it serves is Culture. Congregations can keep believing after the temple falls, and temples can outlive every believer's sincerity.
- **Territory as terrain.** Jurisdiction is a claim *about* places, layered over Physical Reality. The border is institutional; the river it follows is not.

---

# 9.4 Canonical Concepts

## Institution

A chartered organization persisting independently of its members:

```text
inst.milbrook_council
Purpose: govern Milbrook
Offices: elder (3), reeve (1)
Jurisdiction: Milbrook and common lands
Treasury: holding (Economy)
Founded: Y187 (event #71002)
```

Institutions are entities. They are founded, they act through office-holders, they merge, split, reform, and dissolve — every transition an event.

## Office

A defined position, distinct from any occupant:

```text
office.reeve_of_milbrook
Authority: collect tolls, arrest, convene court
Held by: Sela (since Y204)
Succession: council appointment
Vacant: no
```

The office persists through vacancy. "The reeve is dead and no successor is named" is a precise, queryable, dangerous state of the world.

## Law

An explicit rule with scope and sanction:

```text
law.milbrook_grazing
Enacted by: inst.milbrook_council, Y196
Subjects: all herders within jurisdiction
Rule: no grazing on the east meadow before midsummer
Sanction: fine, 5 col per head
```

A law is a fact about what an institution *declares*, not about what happens. Whether the law is known (Information), obeyed (decisions), or enforceable (capacity) are separate questions — and the distance between them is simulation, not noise.

## Jurisdiction

The claimed scope of authority. Jurisdictions overlap, and their overlaps are real: the crown claims the road, the guild claims the trade on it, the temple claims the pilgrims traveling it. Contested jurisdiction is a standing fact other domains can consume — and a war waiting for a spark.

## Legitimacy

Per-community acceptance of an institution's right to rule:

```text
Legitimacy(crown, Milbrook) = 0.71
Legitimacy(crown, hill_clans) = 0.22
```

Legitimacy is earned and spent by events — justice delivered, taxes raised, wars won and lost — and it is the multiplier on every act of enforcement. Below some threshold, each attempted enforcement *generates* grievance faster than compliance.

## Succession

The recorded rule by which an office passes. Succession rules are checkable; a death that satisfies no rule produces a *vacancy with rival claimants* — the institutional structure of a crisis, generated rather than authored.

---

# Designer Note
## The Law Is Not the World

It is tempting to implement law as constraint: forbid the action at the interface.

Cardinal must never do this.

If theft is impossible, honesty means nothing — and thieves, fences, smugglers, corrupt reeves, and every story about justice become unrepresentable. The law is a *fact about consequences*, not a wall: the act remains possible; the institution responds if it can, where it can, as consistently as its capacity and legitimacy allow.

The gap between what is forbidden and what is prevented is not a bug in the design.

It is the habitat of half of literature.

---

# 9.5 Domain Interactions

## Consumes

- **Society:** the communities ruled, the notables who fill offices, the reputations that seed legitimacy
- **Culture:** the norms law codifies, the customs succession follows, the values that make a claim feel rightful
- **Economy:** the tax base, the treasuries, the wealth that buys enforcement — and buys officials
- **Conflict:** the force behind sanction; the wars that redraw jurisdiction
- **Knowledge:** literacy for records, law as a learned technique, administration as craft
- **Information:** what the institution actually knows — crimes unreported are crimes unpunished

## Provides

- **Economy:** property enforcement, contract adjudication, currency issuance, taxation — the scaffolding of scale
- **Conflict:** declared wars, legal violence, treaty constraints — and the legitimacy gaps rebels exploit
- **Society:** the offices that stratify; the justice (or its absence) that reshapes reputation
- **Culture:** the institutional canon that folk practice drifts against
- **Knowledge:** guild monopolies, schools, censorship — the gates on transmission
- **Information:** proclamations, records, and registries — institutional memory as an information source

---

# 9.6 Common Queries

- What institutions claim jurisdiction here, and do their claims conflict?
- Who holds this office, and by what succession rule?
- What laws bind this person, in this place, doing this act?
- What is this institution's legitimacy with this community?
- Can this institution enforce this law here — with what agents, at what reach?
- What treaties bind these institutions, and are they honored?
- Whose claim to this vacant office is strongest under the recorded rules?
- What has this court decided in matters like this one?

---

# 9.7 Architectural Contracts

1. Institutions and offices are entities with identity, history, and succession independent of persons.
2. Claimed authority, effective capacity, and legitimacy are stored separately and never conflated.
3. Laws are recorded facts with enacting institution, scope, subjects, and sanction.
4. Law never prevents action at the engine level; it defines institutional response to action.
5. Enforcement requires real agents, real information, and real reach, and is resolved through the ordinary domains.
6. Legitimacy is per-community, event-driven, and deterministic.
7. Institutional acts — enactment, verdict, appointment, treaty — are events with full provenance.
8. Treasuries, agents, and properties of institutions are ordinary facts in their owning domains.

---

# 9.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Offices survive their holders; vacancy is a first-class state.
2. No institution acts except through persons holding its offices.
3. Claimed, effective, and legitimate authority may always diverge.
4. Law binds only where jurisdiction is claimed; it prevents nothing physically.
5. Enforcement consumes capacity and information; unwitnessed acts go unsanctioned.
6. Legitimacy changes only through events.
7. Succession follows recorded rules; unruled successions produce claimant states, not resolutions.
8. Treaties and charters are durable, checkable, violable facts.
9. Institutional wealth and force are ordinary domain facts with an institutional owner.
10. No institution is architecturally sovereign — the engine takes no side.

---

# 9.9 Anti-Patterns

### The Faction Blob

One monolithic "faction" owning territory, troops, diplomacy, and ideology as a single object. Crowns, courts, guilds, and temples must be separable institutions that can conflict *within* a polity — most politics is internal.

### Law as Physics

Making forbidden acts impossible. Deletes crime, corruption, and justice in one stroke.

### The Omniscient Magistrate

Courts that know every fact of a case. Adjudication must run on testimony, records, and inference — fallible information — or trials are just lookups with pageantry.

### Legitimacy by Fiat

A "government approval" slider moved by script. Legitimacy must be earned and spent through witnessed events, community by community.

### The Immortal Empire

Institutions without maintenance costs, succession risk, or legitimacy decay. Institutions must be expensive promises — kept, renegotiated, or broken — or the map's borders become geology.

### Puppet Officials

Office-holders without personal interests. The reeve who takes bribes, favors his kin, and fears the council is not a corruption *feature* — he is what an official *is* when officials are real people.

---

# 9.10 Future Evolution

Future versions of Cardinal may introduce:

- deliberative bodies with recorded votes and blocs
- legal precedent as accumulated institutional knowledge
- bureaucratic capacity as a modeled resource
- federated and nested sovereignty structures
- institutional reform movements from within
- diplomacy as continuous institutional conversation

Each extends the reach of formal order without changing its nature:

An institution is a promise, and promises are facts.

---

## Preparing for the Next Domain

Beneath every institution, economy, and war, the living world has been keeping its own accounts.

The next chapter returns to it at full scale: **Ecology** — the domain of populations, food webs, and the slow systemic balances that human ambition disturbs at its peril.

---

# END OF CHAPTER 9
