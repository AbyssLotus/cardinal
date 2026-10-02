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
//! gradient across adjacent regions. Materials (§1.9): what objects are *made of*
//! ([`schema::MADE_OF`], a cardinality-many composition) and the properties their materials
//! expose ([`schema::MATERIAL_HARDNESS`], flammability, density, …), queried through
//! [`materials`] by property, never by name. Regions (§1.7): beyond the one containment
//! hierarchy, an entity may belong to any number of overlapping, possibly discontiguous
//! regions ([`schema::IN_REGION`], cardinality-many) -- a watershed, a climate zone, a fox's
//! territory -- answered through [`regions`] ("what regions is this in", "what lies in this
//! region", "do these two overlap"). Proximity (Amendment A-2): what a container holds, what
//! lies within a distance, and the nearest few — answered through the store's spatial index
//! when one is installed ([`index::PhysicalProjector`] is this domain's placement rule), and by
//! scanning otherwise, with identical results ([`nearby`]). Bodies (Amendment A-3): one
//! position fact per entity ([`schema::POSITION`], its base, in its container's frame), a size
//! ([`schema::BODY_SIZE`]), a facing ([`schema::HEADING`]) that also turns the frame of whatever a
//! container holds, and motion as a straight segment written when it starts and ends and
//! derived in between ([`motion`]), so "where is it now", "how fast, which way", and "is it on my
//! left" are all answerable without a write per tick.

pub mod composition;
pub mod index;
pub mod materials;
pub mod motion;
pub mod nearby;
pub mod regions;
pub mod schema;
pub mod space;
pub mod systems;

use kernel::domain::{Domain, ResolveError, Resolved, ValidationError};
use kernel::fact::{Cardinality, FactType};
use kernel::proposal::Change;
use kernel::spatial::SpatialProjector;
use kernel::system::System;
use kernel::time::SimClock;
use kernel::value::Value;
use std::sync::Arc;

/// The tunable rules the physical domain consumes, all sourced from the world package
/// (Vol. IV Ch. 2 §2.2, invariant 5) — no climate, field, or wind number is hardcoded in
/// engine code. Every rule about change over time is a rate or a statistic in simulated time
/// (Vol. II Ch. 2, Amendment A-1), so the same rules give the same climate at any tick length.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct PhysicalConfig {
    /// The world's clock: how much simulated time one tick lasts.
    pub clock: SimClock,
    /// The length of one day/night cycle, in seconds of simulated time.
    pub day_length_seconds: u64,
    /// How often the environment (temperature, light, humidity, pressure, wind) steps, in
    /// seconds of simulated time. Rounded to whole ticks; each step advances exactly the time
    /// it covers.
    pub environment_step_seconds: u64,
    /// Peak diurnal temperature swing, in centidegrees Celsius.
    pub diurnal_amplitude_centi_c: i64,
    /// Typical size (standard deviation) of weather's departure from normal temperature, in
    /// centidegrees Celsius.
    pub temperature_variability_centi_c: i64,
    /// How long a spell of weather tends to last — the memory of the temperature, humidity,
    /// and pressure anomalies — in seconds of simulated time.
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
    /// Material thermal capacity (J/(kg·K)) at which a region's temperature swing is halved
    /// (Vol. III Ch. 1 §1.9). Larger means thermal mass matters less; a region built of no
    /// thermal-mass material is undamped, exactly as before materials existed.
    pub thermal_mass_reference: i64,
}

/// The Physical Reality domain, plugged into the kernel (Appendix A owner of the stage).
///
/// Configured with one set of environmental rules shared by every region. It carries no
/// region list: its systems discover the regions to simulate from committed reality each
/// tick (Vol. V Ch. 2 §2.1, clause 5), so a region seeded at load and a region created
/// mid-simulation are treated alike. Each region carries its own temperature, illumination,
/// humidity, pressure, and wind, evolving under its own weather substreams; elevation,
/// containment, and adjacency are seeded facts (state, not system-driven here).
pub struct PhysicalDomain {
    config: PhysicalConfig,
}

impl PhysicalDomain {
    /// Configure the domain with the given environmental rules.
    pub fn new(config: PhysicalConfig) -> Self {
        Self { config }
    }
}

impl Domain for PhysicalDomain {
    fn name(&self) -> &'static str {
        "physical"
    }

    fn owns(&self, fact_type: FactType) -> bool {
        fact_type == schema::TEMPERATURE
            || fact_type == schema::TEMPERATURE_ANOMALY
            || fact_type == schema::HUMIDITY_ANOMALY
            || fact_type == schema::PRESSURE_ANOMALY
            || fact_type == schema::ILLUMINATION
            || fact_type == schema::HUMIDITY
            || fact_type == schema::PRESSURE
            || fact_type == schema::WIND_SPEED
            || fact_type == schema::WIND_TOWARD
            || fact_type == schema::ELEVATION
            || fact_type == schema::EXPOSURE
            || fact_type == schema::POSITION
            || fact_type == schema::BODY_SIZE
            || fact_type == schema::HEADING
            || fact_type == schema::MOTION_TARGET
            || fact_type == schema::MOTION_START
            || fact_type == schema::MOTION_END
            || fact_type == schema::CONTAINED_IN
            || fact_type == schema::IN_REGION
            || fact_type == schema::ADJACENT_TO
            || fact_type == schema::LEADS_TO
            || fact_type == schema::HAS_PORTAL
            || fact_type == schema::PORTAL_DANGER
            || fact_type == schema::PORTAL_DANGER_OVERRIDE
            || fact_type == schema::MADE_OF
            || fact_type == schema::MATERIAL_DENSITY
            || fact_type == schema::MATERIAL_HARDNESS
            || fact_type == schema::MATERIAL_THERMAL_CAPACITY
            || fact_type == schema::MATERIAL_FLAMMABILITY
            || fact_type == schema::MATERIAL_CONDUCTIVITY
            || fact_type == schema::MATERIAL_TOXICITY
    }

    fn spatial_projector(&self) -> Option<Arc<dyn SpatialProjector>> {
        // Space is Physical Reality's (Appendix A): this domain tells the store where things
        // are, and the store keeps the index current (Amendment A-2).
        Some(Arc::new(index::PhysicalProjector))
    }

    fn cardinality(&self, fact_type: FactType) -> Cardinality {
        // Set-valued relations: a region has several neighbours and may host several portals
        // (Vol. III Ch. 1 §1.5); a location may lie in several overlapping regions (§1.7); an
        // object may be a composite of several materials (§1.9).
        if fact_type == schema::ADJACENT_TO
            || fact_type == schema::HAS_PORTAL
            || fact_type == schema::IN_REGION
            || fact_type == schema::MADE_OF
        {
            Cardinality::Many
        } else {
            Cardinality::One
        }
    }

    fn systems(&self) -> Vec<Box<dyn System>> {
        // One instance of each system kind; each discovers its regions from committed reality
        // and iterates them, so the count is fixed regardless of world size. The environment
        // steps together, on one cadence in simulated time (Vol. V Ch. 3 §3.2, Amendment A-1).
        let c = self.config;
        let step = c
            .clock
            .step(c.environment_step_seconds.saturating_mul(1000));
        let day_ms = c.day_length_seconds.saturating_mul(1000);
        let memory_ms = c.weather_persistence_seconds.saturating_mul(1000);
        vec![
            Box::new(systems::DiurnalCycle::new(
                c.clock,
                step,
                day_ms,
                c.diurnal_amplitude_centi_c,
                c.thermal_mass_reference,
            )),
            Box::new(systems::TemperatureWeather::new(
                step,
                c.temperature_variability_centi_c,
                memory_ms,
                c.thermal_mass_reference,
            )),
            Box::new(systems::DayNightCycle::new(
                c.clock,
                step,
                day_ms,
                c.illumination_peak,
            )),
            Box::new(systems::Precipitation::new(
                step,
                c.humidity_baseline,
                c.humidity_variability,
                memory_ms,
            )),
            Box::new(systems::PressureSystem::new(
                step,
                c.pressure_sea_level,
                c.pressure_elevation_factor,
                c.pressure_variability,
                memory_ms,
            )),
            Box::new(systems::WindSystem::new(
                step,
                c.wind_gradient_divisor,
                schema::MAX_WIND,
            )),
            Box::new(systems::PortalDanger::new(c.fall_danger_per_meter)),
            // Closes finished motion segments: one write per arrival (Amendment A-3).
            Box::new(motion::Settle),
        ]
    }

    fn compose(
        &self,
        fact_type: FactType,
        current: Option<Value>,
        changes: &[Change],
    ) -> Result<Resolved, ResolveError> {
        if fact_type == schema::TEMPERATURE
            || fact_type == schema::TEMPERATURE_ANOMALY
            || fact_type == schema::HUMIDITY_ANOMALY
            || fact_type == schema::PRESSURE_ANOMALY
            || fact_type == schema::ELEVATION
        {
            composition::compose_additive(current, changes)
        } else if fact_type == schema::MOTION_TARGET
            || fact_type == schema::MOTION_START
            || fact_type == schema::MOTION_END
        {
            composition::compose_segment_field(current, changes)
        } else if fact_type == schema::POSITION {
            composition::compose_vec3(current, changes, None)
        } else if fact_type == schema::BODY_SIZE {
            composition::compose_vec3(current, changes, Some((0, schema::MAX_SIZE)))
        } else if fact_type == schema::HEADING {
            composition::compose_heading(current, changes)
        } else if fact_type == schema::ILLUMINATION
            || fact_type == schema::HUMIDITY
            || fact_type == schema::EXPOSURE
        {
            composition::compose_bounded(current, changes, 0, schema::PERCENT_FULL)
        } else if fact_type == schema::PORTAL_DANGER || fact_type == schema::PORTAL_DANGER_OVERRIDE
        {
            composition::compose_bounded(current, changes, 0, schema::MAX_DANGER)
        } else if fact_type == schema::PRESSURE {
            composition::compose_bounded(current, changes, 0, schema::MAX_PRESSURE)
        } else if fact_type == schema::WIND_SPEED {
            composition::compose_bounded(current, changes, 0, schema::MAX_WIND)
        } else if fact_type == schema::MATERIAL_HARDNESS
            || fact_type == schema::MATERIAL_FLAMMABILITY
            || fact_type == schema::MATERIAL_CONDUCTIVITY
            || fact_type == schema::MATERIAL_TOXICITY
        {
            // Normalized material properties (Vol. III Ch. 1 §1.9), bounded like the other
            // 0..=100% fields. Seeded state today; the rule keeps them coherent should a
            // system (weathering, damage) ever drive them.
            composition::compose_bounded(current, changes, 0, schema::PERCENT_FULL)
        } else if fact_type == schema::MATERIAL_DENSITY {
            composition::compose_bounded(current, changes, 0, schema::MAX_DENSITY)
        } else if fact_type == schema::MATERIAL_THERMAL_CAPACITY {
            composition::compose_bounded(current, changes, 0, schema::MAX_THERMAL_CAPACITY)
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
        if fact_type == schema::MOTION_START || fact_type == schema::MOTION_END {
            // A segment is timed in ticks, and there is no tick before the world began.
            match value {
                Resolved::Write(Value::Int(tick)) if *tick < 0 => {
                    return Err(ValidationError::new(
                        "a motion segment cannot be timed before tick 0",
                    ))
                }
                Resolved::Write(Value::Int(_)) | Resolved::Tombstone => {}
                Resolved::Write(_) => {
                    return Err(ValidationError::new("a segment is timed by a tick number"))
                }
            }
        }
        if fact_type == schema::MOTION_TARGET {
            if let Resolved::Write(v) = value {
                if v.as_vec3().is_none() {
                    return Err(ValidationError::new("a motion target is a point"));
                }
            }
        }
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

    fn validate_many(&self, fact_type: FactType, values: &[Value]) -> Result<(), ValidationError> {
        // The set-valued relations are graphs over entities: every member of an adjacency or
        // portal-host set (Vol. III Ch. 1 §1.5), a region-membership set (§1.7), or a
        // material-composition set (§1.9), must be an entity reference, never a scalar. This
        // is the coherence check the cardinality-one path gets from `compose`/`validate`,
        // applied to the whole resolved set.
        if fact_type == schema::ADJACENT_TO
            || fact_type == schema::HAS_PORTAL
            || fact_type == schema::IN_REGION
            || fact_type == schema::MADE_OF
        {
            for v in values {
                if !matches!(v, Value::Entity(_)) {
                    return Err(ValidationError::new(
                        "a relation set may hold only entity references",
                    ));
                }
            }
        }
        Ok(())
    }
}
