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
