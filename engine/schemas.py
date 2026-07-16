"""Pydantic v2 models for every world-package file format (§4).

Design notes:
- Models validate structure and types; cross-reference integrity (dangling
  ids) is checked separately by the registry, which knows every loaded id.
- `extra="allow"` on content models: world packages may carry fields the
  engine doesn't consume yet (lore hooks, authoring notes). Unknown fields
  are preserved, never silently required.
- Every entity id uses the pattern `<type>.<name>` (snake_case).
"""

from __future__ import annotations

import re
from typing import Any, Literal, Optional

from pydantic import BaseModel, ConfigDict, Field, field_validator

ID_PATTERN = re.compile(r"^[a-z][a-z0-9_]*\.[a-z0-9_.]+$")

CanonTier = Literal["confirmed", "inference", "generated"]


class ContentModel(BaseModel):
    model_config = ConfigDict(extra="allow")


class Entity(ContentModel):
    """Common header fields shared by every entity file."""

    id: str
    canon_tier: CanonTier = "generated"
    tags: list[str] = Field(default_factory=list)

    @field_validator("id")
    @classmethod
    def _valid_id(cls, v: str) -> str:
        if not ID_PATTERN.match(v):
            raise ValueError(f"id {v!r} must match '<type>.<name>' snake_case pattern")
        return v


# --- §4.1 world.yaml ------------------------------------------------------


class Calendar(ContentModel):
    epoch_label: str = "Day 0"
    hours_per_day: int = 24
    days_per_season: int = 90
    seasons: list[str] = Field(default_factory=lambda: ["spring", "summer", "autumn", "winter"])


class WorldTime(ContentModel):
    day: int = 0
    hour: int = 0


class EntryPoint(ContentModel):
    location: str
    time: WorldTime = Field(default_factory=WorldTime)


class Currency(ContentModel):
    """Worlds name their own money (col, eddies, berries, glimmer). The DB
    column stays `col` as a generic store; this drives narration and display."""

    id: str = "col"
    name: str = "Col"


class StartingPlayer(ContentModel):
    level: int = 1
    col: int = 0
    items: list[str] = Field(default_factory=list)
    skills: list[str] = Field(default_factory=list)
    vehicles: list[str] = Field(default_factory=list)


class WorldManifest(Entity):
    name: str
    version: str = "0.1.0"
    engine_min_version: str = "0.1.0"
    description: str = ""
    calendar: Calendar = Field(default_factory=Calendar)
    currency: Currency = Field(default_factory=Currency)
    region_label: str = "Floor"  # what this world calls its top-level regions
    entry_point: EntryPoint
    starting_player: StartingPlayer = Field(default_factory=StartingPlayer)


# --- §4.2 rules.yaml --------------------------------------------------------


class Rules(ContentModel):
    """World-rule constants. Deliberately open-shaped: systems read the keys
    they own via `get()` paths; every tunable lives here, never in engine code."""

    death: dict[str, Any] = Field(default_factory=dict)
    leveling: dict[str, Any] = Field(default_factory=dict)
    time_costs: dict[str, Any] = Field(default_factory=dict)
    combat: dict[str, Any] = Field(default_factory=dict)
    economy: dict[str, Any] = Field(default_factory=dict)
    ecology: dict[str, Any] = Field(default_factory=dict)
    skills: dict[str, Any] = Field(default_factory=dict)


# --- §4.3 floors ------------------------------------------------------------


class MonsterPopulation(ContentModel):
    species: str
    carrying_capacity: int
    current: int


class ResourceNode(ContentModel):
    node: str
    richness: float = 1.0


class Zone(ContentModel):
    id: str
    terrain: str = "plains"
    danger_rating: int = 1  # 1-10 absolute scale; NEVER scaled to player
    cover_density: float = 0.0   # 0 open field .. 1 dense cover; feeds ranged combat LOS
    visibility_m: Optional[float] = None  # sightline cap (fog, tunnels, urban canyon)
    monster_populations: list[MonsterPopulation] = Field(default_factory=list)
    resources: list[ResourceNode] = Field(default_factory=list)


class Labyrinth(ContentModel):
    id: str
    levels: int = 1
    boss: Optional[str] = None


class Floor(Entity):
    name: str
    diameter_km: float = 10
    biome: str = "plains"
    settlements: list[str] = Field(default_factory=list)
    zones: list[Zone] = Field(default_factory=list)
    labyrinth: Optional[Labyrinth] = None
    connections: dict[str, str] = Field(default_factory=dict)


# --- §4.4 locations ----------------------------------------------------------


class District(ContentModel):
    id: str
    shops: list[Any] = Field(default_factory=list)  # inline {id: shop.x, ...} defs or refs
    features: list[str] = Field(default_factory=list)


class HistorySeed(ContentModel):
    day: int
    event: str


class Location(Entity):
    floor: str
    type: Literal["city", "village", "dungeon", "landmark", "field_area"] = "landmark"
    safe_zone: bool = False
    population_npc: int = 0
    districts: list[District] = Field(default_factory=list)
    services: list[str] = Field(default_factory=list)
    market: Optional[str] = None
    schedule: dict[str, Any] = Field(default_factory=dict)
    history: list[HistorySeed] = Field(default_factory=list)


# --- §4.5 NPCs ----------------------------------------------------------------


class Goal(ContentModel):
    id: str
    description: str
    priority: float = 0.5
    status: Literal["active", "paused", "completed", "abandoned"] = "active"


class ScheduleBlock(ContentModel):
    from_: str = Field(alias="from")
    to: str
    activity: str
    at: str

    model_config = ConfigDict(extra="allow", populate_by_name=True)


class Relationship(ContentModel):
    with_: str = Field(alias="with")
    disposition: float = 0.0
    type: str = "acquaintance"

    model_config = ConfigDict(extra="allow", populate_by_name=True)


class Knowledge(ContentModel):
    fact: str
    price_col: Optional[int] = None


class NpcCombat(ContentModel):
    level: int = 1
    skills: list[str] = Field(default_factory=list)


class Npc(Entity):
    name: str
    archetype: Optional[str] = None
    location: str
    # actor taxonomy (spec §21): ambient = today's scheduled NPCs;
    # agent = player-type actors run by engine/systems/agents.py;
    # setpiece = boss-script owned. Promotion is one-way (ambient->agent).
    actor_class: Literal["ambient", "agent", "setpiece"] = "ambient"
    policy: Optional[str] = None          # agents: engine policy archetype
    capabilities: list[str] = Field(default_factory=list)
    faction: Optional[str] = None         # membership; feeds dues + standing
    col: int = 0                          # agents: real starting balance
    loadout: list[str] = Field(default_factory=list)
    policy_params: dict[str, Any] = Field(default_factory=dict)
    identity: dict[str, Any] = Field(default_factory=dict)
    personality: dict[str, Any] = Field(default_factory=dict)
    goals: list[Goal] = Field(default_factory=list)
    needs: dict[str, float] = Field(default_factory=dict)
    occupation: Optional[str] = None
    schedule: list[ScheduleBlock] = Field(default_factory=list)
    relationships: list[Relationship] = Field(default_factory=list)
    knowledge: list[Knowledge] = Field(default_factory=list)
    combat: NpcCombat = Field(default_factory=NpcCombat)


class Archetype(Entity):
    population_weight: float = 0.0
    personality: dict[str, Any] = Field(default_factory=dict)
    level_range: list[int] = Field(default_factory=lambda: [1, 1])
    occupation: Optional[str] = None


# --- §4.6 monsters -------------------------------------------------------------


class MonsterStats(ContentModel):
    hp: int
    attack: int
    defense: int = 0
    speed: int = 1
    reaction_ms: int = 1000  # this monster's OWN defensive reaction (dodging the player)
    # How telegraphed this monster's own attacks are, in ms — higher means
    # more time for the player to react. Separate stat from reaction_ms
    # above (that one is the monster's *defense*; this is its *offense*).
    # Previously the engine derived a monster's attack windup from its own
    # reaction_ms, which silently made both concepts the same number and
    # produced monsters that could never be defended against regardless of
    # player build (Test 4 playtest finding). Falls back to a speed-derived
    # default for monsters that don't author one.
    attack_windup_ms: Optional[int] = None


class MonsterBehavior(ContentModel):
    aggression: Literal["passive", "territorial", "aggressive", "predatory"] = "passive"
    perception_range_m: float = 10
    flee_hp_ratio: float = 0.0
    pack_size: list[int] = Field(default_factory=lambda: [1, 1])
    active_hours: list[int] = Field(default_factory=lambda: [0, 24])
    ai_script: str = "default"


class Drop(ContentModel):
    item: str
    chance: float = 1.0
    qty: Any = 1  # int or [min, max]


class Monster(Entity):
    name: str
    level: int
    stats: MonsterStats
    behavior: MonsterBehavior = Field(default_factory=MonsterBehavior)
    ecology: dict[str, Any] = Field(default_factory=dict)
    drops: list[Drop] = Field(default_factory=list)
    xp: int = 0


# --- §4.7 items ------------------------------------------------------------------


class UpgradeSpec(ContentModel):
    max_plus: int = 0
    paths: list[str] = Field(default_factory=list)
    materials_per_attempt: list[dict[str, Any]] = Field(default_factory=list)
    base_success: float = 1.0


class MarketSpec(ContentModel):
    base_value_col: int = 0
    volatility: float = 0.0


class RangedSpec(ContentModel):
    """Makes any weapon a ranged weapon: bows, slings, firearms, launchers.
    A thrown weapon (pick, knife, grenade) sets `thrown: true` and needs no ammo —
    the weapon instance itself is the projectile and must be recovered or lost."""

    thrown: bool = False
    ammo: Optional[str] = None           # item id consumed per shot (arrows, bullets, cells)
    magazine: int = 1                    # shots before a reload
    reload_s: float = 0.0
    optimal_range_m: float = 10.0        # full damage/accuracy inside this
    max_range_m: float = 20.0            # cannot connect beyond this
    projectile_speed_mps: float = 30.0   # vs defender reaction: fast rounds can't be dodged
    spread_deg: float = 0.0              # accuracy cone (shotguns, slings, hip-fire)


class Item(Entity):
    name: str
    category: str = "misc"
    rarity: str = "common"
    description: str = ""
    stats: dict[str, Any] = Field(default_factory=dict)
    weight: float = 0.0
    requirements: Optional[dict[str, Any]] = None
    ranged: Optional[RangedSpec] = None
    upgrade: Optional[UpgradeSpec] = None
    crafting: Optional[dict[str, Any]] = None
    market: MarketSpec = Field(default_factory=MarketSpec)
    drop_sources: list[str] = Field(default_factory=list)
    history_tracking: bool = False
    expires_days: Optional[int] = None


# --- §4.8 skills -------------------------------------------------------------------


class SkillGrowth(ContentModel):
    per_use: float = 0.1
    per_training_hour: float = 1.0
    diminishing_vs_lower_level: bool = True


class SkillUnlock(ContentModel):
    at: int
    grants: str


class Skill(Entity):
    name: str
    category: str = "general"
    requirements: Optional[dict[str, Any]] = None
    proficiency_max: int = 1000
    growth: SkillGrowth = Field(default_factory=SkillGrowth)
    unlocks: list[SkillUnlock] = Field(default_factory=list)
    synergies: list[str] = Field(default_factory=list)
    hidden: bool = False
    unique: bool = False
    holder_limit: Optional[int] = None
    unlock_condition: Optional[dict[str, Any]] = None


class Technique(Entity):
    """A combat technique belonging to a parent skill — genre-agnostic.

    `delivery` selects the resolution path (§8):
      melee      — connects within range_m/arc_deg; defender may parry/dodge/block
      thrown     — the wielded item is the projectile (picks, knives, grenades)
      projectile — fires the weapon's ammo (bows, slings, firearms, launchers)
      beam       — hitscan (lasers, some magic); dodged on tell, never in flight
      area       — affects everything within range_m of the impact point

    SAO sword skills are simply melee techniques whose post_delay_ms is large.
    """

    name: str
    parent_skill: str
    delivery: Literal["melee", "thrown", "projectile", "beam", "area"] = "melee"
    hits: int = 1
    damage_multiplier: float = 1.0
    base_damage: Optional[float] = None  # ability's own power (spells, grenades);
                                         # set -> no weapon or ammo is involved
    resource: Optional[str] = None       # pool consumed (mana, light, energy…)
    resource_cost: float = 0.0           # pools defined in rules combat.pools
    activation_ms: int = 100
    execution_ms: int = 400
    post_delay_ms: int = 500  # the freeze — a real mechanical cost
    range_m: float = 1.5      # melee reach, or blast radius for `area`
    arc_deg: float = 90
    max_range_m: Optional[float] = None  # ranged deliveries; None = weapon's ranged spec
    ammo_per_use: int = 1                # weapon-projectile delivery only
    cooldown_s: float = 1


# Backwards-compatible alias: `swordskill.*` files are melee techniques.
SwordSkill = Technique


# --- §4.9 quests --------------------------------------------------------------------


class QuestOutcome(ContentModel):
    outcome: str = ""
    world_effects: list[dict[str, Any]] = Field(default_factory=list)


QuestFailure = QuestOutcome  # legacy name


class Quest(Entity):
    name: str
    source: str  # the NPC whose goal spawns this
    purpose: str = ""
    requirements: list[dict[str, Any]] = Field(default_factory=list)
    rewards: list[dict[str, Any]] = Field(default_factory=list)
    duration_days: Optional[int] = None
    success: QuestOutcome = Field(default_factory=QuestOutcome)
    failure: QuestOutcome = Field(default_factory=QuestOutcome)
    npc_fallback: dict[str, Any] = Field(default_factory=dict)
    repeatable: bool = False
    world_impact: str = "minor"


# --- §4.10 factions --------------------------------------------------------------------


class FactionRelation(ContentModel):
    with_: str = Field(alias="with")
    disposition: float = 0.0

    model_config = ConfigDict(extra="allow", populate_by_name=True)


class Faction(Entity):
    name: str
    type: str = "guild"
    headquarters: Optional[str] = None
    leadership: list[str] = Field(default_factory=list)
    membership_count: int = 0
    treasury_col: int = 0
    agenda: list[dict[str, Any]] = Field(default_factory=list)
    relations: list[FactionRelation] = Field(default_factory=list)
    policies: dict[str, Any] = Field(default_factory=dict)


# --- modifiers (curses, blessings, tattoos, chrome, mutations…) -----------------


class ModifierEffect(ContentModel):
    """One mechanical consequence of a modifier. Systems dispatch on `type`:

    stat_add / stat_mult — target: stat name (attack, hp_max, speed…)
    skill_grant / technique_grant — target: skill/tech id
    need_rate — target: need name; value multiplies its decay rate
    disposition — target: npc/faction id; value shifts how they react on sight
    regen / damage_over_time — value: HP per hour
    perception — value: bonus/penalty to being noticed (a glowing brand hides poorly)

    Unknown types are preserved for world-specific systems.
    """

    type: str
    target: Optional[str] = None
    value: Any = None


class ModifierAcquisition(ContentModel):
    source: str = "infliction"      # surgery | ritual | tattoo_parlor | infliction | item | birth
    installer_skill: Optional[str] = None
    time_minutes: int = 0
    cost_col: int = 0
    materials: list[dict[str, Any]] = Field(default_factory=list)


class Modifier(Entity):
    """A persistent alteration to a player or NPC: curse, blessing, cyberware,
    tattoo, mutation, disease, brand, geas. One schema — the genre lives in
    `mod_type` and which fields are filled."""

    name: str
    description: str = ""
    mod_type: str = "trait"          # curse | blessing | cyberware | tattoo | mutation | disease | …
    slot: Optional[str] = None       # body location; one active mod per slot unless stackable
    stackable: bool = False
    visibility: Literal["visible", "concealable", "hidden"] = "visible"
    duration_days: Optional[float] = None    # None = permanent
    duration_minutes: Optional[int] = None   # short afflictions (venom, stuns)
    effects: list[ModifierEffect] = Field(default_factory=list)
    side_effects: list[ModifierEffect] = Field(default_factory=list)
    acquisition: ModifierAcquisition = Field(default_factory=ModifierAcquisition)
    removal: Optional[dict[str, Any]] = None  # None = irremovable (a true curse)
    humanity_cost: float = 0.0       # generic essence/corruption knob; rules.yaml decides meaning


# --- devices & interactions (doors, terminals, levers, traps…) --------------------


class InteractionOutcome(ContentModel):
    message: str = ""
    set_state: Optional[str] = None
    effects: list[dict[str, Any]] = Field(default_factory=list)
    # effect types the engine resolves: teleport {to}, give_item {item, qty},
    # npc_state {target, set}, chronicle {headline, category, …}


class Interaction(ContentModel):
    """One verb a device answers to. With a skill, success is a proficiency-vs-
    difficulty check; without one, it always succeeds (a plain lever)."""

    verb: str                        # hack | pick | open | press | use | …
    skill: Optional[str] = None
    difficulty: float = 0.0
    time_minutes: int = 1
    requires_item: Optional[str] = None
    requires_state: Optional[str] = None  # only usable while device is in this state
    success: InteractionOutcome = Field(default_factory=InteractionOutcome)
    failure: InteractionOutcome = Field(default_factory=InteractionOutcome)


class Device(Entity):
    """An interactable world object: door, terminal, camera, shrine, trap.
    Runtime state (current state string) lives in the entities table."""

    name: str
    description: str = ""
    location: str
    initial_state: str = "idle"
    interactions: list[Interaction] = Field(default_factory=list)


# --- vehicles (mounts, carts, cars, boats, ships…) --------------------------------


class VehicleFuel(ContentModel):
    item: str                        # consumed item id (petrol, hay handled via `living`)
    per_km: float = 0.1
    tank_capacity: float = 50.0


class Vehicle(Entity):
    """Anything that carries actors: a mule, a wagon, a motorcycle, a sloop,
    a dropship. `living: true` vehicles are creatures — they eat, rest, and can
    die; machines burn fuel and take durability damage instead."""

    name: str
    description: str = ""
    category: str = "ground"         # ground | mount | water | air | space | rail | …
    living: bool = False
    stats: dict[str, Any] = Field(default_factory=dict)   # hp, armor, handling
    speed_kmh: dict[str, float] = Field(default_factory=dict)  # terrain -> speed; absent = impassable
    seats: int = 1
    cargo_kg: float = 0.0
    fuel: Optional[VehicleFuel] = None
    needs: dict[str, float] = Field(default_factory=dict)  # living vehicles: food/rest like NPCs
    requirements: Optional[dict[str, Any]] = None           # e.g. { skill: skill.riding }
    upgrade: Optional[UpgradeSpec] = None
    crafting: Optional[dict[str, Any]] = None
    market: MarketSpec = Field(default_factory=MarketSpec)


# --- economy ------------------------------------------------------------------------------


class Market(Entity):
    settlement: str
    stock_categories: list[str] = Field(default_factory=list)


class Resource(Entity):
    """A harvestable resource node type (engine extension; see README deviations)."""

    name: str
    regrowth_days: int = 30
    yields: list[Drop] = Field(default_factory=list)


# Category name -> model, used by the registry when loading package dirs.
CATEGORY_MODELS: dict[str, type[Entity]] = {
    "floors": Floor,
    "locations": Location,
    "npcs": Npc,
    "archetypes": Archetype,
    "monsters": Monster,
    "items": Item,
    "skills": Skill,
    "techniques": Technique,
    "swordskills": Technique,  # legacy alias
    "quests": Quest,
    "factions": Faction,
    "markets": Market,
    "resources": Resource,
    "modifiers": Modifier,
    "vehicles": Vehicle,
    "devices": Device,
}
