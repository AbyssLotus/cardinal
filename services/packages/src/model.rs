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

/// Tunable environmental rules the physical domain consumes (Vol. IV Ch. 2 §2.2). Every
/// number here is package data; none is hardcoded in the engine (invariant 5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalRules {
    /// Ticks in one day/night cycle (shared by temperature and illumination).
    pub ticks_per_day: u64,
    /// Peak diurnal temperature swing, in centidegrees Celsius.
    pub diurnal_amplitude_centi_c: i64,
    /// Maximum per-tick temperature weather perturbation, in centidegrees Celsius.
    pub weather_max_swing_centi_c: i64,
    /// Illumination at midday, in hundredths of a percent (0..=10000).
    pub illumination_peak: i64,
    /// Humidity baseline the weather drifts toward, in hundredths of a percent.
    pub humidity_baseline: i64,
    /// Maximum per-tick humidity weather perturbation, in hundredths of a percent.
    pub humidity_swing: i64,
    /// Divisor governing how fast humidity returns to baseline (larger = slower).
    pub humidity_drying_divisor: i64,
    /// Baseline atmospheric pressure at the datum, in decapascals.
    pub pressure_sea_level: i64,
    /// Decapascals of pressure lost per metre of elevation.
    pub pressure_elevation_factor: i64,
    /// Maximum per-tick pressure weather perturbation, in decapascals.
    pub pressure_weather_swing: i64,
    /// Divisor governing how fast pressure returns to baseline (larger = slower).
    pub pressure_settle_divisor: i64,
    /// Divisor scaling wind speed per unit pressure gradient (larger = gentler wind).
    pub wind_gradient_divisor: i64,
    /// Danger points added per metre of a portal's height above the ground (fall danger).
    pub fall_danger_per_meter: i64,
}

/// Tunable metabolic rules the living domain consumes (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LivingRules {
    /// Metabolic set-point body heat, in centidegrees Celsius.
    pub set_point_centi_c: i64,
    /// Divisor governing pull toward the set point (larger = slower).
    pub warm_response: i64,
    /// Divisor governing pull toward ambient temperature (larger = slower).
    pub cold_response: i64,
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
