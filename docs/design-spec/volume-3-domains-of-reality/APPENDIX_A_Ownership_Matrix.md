# Cardinal Architecture Specification
# Volume III
# Domains of Reality
## Appendix A
# The Ownership Matrix

> *One fact, one owner. Everything else reads.*

---

# Purpose

Chapter 12 requires that "the ownership matrix is explicit and complete" — that every fact
type in a Cardinal world has exactly one owning domain, recorded in the specification
before it is implemented.

This appendix *is* that record for the domains defined in Volume III.

It has two parts. Part 1 is the matrix itself: fact categories, their owners, and their
principal consumers. Part 2 records the settled boundary rulings — the cases where
ownership was genuinely arguable, the decision made, and the reason. These rulings are
precedent: when a new boundary question arises, argue from them.

---

# Part 1 — The Matrix

| Fact category | Owner | Principal consumers |
|---|---|---|
| Position, orientation, adjacency | Physical Reality | all |
| Topology and connectivity | Physical Reality | Economy, Conflict, Ecology, Information |
| Regions and containment | Physical Reality | all |
| Material composition and properties | Physical Reality | Resources, Conflict, Knowledge |
| Environmental state (temperature, water, light, weather) | Physical Reality | Living Systems, Ecology, Economy |
| Physical constraints (blocking, visibility, passability) | Physical Reality | Conflict, Information, Economy |
| Vital state (health, energy, needs) | Living Systems | Conflict, decision systems |
| Metabolism, growth, lifecycle stage | Living Systems | Ecology, Society |
| Sensory and physical capability | Living Systems | Information, Conflict, Knowledge |
| Inheritance (biological traits) | Living Systems | Ecology |
| Death (the vital event) | Living Systems | Society, Institutions, Conflict, Ecology |
| Resource definitions, deposits, grades | Resources | Economy, Society, Conflict |
| Stock quantities (non-living) | Resources | Economy, Institutions |
| Stock quantities (living) | **Ecology** (Resources exposes a view) | Resources, Economy |
| Regeneration and depletion rules | Resources | Economy, Ecology |
| Holdings (who possesses what) | Economy | Society, Institutions, Conflict |
| Transfers, exchanges, cleared prices | Economy | Institutions, Information |
| Money and its circulation | Economy (issuance authorized by Institutions) | all social domains |
| Production recipes (material truth) | Economy/Resources data | Knowledge (capability to execute) |
| Markets and their local price memory | Economy | Society, Information |
| Kinship and lineage | Society | Institutions (succession), Culture |
| Households and settlements | Society | Economy, Institutions, Ecology |
| Roles and social standing | Society | Institutions, Culture |
| Reputation (community aggregate) | Society | decision systems, Institutions |
| Group membership | Society | Culture, Conflict, Institutions |
| Values, norms, customs | Culture | decision systems, Institutions, Society |
| Language and intelligibility | Culture | Information, Knowledge |
| Lore (what a culture remembers) | Culture | Information, decision systems |
| Cultural membership and transmission | Culture | Society |
| Techniques and prerequisite graphs | Knowledge (definitions are world-package data) | Economy, Conflict, Institutions |
| Knowing and proficiency (per mind) | Knowledge | Economy, Conflict |
| Records (inscribed knowledge content) | Knowledge (the physical medium is Physical Reality) | Institutions, Culture |
| Discovery state | Knowledge | Information |
| Grievances and hostility state | Conflict | Society, Institutions, decision systems |
| Engagements, combat state, morale | Conflict | Living Systems (receives damage proposals) |
| Truces and terms | Conflict | Institutions, Society |
| Institutions, offices, succession rules | Institutions | Society, Conflict, Economy |
| Laws, jurisdiction, sanctions | Institutions | decision systems, Conflict, Economy |
| Legitimacy (per community) | Institutions | Conflict, Society |
| Treaties and charters | Institutions | Conflict, Economy |
| Populations (aggregate counts) | Ecology | Resources, Living Systems |
| Food webs and carrying capacity | Ecology | Resources, Society |
| Migration and succession (habitat) | Ecology | Physical Reality, Society |
| Observation, knowledge, memory, belief (individual) | Information layer (Vol. II Ch. 4) | decision systems, all |
| Events and the chronicle | Persistence/History (Vol. II Ch. 5) | all; tooling |

Categories marked as world-package *data* (recipes, technique definitions, cultural
content) are authored content; the owning domain governs their runtime state and
semantics.

---

# Part 2 — Settled Boundary Rulings

Each ruling names the dispute, the decision, and the reason. These are normative.

## Ruling 1 — Living stocks: Ecology owns the count

**Dispute:** A fishery is Ecology's population and Resources' stock. Who owns the number?

**Decision:** Ecology owns the single authoritative count. Resources exposes a derived
*view* of it as an extractable stock. Harvest debits and predation debits flow through the
same accounting.

**Reason:** Two ledgers for one school of fish is a divergence factory. With one ledger,
overfishing automatically starves predators and collapses habitats — the consequence is
free. (Ch. 3 §3.5, Ch. 10 §10.4, Ch. 12 §12.1.)

## Ruling 2 — Reputation: Society owns the aggregate, Information owns the opinion

**Dispute:** Is reputation a social fact or personal belief?

**Decision:** Both exist, separately. One person's opinion of another is individual
information (fallible, decaying, personal). Reputation is the per-community *aggregate* of
witnessed and transmitted deeds, owned by Society. Reputation changes only through
information that actually reached the community.

**Reason:** A person must be able to privately trust someone the town shuns. Collapse the
two and either individuality or social memory disappears. (Ch. 5 §5.4.)

## Ruling 3 — Norm vs. law: Culture defines expectation, Institutions define sanction

**Dispute:** Where does an unwritten custom end and a law begin?

**Decision:** A norm — an expectation with social consequence — is Culture. The moment a
rule is *recorded, scoped, and sanctioned by an institution*, that recorded rule is
Institutions'. The norm does not move; the law is a second, separate fact that may drift
from it.

**Reason:** Law and custom diverging (a law nobody respects; a custom no court enforces)
is half of political history. One fact cannot diverge from itself. (Ch. 6 §6.3, Ch. 9 §9.3.)

## Ruling 4 — Hunger vs. eating: biology never decides

**Dispute:** Should needs drive behavior directly?

**Decision:** The need (hunger) is Living Systems. The knowledge of where food is, is
Information. The decision to go get it is a decision system. The travel is Physical
Reality. No stage may be skipped.

**Reason:** Collapse the pipeline and organisms navigate to food they have never seen —
omniscient stomachs. Keep it and fear, ignorance, and sacrifice become possible: an animal
can starve rather than cross open ground. (Ch. 2 §2.4, Vol. II Ch. 4.)

## Ruling 5 — Settlement vs. buildings: the community is not the masonry

**Dispute:** Is a city its structures?

**Decision:** The buildings are Physical Reality. The settlement — the persistent
community bound to a place — is Society's entity, with its own identity and history.

**Reason:** A city razed and rebuilt is the same city; a city abandoned intact is a dead
one. Identity follows the community, not the stone. (Ch. 5 §5.3.)

## Ruling 6 — Lore vs. chronicle: memory never edits history

**Dispute:** What happens when a culture's stories contradict recorded events?

**Decision:** The chronicle (events, owned by the persistence/history layer) is
authoritative and immutable. Lore is Culture's content and may be wrong. Both are stored;
neither overwrites the other.

**Reason:** The gap between what happened and what is remembered is simulation material —
legends, taboos, propaganda. Editing reality to match belief violates Volume I's prime
directive. (Ch. 6 §6.3, Vol. II Ch. 1.)

## Ruling 7 — Office vs. holder: Institutions own the chair, never the person

**Dispute:** Is a magistrate an institutional object?

**Decision:** The office — authority, succession rule, vacancy state — is Institutions'.
The holder is an ordinary person (organism + relationships + holdings) who occupies it.

**Reason:** Offices must survive their holders (vacancy, succession crises), and holders
must keep private interests (corruption, divided loyalty). Merge them and both phenomena
vanish. (Ch. 9 §9.4.)

## Ruling 8 — Knowledge vs. information: capability vs. state

**Dispute:** "The bridge is out" and "how to build a bridge" both feel like knowledge.

**Decision:** Information (Vol. II) describes the world's current state and decays with
it. Knowledge (Ch. 7) is transmissible capability that survives state changes. Techniques,
skills, and records are Knowledge; observations, memories, and beliefs are Information.

**Reason:** They obey different lifecycles: information goes stale when the world changes;
knowledge dies only with its carriers. One architecture cannot honor both decay laws.
(Ch. 7 §7.3.)

## Ruling 9 — Damage: Conflict proposes, owners apply

**Dispute:** When combat wounds a soldier and burns a wall, who writes those facts?

**Decision:** Conflict resolves engagements and *proposes* changes. Living Systems applies
wounds under its own rules; Physical Reality applies structural damage under its own
rules. Conflict never writes another domain's facts.

**Reason:** The owner's coherence rules must judge every change regardless of source —
otherwise combat becomes a second authority on bodies and walls. (Ch. 8 §8.3, Ch. 12
§12.1.)

## Ruling 10 — Money: Economy circulates, Institutions authorize

**Dispute:** Who owns currency?

**Decision:** Money in circulation is ordinary Economy holdings, conserved through
transfers. Its *issuance* (minting, debasement) is an explicit institutional act.

**Reason:** Conservation must be the Economy's invariant, and violating it must require a
recorded act of power — never a side effect. (Ch. 4 §4.4, Ch. 9 §9.3.)

## Ruling 11 — The record vs. the book: content and medium part ways

**Dispute:** Is a schematic an item or knowledge?

**Decision:** The physical medium (paper, tablet) is matter owned by Physical Reality,
held via Economy. The inscribed content is a Knowledge record with its own access
conditions (language, literacy).

**Reason:** Burn the paper and the content dies *only if it lives nowhere else* — the two
facts have different survival conditions, so they are two facts. (Ch. 7 §7.4.)

## Ruling 12 — Individuation: one organism, one owner at a time

**Dispute:** When a deer steps out of a population into an encounter, who owns it?

**Decision:** While aggregate, the deer exists only in Ecology's population accounts.
While instantiated, it is fully a Living Systems organism. The seam reconciles exactly in
both directions; a death while instantiated debits the population.

**Reason:** Double-existence (counted *and* instantiated) double-counts every consequence.
The seam is explicit so the books always balance. (Ch. 10 §10.4.)

---

# Amending This Matrix

New fact types, new domains, and genuinely new boundary disputes will arise.

The procedure is Chapter 12's, restated:

1. A proposed fact type is checked against this matrix for overlap.
2. If ownership is arguable, the dispute is settled *here, in writing, with rationale*
   before implementation — arguing from the rulings above as precedent.
3. The matrix is updated in the same change that introduces the fact type.

An implementation that contradicts this appendix is wrong, however well it works.

A ruling that proves mistaken is amended here first, with its own rationale, and
implementations follow.

---

# END OF APPENDIX A
