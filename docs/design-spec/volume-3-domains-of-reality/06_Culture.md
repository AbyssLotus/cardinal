# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Chapter 6
# Culture

> *Culture is what a community knows without ever having been told.*

---

# Chapter Overview

Society established that groups exist.

Culture establishes that groups *mean* something.

Two villages with identical geography, identical crops, and identical bloodlines can still be profoundly different places — because one buries its dead facing the river and the other burns them on hilltops; because one shakes hands and the other bows; because a word that honors a guest in one is an insult in the other.

None of these differences are material.

All of them are real.

Culture is the domain of shared meaning: the values, norms, customs, languages, stories, and traditions that a community holds collectively and transmits to its children. It is the slowest-changing layer of the social world — and precisely because it changes slowly, it is the layer against which all faster change is measured.

---

# 6.1 Purpose

The purpose of the Culture domain is to make shared meaning a simulated, owned, queryable fact — so that communities can differ in what they believe, and those differences can drive behavior.

Without this domain, every simulated group behaves identically given identical conditions, and the world collapses into a monoculture with regional costumes.

With it:

A norm against usury shapes a settlement's economy.

A taboo on a forest shapes its ecology.

A tradition of hospitality shapes how strangers survive winter.

A language boundary shapes where information stops.

Culture is how the past constrains the present without anyone enforcing it.

---

# 6.2 Responsibilities

The Culture domain owns:

- Cultures — identified bodies of shared meaning held by communities
- Values — what a culture holds good, shameful, sacred, or profane
- Norms — expected conduct: obligations, prohibitions, taboos
- Customs — practiced forms: rites, greetings, festivals, funerary practice
- Language — which tongues exist, who speaks what, and mutual intelligibility
- Traditions and lore — the stories a culture tells about itself (as cultural content, distinct from what actually happened)
- Cultural membership and transmission — who carries a culture, and how it passes to the next generation
- Drift and divergence — the deterministic processes by which isolated communities grow apart

---

# 6.3 Non-Responsibilities

The Culture domain does not own:

- **Individual belief.** One person's conviction is information. Culture is the *shared* frame the person inherited — an individual can privately doubt everything their culture holds.
- **Enforcement.** A norm violated has social consequences (reputation, in Society) or legal ones (Institutions). Culture defines the expectation, never the punishment.
- **The groups themselves.** Settlements, households, and peoples are Society's. Culture attaches to them.
- **Truth.** A culture's lore may contradict recorded history. Both are stored; only the chronicle is authoritative. The gap between what happened and what is remembered is itself simulation material.
- **Formal doctrine.** A church's official theology, with councils and creeds, is an Institution's product. The folk practice surrounding it is Culture. The two can and should diverge.

---

# 6.4 Canonical Concepts

## Culture

An identified, persistent body of shared meaning:

```text
culture.riverfolk
Values: hospitality, thrift, ancestral continuity
Norms: guest-right (3 days), no felling of grave-trees
Customs: river burial, salt greeting
Language: lang.rivertongue
```

A culture is an entity. It has identity, history, carriers, and can spread, split, merge, and die.

## Value

A weighted orientation toward some dimension of conduct:

```text
Value(hospitality) = 0.9
Value(martial_honor) = 0.2
```

Values are the slow variables of the social world — the parameters decision systems consult when a world package wants riverfolk merchants to behave differently from hill-clan raiders *without different code*.

## Norm

A conduct expectation with declared scope:

```text
norm.guest_right
Applies: anyone sheltering a traveler
Expects: three days' food and safety
Violation: shame (severity 0.8)
```

Norms are facts about expectation. Whether an act violated a norm is computable; what happens next belongs to other domains.

## Custom

A practiced form bound to occasions: weddings, harvests, deaths, meetings.

Customs are behavioral templates that agents of a culture perform at the appropriate trigger. They are the *visible surface* of culture — what a traveler actually witnesses.

## Language

A medium of transmission with speakers and intelligibility relationships:

```text
lang.rivertongue
lang.hilltongue     (intelligibility with rivertongue: 0.4)
```

Language gates information flow. A rumor cannot cross a language boundary without a bilingual carrier — which makes translators, traders, and border villages structurally important without any special code.

## Transmission

Culture propagates through declared, deterministic channels: upbringing (children absorb their household's culture), immersion (long residence shifts carried culture), prestige (communities drift toward cultures they admire), and contact (trade and conquest mix cultural content).

Transmission is slow, generational, and lossy — which is exactly what makes cultural change historical rather than instantaneous.

---

# Designer Note
## The Taboo Forest

Consider a forest no one from Milbrook will enter.

Ecologically, it is just a forest. But three generations ago a hunting party died there, the deaths entered lore, the lore hardened into a taboo, and the taboo now *functions as a conservation law*: game is abundant, timber untouched, and the forest has become exactly the refuge the taboo claims it is.

No system was written for "taboo forests."

A conflict event became a memory, the memory became lore, the lore became a norm, and the norm now redirects hunting pressure — each step owned by a different domain, each step already in the architecture.

This is the payoff of treating culture as simulation rather than flavor text: beliefs, even false ones, reshape the material world through behavior.

---

# 6.5 Domain Interactions

## Consumes

- **Society:** the communities that carry culture; the kinship channels transmission runs along
- **Information:** the events and stories that become lore; the communication that spreads practice
- **Physical Reality:** the geography that isolates (mountains breed divergence) or connects (rivers breed exchange)
- **Conflict:** the wars that mix, scatter, or destroy cultural carriers
- **Economy:** the trade contact that transmits customs alongside goods

## Provides

- **Decision systems:** values and norms as behavioral parameters — the reason two agents in identical situations act differently
- **Society:** the shared identity that binds settlements into peoples; the norm violations that move reputation
- **Institutions:** the legitimacy raw material — law codifies norms that culture already holds
- **Economy:** demand shaped by custom (festival goods, taboo goods, funerary goods)
- **Conflict:** casus belli — desecration, insult, and taboo violation as sparks
- **Knowledge:** the lore layer through which discoveries are interpreted, resisted, or embraced

---

# 6.6 Common Queries

- What culture(s) does this person carry?
- What does this culture hold sacred or forbidden?
- Does this act violate a norm of the witnessing community?
- What custom applies to this occasion in this place?
- Can these two speakers understand each other?
- How culturally distant are these two communities?
- What does this culture's lore say about this place or event?
- Which cultures are present in this settlement, and in what proportion?

---

# 6.7 Architectural Contracts

1. Cultures are entities with identity, carriers, and history.
2. All cultural content — values, norms, customs, languages — is world-package data, never engine code.
3. Individuals carry culture through explicit membership facts; carrying admits degree and plurality.
4. Norm violation is computable from the act, the norm, and the witnesses — consequences are delegated to Society and Institutions.
5. Language intelligibility gates information transmission.
6. Cultural change is deterministic, slow, and channel-based; no culture updates by fiat.
7. Lore is stored as cultural content and never overwrites the chronicle.

---

# 6.8 Engineering Invariants

Every implementation SHALL preserve these rules.

1. Culture attaches to communities and persons, never to the engine.
2. Shared meaning is data; two world packages share no assumed values.
3. Values and norms influence decisions only through declared consumption.
4. Norms define expectation, never enforcement.
5. Lore may be false; reality is never edited to match it.
6. Language boundaries are real barriers to information flow.
7. Cultural transmission follows declared channels at declared rates.
8. Divergence under isolation is deterministic.
9. A culture with no living carriers is dead — recorded, not deleted.
10. No culture is architecturally privileged or "default."

---

# 6.9 Anti-Patterns

### The Monoculture With Hats

Regions that differ in art assets but share every behavioral parameter. If values and norms don't differ, culture isn't simulated — it's wallpaper.

### Culture as Faction

Collapsing culture into political allegiance. A kingdom can contain five cultures; a culture can span three kingdoms. The two graphs must remain independent.

### Instant Assimilation

Migrants adopting local culture on arrival. Transmission is generational; the interesting phenomena — enclaves, creoles, divided loyalties — live in the lag.

### Authored Ancient Lore Only

Treating lore as a fixed backstory database. Lore must *accrue*: the events of the simulation's own history should become next century's legends, distortions included.

### The Norm That Enforces Itself

Building punishment into the norm definition. A norm nobody witnesses breaking has no consequence; enforcement must flow through witnesses, reputation, and institutions, or deception and hypocrisy become impossible.

---

# 6.10 Future Evolution

Future versions of Cardinal may introduce:

- language evolution and dialect chains
- syncretism — deterministic merging of contacted cultures
- prestige dynamics and cultural fashion
- material culture: styles readable from artifacts
- oral-versus-written transmission fidelity models
- generational value shift under prosperity and trauma

Each enriches how meaning moves without changing what the domain is:

Culture is the inheritance nobody signs for.

---

## Preparing for the Next Domain

Culture transmits what a community assumes.

The next chapter examines what a civilization *learns*: **Knowledge** — the domain of skills, techniques, discoveries, and their fragile passage between minds and generations.

---

# END OF CHAPTER 6
