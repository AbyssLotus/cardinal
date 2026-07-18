# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 8
# Conflict

> *War is not a game mode. It is what scarcity does when negotiation fails.*

---

# Chapter Overview

Every domain so far can be read as a machine for generating incompatible goals.

Resources are finite, so two parties can want the same vein. Economies concentrate wealth, so the poor can covet and the rich can fear. Societies draw boundaries, so strangers can become enemies. Cultures sanctify, so desecration can demand answer. Knowledge empowers, so capability can tempt.

Conflict is the domain that governs what happens when incompatible goals are pursued by force.

It spans every scale on one architecture: two drunks outside a tavern, a wolf pack defending a kill, a raid on a granary, a siege, a dynastic war. The scales differ in participants and stakes — not in kind.

Crucially, Conflict in Cardinal is not a subsystem that activates when a fight begins.

It is a domain of *standing facts* — hostilities, grievances, threats, and defenses — that exist between fights, shape behavior between fights, and explain why the next fight happens.

---

# 8.1 Purpose

The purpose of the Conflict domain is to make organized violence a consequence rather than a feature.

Fights in Cardinal must have causes discoverable in the world's state: a grievance with provenance, a scarcity with an address, a border with a history. And fights must have consequences that the world absorbs: dead persons with heirs, burned fields with owners, victors with new reputations and new enemies.

The domain therefore owns the full arc:

```text
Incompatibility → Grievance → Hostility → Violence → Outcome → Memory
```

Most of that arc is not combat.

A well-built world spends far more time in armed peace — deterrence, threat, feud, and truce — than in battle. The domain must make the *tension* as real as the *violence*.

---

# 8.2 Responsibilities

The Conflict domain owns:

- Grievances — recorded wrongs between parties, with provenance and severity
- Hostility state — the standing disposition between parties: peace, feud, truce, war
- Threats and deterrence — declared or inferred willingness to use force
- Engagements — the resolution of active violence between participants
- Combat state — positions, stances, wounds-in-progress, morale during an engagement
- Destruction — damage to persons, structures, and holdings as an intended outcome
- Capture, surrender, and rout — the non-lethal terminations of violence
- Truces and terms — negotiated cessations, with their conditions as facts
- Martial organization — bands, warbands, militias, armies as operational structures

---

# 8.3 Non-Responsibilities

The Conflict domain does not own:

- **The combatants' bodies.** Health, stamina, and injury are Living Systems facts. Conflict proposes damage; biology decides what damage means.
- **The weapons.** Arms are items with material properties (Physical Reality) and owners (Economy). Conflict consumes their characteristics.
- **The reasons.** Hunger, greed, honor, and fear originate in other domains. Conflict records the grievance and the hostility, not the underlying want.
- **The sides.** Kin groups, settlements, and peoples are Society; formal war-making authority is Institutions. Conflict binds existing groups into opposition; it never defines the groups.
- **Legality.** Whether a killing is murder, execution, or war is an institutional judgment layered onto the same event.
- **Terrain.** The battlefield is Physical Reality, consumed in full: visibility, cover, elevation, chokepoints, weather.

---

# 8.4 Canonical Concepts

## Grievance

A recorded wrong, the seed of most conflict:

```text
Grievance #2210
Held by: Clan Vess
Against: Milbrook
Cause: event #88121 (grazing-land seizure, Y203)
Severity: 0.7
Satisfied: no
```

Grievances have provenance — they point at real events. They accumulate, compound, transmit through Society's channels (a clan inherits its grudges), and can be *satisfied*: by restitution, by apology customs, by blood.

An unsatisfied grievance is standing fuel.

## Hostility

The authoritative disposition between two parties:

```text
Hostility(Clan Vess, Milbrook) = feud
Hostility(Milbrook, Tolbana) = peace
```

Hostility gates behavior across every domain — trade with an enemy is smuggling, travel through their land is infiltration — and it changes only through events: declaration, incident, negotiation, exhaustion.

## Engagement

A bounded episode of active violence: participants, location, and deterministic round-by-round resolution consuming combatant capability (Living Systems), armament (item facts), and ground (Physical Reality).

Engagements end in death, rout, surrender, or disengagement — every ending an event with consequences.

## Morale

The willingness to continue fighting — a per-combatant fact consuming wounds, leadership, odds, and stakes.

Morale is why most engagements end without annihilation. Beings flee. Fights that always end in extermination are a signature of unsimulated violence.

## Terms

A negotiated cessation as durable fact:

```text
Truce #310
Parties: Clan Vess, Milbrook
Terms: grazing rights ceded; 200 col wergild
Expires: Y210 spring
```

Terms are checkable. Their violation is itself an event — and a new grievance.

---

# Designer Note
## The War Nobody Declared

No one chose the Vess war.

A dry summer (Physical Reality) thinned the pastures. Vess herds edged onto Milbrook's meadow (a trespass — minor grievance). A Milbrook shepherd loosed a dog; a Vess boy was maimed (major grievance, compounding). Milbrook offered no wergild — its elder, new to the role, didn't know the custom (a Knowledge gap in a social position). Vess riders took twelve sheep as self-help restitution; Milbrook called it a raid.

By autumn the hostility fact read *feud*, and every domain had contributed a link: weather, ecology, economy, culture, knowledge, society.

When the historian-player later asks *why* these villages burn each other's barns, the chain is really there — queryable, event by event, back to the dry summer.

That chain is the entire point of this domain.

---

# 8.5 Domain Interactions

## Consumes

- **Physical Reality:** terrain, visibility, cover, fortification, weather — the geometry of every engagement
- **Living Systems:** combatant capability, wounds, death
- **Resources & Economy:** the stakes (what is fought over) and the sinews (what fighting consumes — food, arms, pay)
- **Society:** the sides, their loyalties, and the kin-channels grievances flow through
- **Culture:** honor norms, insult thresholds, restitution customs, rules of feud
- **Institutions:** war-making authority, legality of violence, treaty enforcement
- **Knowledge:** martial technique, fortification craft, command skill
- **Information:** what each side actually knows — scouting, rumor, and the fog that decides battles before they begin

## Provides

- **Physical Reality:** destruction — razed structures, burned fields, corpse-strewn ground
- **Living Systems / Ecology:** death tolls; hunting pressure inverted (predators fought off)
- **Society:** displacement, widowhood, hardened boundaries, veterans' standing
- **Economy:** plunder transfers, blockades, ransoms, and war's insatiable demand
- **Culture:** the battles that become lore, the wounds that become taboos
- **Institutions:** the victories that legitimate and the defeats that topple
- **Information:** events that echo across the world at the speed of rumor

---

# 8.6 Common Queries

- What is the hostility state between these parties?
- What unsatisfied grievances does this party hold, and against whom?
- Who is currently engaged in violence, where?
- What would satisfy this grievance under the relevant culture's customs?
- What force can this settlement raise, and how quickly?
- Which routes are unsafe given current hostilities?
- What terms bind these parties, and are they being honored?
- What did this war cost each side?

---

# 8.7 Architectural Contracts

1. Violence requires cause: every engagement traces to hostility, and hostility to events.
2. Grievances carry provenance, severity, and satisfaction state.
3. Hostility between parties is authoritative, symmetric-or-declared, and changes only by event.
4. Engagement resolution is deterministic, consuming declared facts from Living Systems, Physical Reality, and armament.
5. Damage is proposed by Conflict and applied by the owning domains.
6. Morale is a first-class fact; termination without annihilation is the architectural norm.
7. Terms are durable, checkable facts whose violation generates events.
8. All conflict participants are ordinary entities — no protagonist exceptions.

---

# 8.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. No violence without a traceable cause chain.
2. Grievances are append-only history; satisfaction is an event, not an erasure.
3. Hostility gates cross-domain behavior deterministically.
4. Combat consumes real capability, real armament, real ground.
5. The same resolution architecture spans tavern brawl and siege.
6. Death in conflict is ordinary Living Systems death, with full social consequence.
7. Destruction permanently alters the owning domains' facts.
8. Morale can end any engagement.
9. Terms bind until expiry, satisfaction, or recorded violation.
10. No entity fights without the information to find its enemy.

---

# 8.9 Anti-Patterns

### Aggro Radius Warfare

Hostility as proximity trigger. Enemies must fight because of standing facts, not because someone walked too close.

### The Bloodless Peace

Hostilities that end by timer, leaving no terms, no grievances, no memory. Wars must end the way they start: through events with owners.

### Extermination Default

Every fight to the death. Without morale, rout, and surrender, war has no prisoners, no refugees, no veterans — only counters reaching zero.

### Damage Sponges

Scaling toughness to make fights "last longer." Duration must come from real capability differences, positioning, and morale — not inflated vitals.

### The Instant Army

Forces conjured at declaration. Armies are people with farms unharvested, fed from granaries with addresses, paid from treasuries that empty. Raising them must cost what it costs.

### Consequence-Free Victory

Winning that only adds. Every victory should reshape the victor: casualties, debts, grieving kin, wary neighbors, and a reputation that precedes them.

---

# 8.10 Future Evolution

Future versions of Cardinal may introduce:

- campaign logistics (supply lines as literal goods movement)
- command hierarchies with information lag
- fortification engineering and siegecraft progression
- mercenary markets
- laws of war as institutional overlays
- trauma as a lasting Living Systems consequence

Each raises fidelity without moving the foundation:

Conflict is caused, costly, and remembered.

---

## Preparing for the Next Domain

Feuds burn until something larger than either side contains them.

The next chapter examines those containers: **Institutions** — the domain of offices, laws, and organizations, where authority becomes a fact and violence acquires a monopoly.

---

# END OF CHAPTER 8
