//! The in-memory world-package model (Vol. IV Ch. 1-3).
//!
//! A package is data: a manifest, the domains it selects, the rules each consumes, and the
//! initial content its world begins with (Vol. IV Ch. 1 §1.2). This model is the parsed,
//! validated shape the loader turns into a running world.

use crate::version::{EngineReq, Version};

/// A complete world-package definition (Vol. IV Ch. 1 §1.2).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct WorldPackage {
    /// Identity, version, and engine requirement.
    pub manifest: Manifest,
    /// The world's clock: tick length and day length (Vol. II Ch. 2, Amendment A-1).
    pub clock: ClockRules,
    /// Tunable rules for the physical domain.
    pub physical_rules: PhysicalRules,
    /// Tunable rules for the living domain, present only if the domain is selected.
    pub living_rules: Option<LivingRules>,
    /// Tunable rules for the information layer, present only if it is selected (Amendment
    /// A-8).
    pub information_rules: Option<InformationRules>,
    /// Tunable rules for decision systems, present only if minds are selected (Amendment A-9).
    pub minds_rules: Option<MindsRules>,
    /// Tunable rules for Society's bonds, present only if Society is selected (Amendment A-11).
    pub society_rules: Option<SocietyRules>,
    /// Tunable rules for deposits, present only if Resources is selected (Amendment A-15).
    pub resources_rules: Option<ResourcesRules>,
    /// The regions the world begins with.
    pub regions: Vec<RegionSpec>,
    /// The organisms the world begins with (living domain), each placed in a region.
    pub organisms: Vec<OrganismSpec>,
    /// Extra containment links seeded at generation (e.g. region within a continent),
    /// beyond the organism-in-region links implied by [`WorldPackage::organisms`].
    pub containment: Vec<ContainmentSpec>,
    /// Undirected adjacency edges between regions -- the world's topology (Vol. III Ch. 1
    /// §1.5), seeded as a cardinality-many physical fact in both directions.
    pub adjacency: Vec<AdjacencySpec>,
    /// Per-region exposure to the open sky (Vol. III Ch. 1 §1.6). A region absent here is
    /// fully exposed.
    pub exposure: Vec<ExposureSpec>,
    /// Local positions of entities within their immediate containers (Vol. III Ch. 1 §1.3).
    pub positions: Vec<PositionSpec>,
    /// Portals -- located connections from a spot in one region to another (Vol. III Ch. 1
    /// §1.5).
    pub portals: Vec<PortalSpec>,
    /// World-pinned danger for specific portals (Vol. III Ch. 1 §1.11). A portal absent here
    /// has its danger derived from height (and, later, weather).
    pub portal_danger: Vec<PortalDangerSpec>,
    /// The materials the world defines, each an entity exposing property facts (Vol. III Ch. 1
    /// §1.9). Referenced by [`WorldPackage::made_of`].
    pub materials: Vec<MaterialSpec>,
    /// Which materials each physical object is composed of (Vol. III Ch. 1 §1.9), seeded as a
    /// cardinality-many `made_of` fact — one entry per object/material link.
    pub made_of: Vec<MadeOfSpec>,
    /// Overlapping region memberships beyond the containment hierarchy (Vol. III Ch. 1 §1.7),
    /// seeded as a cardinality-many `in_region` fact — one entry per location/region link.
    pub in_region: Vec<RegionMembershipSpec>,
    /// Body sizes (Amendment A-3). An entity absent here is a point.
    pub bodies: Vec<BodySpec>,
    /// Facings (Amendment A-3). An entity absent here faces its frame's north.
    pub facing: Vec<FacingSpec>,
    /// Motion already under way when the world begins (Amendment A-3).
    pub motion: Vec<MotionSpec>,
    /// Per-entity constraint flags (Amendment A-4).
    pub flags: Vec<FlagSpec>,
    /// Linked faces of openings (Amendment A-4), each seeded in both directions.
    pub portal_pairs: Vec<(u64, u64)>,
    /// Terrain heightfields (Amendment A-4).
    pub terrain: Vec<TerrainSpec>,
    /// Travel intents under way when the world begins (Ruling 13).
    pub travel: Vec<TravelSpec>,
    /// Places that exist to hold other places — a continent, a city's hinterland, a town — each
    /// with the place it lies in (`None` for the outermost). A place declared here has no climate
    /// of its own; it, and everything in it without one, inherits the nearest enclosing climate
    /// (Amendment A-5).
    pub places: Vec<(u64, Option<u64>)>,
    /// How far each organism can see, in centimetres (Living Systems' sensory capability,
    /// Amendment A-8): `(organism, sight range)`. An organism absent here perceives nothing.
    pub senses: Vec<(u64, i64)>,
    /// What each mind knows at the start (Amendment A-8; Vol. IV Ch. 5): `(mind, things)`.
    /// For each thing, where it is; for an opening, also where it leads and whether it is open;
    /// for a place, also how warm its air is.
    pub knows: Vec<(u64, Vec<u64>)>,
    /// Which entities have minds, and how fast each walks when it chooses to go somewhere
    /// (cm/s): `(mind, walk speed)` (Amendment A-9).
    pub minds: Vec<(u64, i64)>,
    /// Each mind's temperament: `(mind, [three traits])`, each in hundredths of a percent
    /// (Amendment A-11).
    pub temperament: Vec<(u64, [i64; 3])>,
    /// The kinds of need that can arise (Amendment A-11), each an entity with its rules.
    pub need_kinds: Vec<NeedKindSpec>,
    /// Deposits that regrow and yield items when picked (Amendment A-15).
    pub deposits: Vec<DepositSpec>,
    /// Recipes: what makers make, from what, where, and how long it takes (Amendment A-17).
    pub recipes: Vec<RecipeSpec>,
    /// Job kinds: the work the world declares (Amendment A-18).
    pub jobs: Vec<JobSpec>,
    /// Roles: who holds which job, `(person, job)` (Amendment A-18).
    pub roles: Vec<(u64, u64)>,
    /// Who owns what at the start: `(thing, owner)` (Amendment A-19).
    pub owners: Vec<(u64, u64)>,
    /// How curious each mind is: `(mind, worth of seeing a place it has never stood in)`
    /// (Amendment A-19). Absent: incurious.
    pub curiosity: Vec<(u64, i64)>,
    /// What each mind likes: `(mind, material, worth)` (Amendment A-19).
    pub likes: Vec<(u64, u64, i64)>,
    /// Where each mind keeps what it owns: `(mind, place)` (Amendment A-19).
    pub home: Vec<(u64, u64)>,
    /// How hungry each organism is at the start (Amendment A-16): `(organism, hunger)`. Absent:
    /// fed.
    pub hunger: Vec<(u64, i64)>,
    /// The dependences organisms start with (Amendment A-13): `(organism, material, level)`.
    pub dependence: Vec<(u64, u64, i64)>,
    /// How tired each organism is at the start, in hundredths of a percent (Amendment A-10):
    /// `(organism, fatigue)`. An organism absent here starts rested.
    pub fatigue: Vec<(u64, i64)>,
    /// Daily routines: `(mind, from hour, to hour, where)` — where the world says a mind belongs
    /// between those hours (a window that wraps past midnight when `from > to`).
    pub routines: Vec<(u64, i64, i64, u64)>,
}

/// A constraint flag a world may set on an entity (Amendment A-4).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Flag {
    /// Nothing passes through it.
    Solid,
    /// Blocks sight (on a portal: when closed).
    Opaque,
    /// A walled region, crossed only through its portals.
    Enclosed,
    /// A free body: falls when unsupported, can travel.
    Mobile,
    /// A portal that starts closed.
    Closed,
}

/// The flags one entity carries.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct FlagSpec {
    /// The entity's raw id.
    pub entity_id: u64,
    /// Its flags, as declared.
    pub flags: Vec<Flag>,
}

/// A region's terrain (Amendment A-4): `heights` row by row, `columns` to a row, samples
/// `spacing` centimetres apart in the region's frame, sample `[0, 0]` at its origin.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TerrainSpec {
    /// The region's raw id.
    pub region_id: u64,
    /// Distance between neighbouring samples, in centimetres.
    pub spacing: i64,
    /// Samples per row.
    pub columns: usize,
    /// Heights in centimetres, row-major.
    pub heights: Vec<i64>,
}

/// A travel intent under way at the world's start (Ruling 13): the body is heading for
/// `target` at `speed_cm_s`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TravelSpec {
    /// The traveller's raw id.
    pub entity_id: u64,
    /// Where it is going: a place to enter or a thing to reach.
    pub target: u64,
    /// Its speed, in centimetres per second.
    pub speed_cm_s: i64,
}

/// A body's size (Amendment A-3), in centimetres: it spans `half_width` to either side of its
/// base, `half_depth` fore and aft, and `height` upward.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BodySpec {
    /// The body's raw id.
    pub entity_id: u64,
    /// Half its width (along its own x axis), in centimetres.
    pub half_width: i64,
    /// Half its depth (along its own y axis, front to back), in centimetres.
    pub half_depth: i64,
    /// Its height above its base, in centimetres.
    pub height: i64,
}

/// A body's facing (Amendment A-3): a compass bearing in hundredths of a degree, clockwise
/// from its frame's north.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FacingSpec {
    /// The body's raw id.
    pub entity_id: u64,
    /// Its heading, in hundredths of a degree (any value; it wraps).
    pub heading: i64,
}

/// Motion under way at the world's start (Amendment A-3): the body is travelling in a straight
/// line from its position toward `target` (its container's frame, centimetres) and arrives
/// `seconds` of simulated time after the world begins.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MotionSpec {
    /// The body's raw id.
    pub entity_id: u64,
    /// Where it is heading, in centimetres in its container's frame.
    pub target: [i64; 3],
    /// How long until it arrives, in seconds of simulated time.
    pub seconds: u64,
}

/// A seeded region membership (Vol. III Ch. 1 §1.7): `location_id` lies within `region_id`,
/// alongside — not instead of — its place in the containment hierarchy. One location may have
/// many such links (a farmhouse in a watershed, a climate zone, and a territory at once). The
/// region entity needs no facts of its own: a classification region is the places that name it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegionMembershipSpec {
    /// The member location's (or region's) raw id.
    pub location_id: u64,
    /// The region it lies within (a region entity's raw id).
    pub region_id: u64,
}

/// A property a material may expose (Vol. III Ch. 1 §1.9). Materials expose *characteristics*,
/// never identities; this enum is the closed set of characteristics the loader can seed today.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MaterialProperty {
    /// Density, in kg/m³.
    Density,
    /// Hardness / structural strength, 0..=10000.
    Hardness,
    /// Specific heat capacity, in J/(kg·K).
    ThermalCapacity,
    /// Flammability, 0..=10000.
    Flammability,
    /// Conductivity, 0..=10000.
    Conductivity,
    /// Toxicity, 0..=10000.
    Toxicity,
    /// How far it can be taken into a body, 0..=10000 (Amendment A-12).
    Edible,
    /// How strongly a dose acts on a body, 0..=10000 (Amendment A-13).
    Potency,
    /// How much dependence a dose builds, 0..=10000 (Amendment A-13).
    Habit,
    /// How much a unit feeds a body, 0..=10000 (Amendment A-16).
    Nutrition,
}

/// One material the world defines (Vol. III Ch. 1 §1.9): a material entity and the properties
/// it exposes. A material exposes only the characteristics it has, so `properties` may be a
/// partial set — the loader seeds exactly what is declared, never a default.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct MaterialSpec {
    /// The material entity's raw id.
    pub id: u64,
    /// The properties this material exposes, each with its fixed-point value.
    pub properties: Vec<(MaterialProperty, i64)>,
}

/// A seeded composition link (Vol. III Ch. 1 §1.9): physical object `object_id` is made, in
/// part, of material `material_id`. One object may have several such links (a composite).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MadeOfSpec {
    /// The composed object's raw id.
    pub object_id: u64,
    /// A material it is made of (a material entity's raw id).
    pub material_id: u64,
}

/// A package's identity card (Vol. IV Ch. 1 §1.2, The Manifest).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Manifest {
    /// Namespaced, stable package id (e.g. `"world.wilderness"`).
    pub id: String,
    /// The package's own version.
    pub version: Version,
    /// The range of engine versions this package may run against (enforced, not advisory).
    pub engine: EngineReq,
    /// The domains this package selects. Physical Reality must be present (Vol. IV Ch. 2).
    pub domains: Vec<String>,
}

/// The world's clock rule (Vol. II Ch. 2, *Simulated Duration*, Amendment A-1): how much
/// simulated time a tick lasts and how long a day is. Every rate elsewhere in the package is
/// declared in simulated time, so changing `tick_ms` changes resolution, never behaviour.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ClockRules {
    /// Simulated length of one tick, in milliseconds (always positive).
    pub tick_ms: u64,
    /// Simulated length of one day/night cycle, in seconds.
    pub day_seconds: u64,
}

/// Tunable environmental rules the physical domain consumes (Vol. IV Ch. 2 §2.2). Every
/// number here is package data; none is hardcoded in the engine (invariant 5). Rules about
/// change over time are rates or statistics in simulated time (Amendment A-1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalRules {
    /// How often the environment steps, in seconds of simulated time.
    pub environment_step_seconds: u64,
    /// Peak diurnal temperature swing, in centidegrees Celsius.
    pub diurnal_amplitude_centi_c: i64,
    /// Typical size (standard deviation) of weather's departure from normal temperature, in
    /// centidegrees Celsius.
    pub temperature_variability_centi_c: i64,
    /// How long a spell of weather tends to last, in seconds of simulated time.
    pub weather_persistence_seconds: u64,
    /// Illumination at midday, in hundredths of a percent (0..=10000).
    pub illumination_peak: i64,
    /// Humidity baseline the weather departs from, in hundredths of a percent.
    pub humidity_baseline: i64,
    /// Typical size of weather's departure from baseline humidity, in hundredths of a percent.
    pub humidity_variability: i64,
    /// Baseline atmospheric pressure at the datum, in decapascals.
    pub pressure_sea_level: i64,
    /// Decapascals of pressure lost per metre of elevation.
    pub pressure_elevation_factor: i64,
    /// Typical size of weather's departure from baseline pressure, in decapascals.
    pub pressure_variability: i64,
    /// Divisor scaling wind speed per unit pressure gradient (larger = gentler wind).
    pub wind_gradient_divisor: i64,
    /// Danger points added per metre of a portal's height above the ground (fall danger).
    pub fall_danger_per_meter: i64,
    /// Thermal mass — heat stored per unit volume, kJ/(m³·K) — at which a region's temperature
    /// swing is halved (Vol. III Ch. 1 §1.9; Amendment A-6). Governs how strongly thermal mass
    /// resists the day/night swing and the weather, and lengthens an indoor room's lag.
    pub thermal_mass_reference: i64,
    /// Gravitational acceleration, in centimetres per second squared.
    pub gravity_cm_s2: i64,
    /// The highest a body steps up without climbing, in centimetres.
    pub step_height_cm: i64,
    /// The steepest ground a body walks over, as a percentage grade.
    pub max_slope_percent: i64,
    /// The cell size of a travel planning grid, in centimetres.
    pub nav_cell_cm: i64,
    /// How far beyond its own body a body can reach to operate something, in centimetres.
    pub reach_cm: i64,
    /// Time constant of a sheltered room's air following the air outside it, in seconds.
    pub indoor_coupling_seconds: u64,
    /// How often what each sighted body can see is refreshed, in seconds (Amendment A-8).
    pub sight_step_seconds: u64,
    /// The least illumination, in hundredths of a percent, in which a thing can be seen.
    pub sight_min_illumination: i64,
    /// The most a body can carry, in kilograms (Amendment A-12).
    pub carry_limit_kg: i64,
}

/// Tunable rules decision systems decide by (Vol. IV Ch. 2 §2.2; Amendment A-9).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MindsRules {
    /// How often a mind thinks, in seconds of simulated time.
    pub think_step_seconds: u64,
    /// Body heat, in centidegrees, below which a mind feels cold.
    pub cold_below_centi_c: i64,
    /// The age, in seconds, at which a belief is trusted half as much as a fresh one.
    pub trust_half_age_seconds: u64,
    /// What each opening on the way costs a choice.
    pub hop_cost: i64,
    /// What keeping a routine is worth.
    pub routine_value: i64,
    /// How much better a new choice must be before a mind abandons the one it has made.
    pub switch_margin: i64,
    /// Fatigue above which a mind is tired (Amendment A-10).
    pub tired_above: i64,
    /// Fatigue below which a resting mind is rested.
    pub rested_below: i64,
    /// How strongly a need that arose must be felt before it moves a mind (Amendment A-11).
    pub need_above: i64,
    /// What each hundredth of a percent of need beyond that line is worth, in percent.
    pub need_weight: i64,
    /// Hunger past which a mind goes looking for food (Amendment A-16).
    pub hungry_above: i64,
    /// What working one's job is worth, in its hours (Amendment A-18).
    pub work_value: i64,
    /// How far past the tired line a mind up and about its routine or work carries on
    /// (Amendment A-20).
    pub tired_margin: i64,
    /// The world's sleeping hours, `(from, to)` in whole hours; wrapping past midnight when
    /// `from > to` (Amendment A-18).
    pub sleep_hours: (i64, i64),
}

/// Tunable rules deposits follow (Amendment A-15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ResourcesRules {
    /// How often deposits regrow, in seconds.
    pub regrow_step_seconds: u64,
}

/// A deposit: a thing in the world holding a stock that regrows, and yields an item per pick
/// (Amendment A-15).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct DepositSpec {
    /// The deposit's entity id.
    pub id: u64,
    /// What each unit is made of (a material id).
    pub made_of: u64,
    /// Each unit's size, `[half-width, half-depth, height]`, cm.
    pub size: [i64; 3],
    /// Units regrown per day.
    pub per_day: i64,
    /// The most units it holds.
    pub cap: i64,
    /// Units it holds at the start.
    pub stock: i64,
}

/// A recipe: what it needs, what it makes, where, and how long the making takes
/// (Amendment A-17).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct RecipeSpec {
    /// The recipe's entity id.
    pub id: u64,
    /// What it needs: `(material, how many things made of it)`.
    pub needs: Vec<(u64, i64)>,
    /// What the product is made of (a material id).
    pub makes: u64,
    /// The product's size, `[half-width, half-depth, height]`, cm.
    pub size: [i64; 3],
    /// The workplace it is made at.
    pub at: u64,
    /// How long the product is in the making, in seconds.
    pub takes_seconds: i64,
}

/// What a job does (Amendment A-18): one of the engine's closed set of mechanisms.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum JobWork {
    /// Bring things of a material from a source (a deposit, or a place) to the store.
    Carry {
        /// The material carried.
        material: u64,
        /// Where it is got from.
        from: u64,
    },
    /// Keep the store supplied with what a recipe makes.
    Make {
        /// The recipe.
        recipe: u64,
    },
}

/// A kind of job (Amendment A-18).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JobSpec {
    /// The job's entity id.
    pub id: u64,
    /// What it does.
    pub work: JobWork,
    /// Its store: the place its goods go to.
    pub to: u64,
    /// How many goods it keeps in the store.
    pub keep: i64,
    /// Its hours, `(from, to)` in whole hours of the day; wrapping past midnight when
    /// `from > to`.
    pub hours: (i64, i64),
}

/// Tunable rules Society's bonds follow (Amendment A-11).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct SocietyRules {
    /// How often bonds are made and broken, in seconds.
    pub courtship_step_seconds: u64,
    /// Fondness, both ways, past which two persons become lovers.
    pub bond_above: i64,
    /// Fondness, both ways, below which lovers part.
    pub part_below: i64,
}

/// A kind of need that can arise (Amendment A-11): what makes it arise, what meets it, and its
/// rates, from the engine's closed set of mechanisms.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct NeedKindSpec {
    /// The kind's entity id.
    pub id: u64,
    /// 1, from a bond; 2, from a dependence.
    pub arises: i64,
    /// 1, by presence; 2, by a dose.
    pub met_by: i64,
    /// Growth per hour unmet, hundredths of a percent.
    pub rise: i64,
    /// Easing per hour met.
    pub ease: i64,
    /// Health lost per hour, felt in full.
    pub harm: i64,
    /// For a kind that arises from dependence, the dependence past which it arises (0 for
    /// others; Amendment A-13).
    pub above: i64,
}

/// Tunable rules the information layer consumes (Vol. IV Ch. 2 §2.2; Amendment A-8).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct InformationRules {
    /// How often minds perceive, in seconds of simulated time.
    pub perception_step_seconds: u64,
    /// The least change of warmth, in centidegrees, a mind notices.
    pub warmth_resolution_centi_c: i64,
    /// The least change of fatigue or health, in hundredths of a percent, a mind notices.
    pub need_resolution: i64,
    /// How often fondness grows or fades, in seconds (Amendment A-11).
    pub affection_step_seconds: u64,
    /// Fondness gained per hour in another's sight, at full compatibility.
    pub affection_per_hour: i64,
    /// Fondness lost per day out of another's sight.
    pub affection_fade_per_day: i64,
}

/// Tunable metabolic rules the living domain consumes (Vol. IV Ch. 2 §2.2), as time constants
/// in simulated time (Amendment A-1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LivingRules {
    /// How often metabolism steps, in seconds of simulated time.
    pub metabolism_step_seconds: u64,
    /// Metabolic set-point body heat, in centidegrees Celsius.
    pub set_point_centi_c: i64,
    /// Time constant of the pull toward the set point, in seconds (larger = slower).
    pub warm_response_seconds: u64,
    /// Time constant of the pull toward ambient temperature, in seconds (larger = slower).
    pub cold_response_seconds: u64,
    /// Fatigue gathered per waking hour, in hundredths of a percent (Amendment A-10).
    pub tire_per_hour: i64,
    /// Fatigue shed per hour of rest.
    pub rest_per_hour: i64,
    /// Body heat, in centidegrees, below which the cold harms.
    pub hypothermia_below_centi_c: i64,
    /// Health lost per degree below that line, per hour.
    pub cold_harm_per_degree_hour: i64,
    /// The highest fall, in centimetres, that does no harm.
    pub safe_fall_cm: i64,
    /// Health lost per metre fallen beyond the safe drop.
    pub fall_harm_per_metre: i64,
    /// Health recovered per hour while nothing harms.
    pub heal_per_hour: i64,
    /// Dependence lost per day without the substance (Amendment A-13).
    pub dependence_fade_per_day: i64,
    /// Hunger gathered per hour (Amendment A-16).
    pub hunger_per_hour: i64,
    /// Hunger past which a body is starving.
    pub starving_above: i64,
    /// Health lost per hour while starving.
    pub starving_harm_per_hour: i64,
}

/// One region the world begins with (Vol. IV Ch. 4, generation): an id, a starting
/// temperature, and an optional elevation, in fixed-point centi-units.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegionSpec {
    /// The region entity's raw id.
    pub id: u64,
    /// Its initial temperature, in centidegrees Celsius.
    pub temperature_centi_c: i64,
    /// Its elevation in centimetres above the datum, if the package specifies one.
    pub elevation: Option<i64>,
}

/// One organism the world begins with (Vol. IV Ch. 4, generation): an id, the region it
/// inhabits (seeded as a containment fact), and its starting body heat.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OrganismSpec {
    /// The organism entity's raw id.
    pub id: u64,
    /// The raw id of the region it inhabits (seeded as `contained_in`).
    pub region_id: u64,
    /// Its initial body heat, in centidegrees Celsius.
    pub body_heat_centi_c: i64,
}

/// A seeded containment link (Vol. III Ch. 1 §1.8): `child` exists within `parent`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ContainmentSpec {
    /// The contained entity's raw id.
    pub child_id: u64,
    /// The container entity's raw id.
    pub parent_id: u64,
}

/// A seeded undirected adjacency edge (Vol. III Ch. 1 §1.5): regions `a` and `b` border each
/// other.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AdjacencySpec {
    /// One region's raw id.
    pub a: u64,
    /// The neighbouring region's raw id.
    pub b: u64,
}

/// A region's seeded exposure to the open sky (Vol. III Ch. 1 §1.6), in hundredths of a
/// percent (0..=10000): sealed chamber 0, cave mouth or canopy partial, open ground full.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ExposureSpec {
    /// The region entity's raw id.
    pub region_id: u64,
    /// Its exposure, 0..=10000.
    pub exposure: i64,
}

/// A seeded local position of an entity within its immediate container (Vol. III Ch. 1 §1.3),
/// in centimetres. Z is optional (absent = 0, ground level).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PositionSpec {
    /// The entity's raw id.
    pub entity_id: u64,
    /// Local X in centimetres.
    pub x: i64,
    /// Local Y in centimetres.
    pub y: i64,
    /// Local Z (height) in centimetres, if specified.
    pub z: Option<i64>,
}

/// A seeded portal (Vol. III Ch. 1 §1.5): a located connection from a spot in `host_region`
/// to `dest_region`. The portal is an entity placed in its host at (x, y[, z]); it leads to
/// its destination. Z optional.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PortalSpec {
    /// The portal entity's raw id.
    pub portal_id: u64,
    /// The region the portal is located in.
    pub host_region: u64,
    /// The region the portal leads to.
    pub dest_region: u64,
    /// Local X of the portal within its host, in centimetres.
    pub x: i64,
    /// Local Y of the portal within its host, in centimetres.
    pub y: i64,
    /// Local Z of the portal within its host, if specified.
    pub z: Option<i64>,
}

/// A world-pinned danger value for a portal (Vol. III Ch. 1 §1.11), 0..=10000 -- overrides
/// the height-derived default.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PortalDangerSpec {
    /// The portal entity's raw id.
    pub portal_id: u64,
    /// Its fixed danger, 0..=10000.
    pub danger: i64,
}
