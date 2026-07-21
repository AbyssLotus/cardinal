//! # Physical Reality domain -- Vol. III Ch. 1
//!
//! Owns (Appendix A): space (position, containment, elevation, adjacency), connectivity,
//! materials, and environmental state. "Physical Reality rarely owns the actors. It owns the
//! stage upon which they act" (Vol. III Ch. 1 §1.13).
//!
//! **Mandatory in every world** (Vol. IV Ch. 2 §2.1): the one domain that may never be
//! disabled -- every fact needs somewhere to exist (Vol. III Ch. 1 §1.4).
//!
//! **Domains never import domains** (Vol. V Ch. 1 §1.1, rule 2). This crate depends on
//! `kernel` and nothing else in the workspace; cross-domain effect happens through committed
//! facts, never direct calls (Vol. III Ch. 12 §12.1).
//!
//! ## What this crate represents so far
//! Space: [`schema::CONTAINED_IN`] (immediate containment, walked into a hierarchy),
//! [`schema::ELEVATION`], and [`schema::ADJACENT_TO`] (a cardinality-many topology). The
//! environmental fields, each varying across space and time (Vol. III Ch. 1 §1.10):
//! [`schema::TEMPERATURE`], [`schema::ILLUMINATION`] (a day/night cycle), [`schema::HUMIDITY`]
//! (weather), [`schema::PRESSURE`] (falls with elevation, drifts with weather), and wind --
//! [`schema::WIND_SPEED`] and [`schema::WIND_TOWARD`] -- which flows down the pressure
//! gradient across adjacent regions. Overlapping regions (§1.7) and materials (§1.9) remain
//! to build on the multi-value foundation.

pub mod composition;
pub mod schema;
pub mod space;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::{Cardinality, FactType};
use kernel::identity::EntityId;
use kernel::proposal::Change;
use kernel::system::System;
use kernel::value::Value;

/// The tunable rules the physical domain consumes, all sourced from the world package
/// (Vol. IV Ch. 2 §2.2, invariant 5) — no climate, field, or wind number is hardcoded in
/// engine code.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalConfig {
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
    /// Divisor governing how fast pressure returns to its baseline (larger = slower).
    pub pressure_settle_divisor: i64,
    /// Divisor scaling wind speed per unit pressure gradient (larger = gentler wind).
    pub wind_gradient_divisor: i64,
}

/// The Physical Reality domain, plugged into the kernel (Appendix A owner of the stage).
///
/// Configured over a set of regions sharing one set of environmental rules; each region
/// carries its own temperature, illumination, humidity, pressure, and wind, evolving under
/// its own weather substreams. Elevation, containment, and adjacency are seeded facts (state,
/// not system-driven here).
pub struct PhysicalDomain {
    regions: Vec<EntityId>,
    config: PhysicalConfig,
}

impl PhysicalDomain {
    /// Configure the domain over `regions` with the given environmental rules.
    pub fn new(regions: Vec<EntityId>, config: PhysicalConfig) -> Self {
        Self { regions, config }
    }
}

impl Domain for PhysicalDomain {
    fn name(&self) -> &'static str {
        "physical"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        fact_type == schema::TEMPERATURE
            || fact_type == schema::ILLUMINATION
            || fact_type == schema::HUMIDITY
            || fact_type == schema::PRESSURE
            || fact_type == schema::WIND_SPEED
            || fact_type == schema::WIND_TOWARD
            || fact_type == schema::ELEVATION
            || fact_type == schema::EXPOSURE
            || fact_type == schema::POSITION_X
            || fact_type == schema::POSITION_Y
            || fact_type == schema::POSITION_Z
            || fact_type == schema::CONTAINED_IN
            || fact_type == schema::ADJACENT_TO
            || fact_type == schema::LEADS_TO
            || fact_type == schema::HAS_PORTAL
    }

    fn cardinality(&self, fact_type: FactType) -> Cardinality {
        // Adjacency is set-valued: a region has several neighbours (Vol. III Ch. 1 §1.5).
        if fact_type == schema::ADJACENT_TO || fact_type == schema::HAS_PORTAL {
            Cardinality::Many
        } else {
            Cardinality::One
        }
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        let mut out: Vec<Box<dyn System>> = Vec::with_capacity(self.regions.len() * 6);
        for &region in &self.regions {
            out.push(Box::new(systems::DiurnalCycle::new(
                region,
                self.config.ticks_per_day,
                self.config.diurnal_amplitude_centi_c,
            )));
            out.push(Box::new(systems::WeatherNoise::new(
                region,
                self.config.weather_max_swing_centi_c,
            )));
            out.push(Box::new(systems::DayNightCycle::new(
                region,
                self.config.ticks_per_day,
                self.config.illumination_peak,
            )));
            out.push(Box::new(systems::Precipitation::new(
                region,
                self.config.humidity_baseline,
                self.config.humidity_swing,
                self.config.humidity_drying_divisor,
            )));
            out.push(Box::new(systems::PressureSystem::new(
                region,
                self.config.pressure_sea_level,
                self.config.pressure_elevation_factor,
                self.config.pressure_weather_swing,
                self.config.pressure_settle_divisor,
            )));
            out.push(Box::new(systems::WindSystem::new(
                region,
                self.config.wind_gradient_divisor,
                schema::MAX_WIND,
            )));
        }
        out
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::TEMPERATURE
            || fact_type == schema::ELEVATION
            || fact_type == schema::POSITION_X
            || fact_type == schema::POSITION_Y
            || fact_type == schema::POSITION_Z
        {
            composition::compose_additive(current, changes)
        } else if fact_type == schema::ILLUMINATION
            || fact_type == schema::HUMIDITY
            || fact_type == schema::EXPOSURE
        {
            composition::compose_bounded(current, changes, 0, schema::PERCENT_FULL)
        } else if fact_type == schema::PRESSURE {
            composition::compose_bounded(current, changes, 0, schema::MAX_PRESSURE)
        } else if fact_type == schema::WIND_SPEED {
            composition::compose_bounded(current, changes, 0, schema::MAX_WIND)
        } else if fact_type == schema::CONTAINED_IN
            || fact_type == schema::WIND_TOWARD
            || fact_type == schema::LEADS_TO
        {
            composition::compose_entity_ref(current, changes)
        } else {
            Err(ResolveError::new(
                "physical: fact type not owned by this domain",
            ))
        }
    }

    fn validate(&self, fact_type: FactType, value: &Resolved) -> Result<(), ValidationError> {
        if fact_type == schema::TEMPERATURE {
            if let Resolved::Write(Value::Int(centi_c)) = value {
                if *centi_c < schema::ABSOLUTE_ZERO_CENTI_C {
                    return Err(ValidationError::new(
                        "temperature resolved below absolute zero",
                    ));
                }
            }
        } else if fact_type == schema::CONTAINED_IN
            || fact_type == schema::WIND_TOWARD
            || fact_type == schema::LEADS_TO
        {
            if let Resolved::Write(v) = value {
                if !matches!(v, Value::Entity(_)) {
                    return Err(ValidationError::new(
                        "this spatial fact must reference an entity",
                    ));
                }
            }
        }
        Ok(())
    }
}
