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
    /// The regions the world begins with, each with a starting temperature.
    pub regions: Vec<RegionSpec>,
    /// The organisms the world begins with (living domain), each placed in a region.
    pub organisms: Vec<OrganismSpec>,
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

/// Tunable climate rules the physical domain consumes (Vol. IV Ch. 2 §2.2). Every number
/// here is package data; none is hardcoded in the engine (invariant 5).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalRules {
    /// Ticks in one day/night cycle.
    pub ticks_per_day: u64,
    /// Peak diurnal temperature swing, in centidegrees Celsius.
    pub diurnal_amplitude_centi_c: i64,
    /// Maximum per-tick weather perturbation, in centidegrees Celsius.
    pub weather_max_swing_centi_c: i64,
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

/// One region the world begins with (Vol. IV Ch. 4, generation): an id and a starting
/// temperature in centidegrees Celsius.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RegionSpec {
    /// The region entity's raw id.
    pub id: u64,
    /// Its initial temperature, in centidegrees Celsius.
    pub temperature_centi_c: i64,
}

/// One organism the world begins with (Vol. IV Ch. 4, generation): an id, the region it
/// inhabits, and its starting body heat in centidegrees Celsius.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct OrganismSpec {
    /// The organism entity's raw id.
    pub id: u64,
    /// The raw id of the region it inhabits (whose temperature it senses).
    pub region_id: u64,
    /// Its initial body heat, in centidegrees Celsius.
    pub body_heat_centi_c: i64,
}
