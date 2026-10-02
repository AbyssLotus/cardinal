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
