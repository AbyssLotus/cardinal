# Project Cardinal — Design Specification

**Version:** 1.0
**Audience:** Claude Code (implementation agent)
**Status:** Authoritative implementation spec derived from the Foundational Project Brief

---

## 1. Purpose & Scope

Project Cardinal is a **persistent world simulation engine** that recreates the experience of living inside a functioning virtual MMORPG. The first world package recreates *Sword Art Online*'s Aincrad, but the engine is world-agnostic.

This is **not** a chatbot, roleplay assistant, or story generator. It is a simulation framework in which narrative emerges from system interaction.

### 1.1 Prime Directives (non-negotiable invariants)

1. **Simulation before narration.** Every player action is resolved by the simulation engine first. The narrator layer only describes state changes that have already been computed and committed. Narration NEVER creates, modifies, or invents world state.
2. **The world persists.** All world state survives process restarts, and off-screen regions continue to advance when time passes. Nothing resets when the player leaves an area.
3. **No player protection.** No level scaling, no rubber-banding, no guaranteed success, no plot armor, no convenience spawns. World rules apply equally to the player and every NPC.
4. **Engine / World Package separation.** The engine contains zero Aincrad-specific content. All content lives in data files under a world package. Loading a different world package must require no engine code changes.
5. **Canon discipline.** Generated content must never contradict confirmed canon (see §12).

### 1.2 The Simulation Loop (the heart of the system)

Every player input follows this exact pipeline. Implement it as a literal, inspectable pipeline — not an implicit LLM behavior.

```
Player input (natural language or command)
   │
   ▼
1. INTERPRET   — parse input into one or more Action objects (intent, target, parameters)
2. VALIDATE    — check the action against world rules, player state, position, skills
3. COST        — compute time cost, resource cost, risk
4. ADVANCE     — advance the world clock by the time cost
5. TICK        — run all system updates for the elapsed time (NPCs, economy, weather,
                 monsters, quests, factions) — for ALL regions, not just the player's
6. RESOLVE     — resolve the player's action against the updated world state
                 (combat rounds, skill checks, dialogue outcomes, market transactions)
7. COMMIT      — write all resulting state deltas to the persistence layer (atomic)
8. NARRATE     — the narrator renders the committed deltas visible to the player
                 into prose, revealing ONLY what the player can perceive
```

Steps 1–7 are deterministic given a seed. Step 8 is generative but read-only.

---

## 2. Technology Stack

| Concern | Choice | Rationale |
|---|---|---|
| Engine language | **Python 3.11+** | Fast iteration, rich data tooling, Claude Code fluency |
| World package data | **YAML** (authored) → validated to JSON Schema | Human-editable content, strict validation |
| Persistent state | **SQLite** (single file per save) | Atomic transactions, queryable history, zero-config |
| Schemas / models | **Pydantic v2** | Runtime validation of every data object |
| RNG | `random.Random` seeded per-save, with named substreams | Reproducibility, replayable saves |
| Narrator | Anthropic API (pluggable `Narrator` interface) | Read-only prose layer; engine runs headless without it |
| Interface | CLI first (`cardinal play <save>`) | UI-agnostic engine; other frontends later |
| Tests | pytest | Every system must be testable headless (no LLM) |

**Hard rule:** the engine must run a full simulation tick with the narrator disabled. LLM output is never a dependency of state computation.

---

## 3. Repository Layout

```
cardinal/
├── pyproject.toml
├── README.md
├── CARDINAL_DESIGN_SPEC.md          # this document
│
├── engine/                          # world-agnostic simulation engine
│   ├── __init__.py
│   ├── core/
│   │   ├── clock.py                 # world time, calendar, scheduling (§6)
│   │   ├── loop.py                  # the 8-step simulation loop (§1.2)
│   │   ├── rng.py                   # seeded RNG substreams
│   │   ├── events.py                # internal event bus (system → system signals)
│   │   └── registry.py             # loads & indexes world package data
│   ├── actions/
│   │   ├── parser.py                # natural language → Action objects
│   │   ├── actions.py               # Action type definitions
│   │   └── validator.py             # legality checks against world rules
│   ├── systems/                     # each system = one module w/ tick() + handlers
│   │   ├── npc.py                   # NPC agents: goals, schedules, decisions (§7)
│   │   ├── combat.py                # positional combat resolution (§8)
│   │   ├── skills.py                # skill usage, proficiency growth (§9)
│   │   ├── items.py                 # durability, crafting, upgrades (§10)
│   │   ├── economy.py               # supply/demand, pricing, merchants (§11)
│   │   ├── quests.py                # quest lifecycle & expiry (§13)
│   │   ├── factions.py              # guild politics, relationships
│   │   ├── ecology.py               # monster populations, migration, respawn
│   │   ├── weather.py               # weather & seasons
│   │   └── worldevents.py           # emergent events from system thresholds
│   ├── memory/
│   │   ├── player_memory.py         # §5.1
│   │   ├── npc_memory.py            # §5.2
│   │   └── world_memory.py          # §5.3 (chronicle)
│   ├── persistence/
│   │   ├── store.py                 # SQLite layer, atomic commit of deltas
│   │   ├── schema.sql               # DB schema (§14)
│   │   └── migrations/
│   ├── narrator/
│   │   ├── base.py                  # Narrator interface (render(deltas, perception))
│   │   ├── llm_narrator.py          # Anthropic-backed prose renderer
│   │   ├── plain_narrator.py        # deterministic text renderer (for tests)
│   │   └── perception.py            # computes what the player can actually see (§15)
│   └── canon/
│       └── guard.py                 # canon-tier tagging & contradiction checks (§12)
│
├── worlds/                          # world packages (content only, no code)
│   └── aincrad/
│       ├── world.yaml               # manifest (§4.1)
│       ├── rules.yaml               # world-rule constants (§4.2)
│       ├── canon/
│       │   ├── confirmed.yaml       # tier-1 canon facts
│       │   └── inferences.yaml      # tier-2 reasoned extensions
│       ├── floors/
│       │   ├── floor_001.yaml       # one file per floor (§4.3)
│       │   └── ...
│       ├── locations/               # cities, dungeons, landmarks (§4.4)
│       ├── npcs/                    # one file per named NPC + archetype files (§4.5)
│       │   ├── archetypes/
│       │   └── named/
│       ├── monsters/                # species definitions (§4.6)
│       ├── items/                   # item definitions by category (§4.7)
│       ├── skills/                  # skill definitions (§4.8)
│       ├── quests/                  # authored quest templates (§4.9)
│       ├── factions/                # guilds & organizations (§4.10)
│       ├── economy/
│       │   ├── markets.yaml         # market definitions per settlement
│       │   └── base_prices.yaml
│       └── lore/                    # freeform lore docs the narrator may draw on
│
├── saves/                           # runtime state (gitignored)
│   └── <save_name>/
│       ├── state.db                 # SQLite world state
│       └── meta.yaml                # world package + version + seed
│
├── cli/
│   └── main.py                      # `cardinal new|play|tick|inspect`
│
└── tests/
    ├── test_loop.py
    ├── test_combat.py
    ├── test_economy.py
    ├── test_npc.py
    ├── test_persistence.py
    └── fixtures/testworld/          # tiny synthetic world package for tests
```

**Dependency rule:** `engine/` may never import from `worlds/`. World content reaches the engine only through `registry.py` loading validated YAML.

---

## 4. World Package File Formats

All world files are YAML validated by Pydantic models at load time. Every entity has a globally unique `id` using the pattern `<type>.<name>` (e.g. `npc.argo`, `item.anneal_blade`, `loc.town_of_beginnings`). Cross-references always use these ids. Loading fails loudly on any dangling reference.

Common header fields on every entity file:

```yaml
id: npc.argo            # unique, stable, snake_case
canon_tier: confirmed    # confirmed | inference | generated  (§12)
tags: [informant, floor_1]
```

### 4.1 `world.yaml` — Package Manifest

```yaml
id: world.aincrad
name: "Aincrad"
version: "0.1.0"
engine_min_version: "0.1.0"
description: "The floating castle of Sword Art Online."
calendar:
  epoch_label: "November 6, 2022"     # in-world day 0
  hours_per_day: 24
  days_per_season: 90
  seasons: [autumn, winter, spring, summer]
entry_point:
  location: loc.town_of_beginnings
  time: { day: 0, hour: 13 }
starting_player:
  level: 1
  col: 500
  items: [item.small_sword, item.starter_clothes, item.bread_x2]
```

### 4.2 `rules.yaml` — World Rule Constants

Every tunable number lives here, never hardcoded in engine code.

```yaml
death:
  permadeath: true                # SAO rule: HP 0 = character deletion
  safe_zones_prevent_combat: true
leveling:
  xp_curve: "100 * level ** 1.8"  # evaluated by engine formula parser
  stat_points_per_level: 3
time_costs:                        # minutes, used by the COST step
  travel_per_km_road: 12
  travel_per_km_wilderness: 25
  shop_transaction: 5
  meal: 30
  smithing_base: 90
combat:
  base_reaction_ms: 800
  post_skill_delay_multiplier: 1.0  # sword-skill freeze duration scaling
economy:
  price_elasticity: 0.35
  merchant_restock_days: 3
  supply_decay_per_day: 0.02
ecology:
  respawn_check_hours: 6
  migration_pressure_threshold: 0.7
```

### 4.3 Floor File — `floors/floor_001.yaml`

```yaml
id: floor.1
canon_tier: confirmed
name: "Floor 1"
diameter_km: 10
biome: plains_and_forest
settlements: [loc.town_of_beginnings, loc.horunka, loc.tolbana]
zones:                              # contiguous regions with ecology data
  - id: zone.f1_west_fields
    terrain: grassland
    danger_rating: 1                # 1–10 absolute scale; NEVER scaled to player
    monster_populations:
      - species: mon.frenzy_boar
        carrying_capacity: 400
        current: 400
      - species: mon.dire_wolf
        carrying_capacity: 60
        current: 60
    resources:
      - node: res.medicinal_herb
        richness: 0.8               # depletes with harvesting, regrows seasonally
labyrinth:
  id: loc.f1_labyrinth
  levels: 20
  boss: mon.illfang_the_kobold_lord   # exists because the data says so
connections:
  up: floor.2                        # unlocked only when boss defeated (world event)
```

### 4.4 Location File — `locations/town_of_beginnings.yaml`

```yaml
id: loc.town_of_beginnings
canon_tier: confirmed
floor: floor.1
type: city                          # city | village | dungeon | landmark | field_area
safe_zone: true
population_npc: 2000                # ambient population (archetype-simulated)
districts:
  - id: dist.tob_market
    shops: [shop.tob_general, shop.tob_smith_a]
  - id: dist.tob_black_iron_palace
    features: [monument_of_life, teleport_gate]
services: [inn, teleport_gate, market, bank]
market: market.tob                  # link into economy/markets.yaml
schedule:
  gates_open: 06:00
  market_hours: [08:00, 20:00]
history:                             # world memory seeds
  - day: 0
    event: "Ten thousand players trapped at launch."
```

### 4.5 NPC Files

Two kinds: **named NPCs** (fully specified individuals) and **archetypes** (templates the engine instantiates to populate the world). Both share one schema.

`npcs/named/argo.yaml`:

```yaml
id: npc.argo
canon_tier: confirmed
name: "Argo the Rat"
archetype: arch.information_broker
location: loc.town_of_beginnings     # current position, updated at runtime
identity:
  age: 16
  appearance: "Whisker face-paint, hooded cloak."
personality:
  traits: [shrewd, curious, mercantile, loyal_for_a_price]
  values: [information, fairness_in_deals]
  fears: [being_seen_as_unreliable]
goals:                                # drives autonomous behavior (§7)
  - id: goal.argo_network
    description: "Build the most complete information network on the frontlines."
    priority: 0.9
    status: active
needs:                                # decay over time, drive schedule deviations
  food: 1.0
  rest: 1.0
  income: 0.6
occupation: information_broker
schedule:                             # default daily loop, interruptible by goals
  - { from: "07:00", to: "09:00", activity: gather_rumors, at: dist.tob_market }
  - { from: "09:00", to: "18:00", activity: sell_information, at: dist.tob_market }
  - { from: "18:00", to: "22:00", activity: socialize, at: loc.tob_inn }
relationships:
  - { with: npc.kirito, disposition: 0.4, type: business_contact }
knowledge:                            # what the NPC knows (narrator may only reveal via NPC)
  - fact: "Location of the secret smith on Floor 2."
    price_col: 2000
combat:
  level: 12
  skills: [skill.hiding, skill.acrobatics, skill.dagger]
```

Archetype files are identical but with distributions instead of fixed values:

```yaml
id: arch.town_merchant
population_weight: 0.05
personality:
  traits_pool: { greedy: 0.3, honest: 0.5, gossipy: 0.4 }
level_range: [3, 10]
```

### 4.6 Monster File — `monsters/frenzy_boar.yaml`

```yaml
id: mon.frenzy_boar
canon_tier: confirmed
name: "Frenzy Boar"
level: 2
stats: { hp: 120, attack: 14, defense: 8, speed: 6, reaction_ms: 1100 }
behavior:
  aggression: territorial            # passive | territorial | aggressive | predatory
  perception_range_m: 15
  flee_hp_ratio: 0.0                 # never flees
  pack_size: [1, 3]
  active_hours: [6, 20]              # diurnal
  ai_script: charge_and_gore         # named behavior in engine/systems/ecology.py
ecology:
  diet: grazer
  habitat: [grassland]
  reproduction_days: 30              # populations recover over time
drops:
  - { item: item.boar_meat, chance: 0.9, qty: [1, 2] }
  - { item: item.boar_tusk, chance: 0.35, qty: 1 }
xp: 24
```

### 4.7 Item File — `items/anneal_blade.yaml`

```yaml
id: item.anneal_blade
canon_tier: confirmed
name: "Anneal Blade"
category: weapon.one_handed_sword
rarity: uncommon
description: "A reliable one-handed sword awarded by the Secret Medicine quest."
stats: { attack: 38, accuracy: 5, durability_max: 1200 }
weight: 3.2
requirements: { skill: skill.one_handed_sword, proficiency: 50 }
upgrade:
  max_plus: 8
  paths: [sharpness, durability, quickness]
  materials_per_attempt: [{ item: item.iron_ingot, qty: 2 }]
  base_success: 0.85                 # decreases per attempt tier
crafting: null                       # quest reward only
market:
  base_value_col: 15000
  volatility: 0.4
drop_sources: [quest.secret_medicine_of_the_forest]
history_tracking: true               # engine records owners, kills, upgrades
```

### 4.8 Skill File — `skills/one_handed_sword.yaml`

```yaml
id: skill.one_handed_sword
canon_tier: confirmed
name: "One-Handed Sword"
category: weapon
requirements: null
proficiency_max: 1000
growth:                              # proficiency gain rules
  per_use: 0.4
  per_training_hour: 2.0
  diminishing_vs_lower_level: true
unlocks:                             # sword skills gated by proficiency
  - { at: 0,   grants: swordskill.horizontal }
  - { at: 50,  grants: swordskill.slant }
  - { at: 150, grants: swordskill.vertical_arc }
synergies: [skill.blade_throwing]
hidden: false
unique: false                        # unique skills have holder_limit: 1
```

Sword skills (techniques) are their own files:

```yaml
id: swordskill.horizontal
name: "Horizontal"
parent_skill: skill.one_handed_sword
hits: 1
damage_multiplier: 1.4
activation_ms: 150
execution_ms: 400
post_delay_ms: 800                   # the freeze — a real mechanical cost
range_m: 1.5
arc_deg: 120
cooldown_s: 4
```

### 4.9 Quest File — `quests/secret_medicine_of_the_forest.yaml`

Quests are **NPC goals exposed to the world**, not player objectives. They run whether or not the player participates.

```yaml
id: quest.secret_medicine_of_the_forest
canon_tier: confirmed
name: "Secret Medicine of the Forest"
source: npc.horunka_mother           # the NPC whose goal spawns this
purpose: "Her daughter is sick; she needs a Little Nepenthes ovule."
requirements:
  - obtain: { item: item.nepenthes_ovule, qty: 1 }
rewards:
  - { item: item.anneal_blade, qty: 1 }
duration_days: 14                     # after this, resolve failure consequences
failure:
  outcome: "The daughter's condition worsens; the household withdraws."
  world_effects:
    - { type: npc_state, target: npc.horunka_mother, set: { mood: grieving } }
npc_fallback:                          # what happens with NO player involvement
  chance_npc_resolves_per_day: 0.03    # another actor may complete it
repeatable: false
world_impact: minor
```

### 4.10 Faction File — `factions/aincrad_liberation_front.yaml`

```yaml
id: fac.alf
name: "Aincrad Liberation Front"
type: guild
headquarters: loc.town_of_beginnings
leadership: [npc.thinker]
membership_count: 300
treasury_col: 250000
agenda:
  - { goal: "Support low-level players", priority: 0.8 }
  - { goal: "Contest frontline authority", priority: 0.5 }
relations:
  - { with: fac.dka, disposition: -0.3 }
policies: { tax_rate: 0.1, recruitment: open }
```

---

## 5. Memory Systems

Three memory stores, all persisted in SQLite (§14). Memories are structured records, not prose. The narrator may summarize them but never invent them.

### 5.1 Player Memory

Tables: `player`, `player_inventory`, `player_skills`, `player_reputation`, `player_quest_log`, `player_history`.

```
player_history(id, day, hour, kind, summary, refs_json)
  kind ∈ { combat, quest, social, travel, craft, milestone }
```

Reputation is per-scope: `player_reputation(scope_id, value)` where scope is an NPC, faction, or settlement. Reputation changes only via committed events.

### 5.2 NPC Memory

Every *named* NPC (and any archetype NPC the player has meaningfully interacted with — they get promoted to named at first significant interaction) has:

```
npc_memory(npc_id, day, hour, kind, subject_id, valence, summary, decays)
  kind ∈ { conversation, promise, betrayal, assistance, combat, observation, rumor }
  valence ∈ [-1.0, 1.0]
```

Rules:
- **Promises** create a `promise` row with a due condition; the quest/NPC system checks these and generates consequences on breach.
- Memories decay in *salience* (not deleted) unless `kind ∈ {promise, betrayal}` or |valence| ≥ 0.8 — those are permanent.
- Rumors propagate: NPCs sharing a location and a `socialize` activity exchange high-salience memories with a transmission chance, mutating certainty (`rumor_certainty` field). This is how reputation spreads.

### 5.3 World Memory (The Chronicle)

Append-only event log of world-scale facts:

```
chronicle(id, day, hour, category, headline, detail, actors_json, location_id, visibility)
  category ∈ { boss_defeat, war, disaster, economy, politics, discovery, death }
  visibility ∈ { public, regional, secret }
```

The chronicle is the source of truth for "what has happened in this world." NPC knowledge, market shifts, and narrator context all derive from it. Floor unlocks, guild wars, and famines are chronicle entries with system side-effects.

---

## 6. Time Simulation

- World clock resolution: **1 minute**. Stored as `(day, minute_of_day)`.
- Every action's COST step returns a duration; ADVANCE moves the clock; TICK fires system updates.
- Tick granularity is hierarchical to stay cheap:
  - **Minute-level:** only the player's current zone (combat, nearby NPC actions).
  - **Hour-level:** NPC schedule transitions, shop open/close, weather, need decay — active floor.
  - **Day-level:** economy repricing, ecology (respawn/migration), quest expiry checks, faction agenda steps, rumor propagation — entire world.
- **Catch-up simulation:** when a region hasn't ticked in a long time (player returns after 20 days), run its day-level ticks in a batch before the player perceives it. Off-screen simulation uses the same rules, coarser resolution — never "generate a plausible outcome," always compute it.
- Consequences of time (all data-driven from `rules.yaml` and schedules): shops close, monsters' `active_hours` gate spawning/aggression, travel consumes time, crafting takes time, perishable items expire (`expires_days` on item), quests expire.

---

## 7. NPC Simulation (`engine/systems/npc.py`)

NPCs are utility-driven agents. Per hour-tick, each *active* named NPC runs:

```
1. Update needs (decay food/rest/income/safety).
2. Score candidate activities:
     score = goal_priority_weight + need_urgency + schedule_adherence + opportunity
3. Execute top activity (may be: follow schedule, pursue goal step, satisfy need,
   react to memory/event).
4. Emit any world effects (purchases → economy, travel → location change,
   conversations → memories/rumors).
```

- Goals decompose into **goal steps** (data or engine-generated): `acquire item`, `travel`, `persuade npc`, `earn col`, `investigate chronicle event`. Goal progress persists.
- Ambient population is simulated statistically (archetype pools per settlement — aggregate demand, crime rates, rumor volume) so cities feel alive without per-agent cost.
- Disposition toward the player = base personality response + Σ(memory valence × salience) + faction/reputation modifiers. Dialogue options and prices derive from disposition. The narrator receives disposition and relevant memories as *context*, never invents them.
- NPCs can die, permanently. Deaths enter the chronicle and NPC memories of witnesses.

---

## 8. Combat System (`engine/systems/combat.py`)

Combat is resolved on a **1-second round, continuous-position** model on a 2D plane per encounter.

Encounter state per combatant: `position (x,y)`, `facing`, `hp`, `stamina`, `active_skill`, `state ∈ {ready, winding_up, executing, post_delay, staggered, moving}`, `cooldowns{}`.

Per round, in initiative order (initiative = speed + reaction roll):
1. **Intent** — player intent parsed from input; NPC/monster intent from `ai_script`.
2. **Movement** — apply movement (speed m/s × 1s), terrain modifiers from zone data.
3. **Range/arc check** — attack connects only if target within `range_m` and `arc_deg`.
4. **Timing resolution** — defender in `ready` state with sufficient reaction (`reaction_ms` vs attacker's `activation_ms + execution_ms`) may parry/dodge/block (each a skill check with stamina cost). Defenders in `post_delay` or `staggered` cannot react — the sword-skill freeze is the core risk/reward mechanic.
5. **Damage** — `weapon.attack × skill.damage_multiplier × proficiency_factor − armor.defense`, ±10% seeded variance. Durability loss on both weapon and struck armor.
6. **State transitions** — apply `post_delay_ms`, cooldowns, stagger on heavy hits, morale checks (`flee_hp_ratio`).

- **No level scaling. Ever.** A level-2 boar hits a level-40 player for the same 14 base attack.
- Party coordination: switch mechanic — a defender's successful parry grants allies a 1-round `opening` window (+accuracy, target cannot react).
- Death at HP 0: if `rules.death.permadeath`, the character is deleted from the save (the save records the death in the chronicle; the game is over). This must actually work.
- Combat rewards preparation and knowledge: monster `ai_script`s are deterministic patterns that can be learned.

---

## 9. Skill System Runtime

- Player/NPC skill state: `(skill_id, proficiency, equipped_slot)`. Skill slots limited by rule (`rules.yaml: skills.max_equipped`).
- Proficiency grows per validated use (`growth.per_use`), reduced against much-lower-level targets, plus training activities (time cost → growth).
- Proficiency thresholds auto-grant sword skills / recipes per the skill file's `unlocks`.
- **Hidden skills:** flagged `hidden: true`; granted only when their `unlock_condition` (data-defined predicate: e.g., "parry 100 attacks with a dagger") is met. Never revealed by the narrator beforehand.
- **Unique skills:** `unique: true, holder_limit: 1`; the engine enforces the limit world-wide; acquisition conditions are canon data, not narrative gifts.

---

## 10. Item System Runtime

- Every item **instance** is a row (`item_instance(id, def_id, owner_id, durability, plus_level, custom_history_json)`) — instances of the same definition diverge (durability, upgrades, provenance).
- Durability decreases on use; at 0 the instance shatters (chronicle-worthy if notable). Repair = smith NPC or player skill + materials + time.
- Upgrading uses the item file's `upgrade` block; failure consumes materials and may decrement plus level (per world rules).
- `history_tracking: true` items append events (owners, boss kills) — provenance affects `market_value` and NPC dialogue.
- Weight limits from strength stat; encumbrance affects combat speed and travel time.

---

## 11. Economy Simulation (`engine/systems/economy.py`)

Per-market (settlement-level) model, day-tick:

```
price(item, market) = base_value
                    × (demand_index / supply_index) ^ price_elasticity
                    × scarcity_events_modifier
                    × faction_tax
```

- **Supply index** per item per market: increases when players/NPCs sell or NPC producers craft; decays daily; shocked by ecology (over-farmed boars → leather supply falls at the source but crashes prices where dumped).
- **Demand index:** driven by NPC consumption (ambient population needs), crafting recipes' input demand, quest-driven spikes, and chronicle events (a floor unlock spikes demand for potions and crystals).
- Merchants are NPCs with `occupation: merchant`: they hold inventory, restock via routes (travel time matters), adjust their personal margin by personality (`greedy` trait), and can run out.
- Player actions are just supply/demand inputs — dumping 200 boar tusks visibly crashes the tusk price in that market and arbitrage opportunities emerge across floors.
- Rare item entering circulation (first Anneal Blade sold) creates a chronicle `economy` entry and reprices the category.

---

## 12. Canon Management (`engine/canon/guard.py`)

Every entity carries `canon_tier`:

| Tier | Meaning | Mutability |
|---|---|---|
| `confirmed` | Directly established by official SAO material | Immutable facts; state may evolve (Illfang can be killed) but definitions can't be contradicted |
| `inference` | Reasoned from established rules; documented in `canon/inferences.yaml` with its justification | Revisable if it conflicts with confirmed canon |
| `generated` | Created by the simulation to fill gaps | Free, but must pass consistency checks |

Rules:
- Generation (new NPCs, quests, locations) must run through `guard.check(entity)`: validates no id collisions with confirmed canon, no rule violations (`rules.yaml`), and tier tagging.
- When content is missing, generate from **world rules + adjacent data** (e.g., a Floor 3 village gets economy/NPCs from archetypes and floor biome), never from "what would be cool."
- The narrator is instructed (system prompt) that confirmed-canon facts provided in context are inviolable and gaps must be handed back to the engine's generator, not improvised.

---

## 13. Quest System Runtime

Quest lifecycle: `dormant → available → active → {completed | failed | expired | npc_resolved}`.

- Quests instantiate from authored templates **or** emerge from NPC goals crossing an `externalize` threshold (an NPC who can't achieve a goal alone posts it — the engine composes a quest record from the goal, with generated tier).
- `duration_days` counts from availability; expiry fires `failure.world_effects` regardless of player awareness.
- `npc_fallback` rolls daily — the world does not wait for the player.
- Consequences are first-class: every terminal state applies `world_effects` (NPC state, chronicle entries, economy shocks, faction disposition).

---

## 14. Persistence Layer (`engine/persistence/`)

SQLite, one DB per save. Core tables (see `schema.sql`):

```
world_clock(day, minute)
entities(id, kind, def_id, state_json, location_id, updated_day)   -- runtime state of every live entity
item_instances(id, def_id, owner_id, durability, plus, history_json)
npc_memory(...)            -- §5.2
player_* tables            -- §5.1
chronicle(...)             -- §5.3
markets(market_id, item_def, supply_idx, demand_idx, price)
quests(instance_id, def_id, state, available_day, expires_day)
goals(npc_id, goal_id, progress_json, status)
promises(npc_id, subject_id, due_condition_json, status)
rng_streams(name, state)   -- reproducibility
zone_ticks(zone_id, last_ticked_day)  -- catch-up bookkeeping
```

- All step-7 COMMITs are single transactions; a crash mid-turn loses at most the un-narrated turn.
- Definitions (YAML) are *not* copied into the DB — the DB stores state referencing def ids; `meta.yaml` pins world package version.

---

## 15. Perception & the Narrator

`perception.py` computes what the player can perceive from committed deltas:
- Line of sight / same location; hearing range; UI-style knowledge (own HP, cursor colors, skill list); social knowledge (what an NPC chose to say).
- Hidden information (NPC internal goals, monster stats beyond observation, other regions' events) is **excluded** from narrator context unless the player has learned it in-world.

The narrator (`llm_narrator.py`) receives: `{perceived_deltas, location_snapshot, present_npcs(disposition + relevant memories), recent player history, style guide}` and returns prose. It has **no tool access and no ability to mutate state**.

### AI Game Master Rules (encoded in the narrator system prompt + engine constraints)

Must: simulate before describing (enforced structurally), maintain continuity (context from memory systems), respect rules, track consequences (engine-side), avoid forced outcomes and favoritism, preserve uncertainty (narrate only perceived facts), maintain immersion (no meta-talk).

Must not: reveal hidden info, control player decisions (always end awaiting input; never narrate the player's choices, only their stated action's results), rewrite committed events (the DB is append-truth), ignore mechanics for drama, grant rewards (has no mechanism to).

---

## 16. CLI

```
cardinal new <save> --world aincrad [--seed N]
cardinal play <save> [--narrator llm|plain]
cardinal tick <save> --days N            # advance world with no player (testing persistence)
cardinal inspect <save> <query>          # debug: entity state, market, chronicle
cardinal validate worlds/aincrad         # schema + reference + canon checks
```

`cardinal tick` is the acceptance test for "the world exists independently of the player."

---

## 17. Testing Requirements

- Every system has headless unit tests using `tests/fixtures/testworld` (a 2-zone, 5-NPC synthetic package).
- Golden tests: same seed + same action script ⇒ identical end state.
- Invariant tests: no level scaling (assert monster stats independent of player level), canon guard rejects contradictions, quest expiry fires without player presence, economy conservation (col is neither created nor destroyed outside defined sources/sinks).
- `cardinal tick --days 365` on the fixture world must complete without error and produce a non-trivial chronicle.

---

## 18. Implementation Milestones

1. **M1 — Skeleton:** repo layout, Pydantic schemas for all §4 formats, registry + `cardinal validate`, SQLite store, clock, minimal loop with plain narrator.
2. **M2 — Living world:** NPC agents (schedules, needs, goals), time hierarchy + catch-up, chronicle, `cardinal tick`.
3. **M3 — Player systems:** action parser, combat, skills, items, perception, permadeath.
4. **M4 — Economy & quests:** markets, merchants, quest lifecycle, rumor propagation.
5. **M5 — Content & narrator:** Aincrad Floor 1 package (Town of Beginnings, 2 villages, ~15 named NPCs, labyrinth, Illfang, Secret Medicine quest), LLM narrator with GM rules.
6. **M6 — Hardening:** golden/invariant tests, save migration, balance pass via long-run `tick` telemetry.

**Definition of done for v0.1:** a player can start in the Town of Beginnings, live for an in-game month on Floor 1, and `cardinal inspect` shows a chronicle, market history, and NPC memories that would be materially different in a world the player never entered.
