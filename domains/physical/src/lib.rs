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

pub mod act;
pub mod climate;
pub mod composition;
pub mod index;
pub mod materials;
pub mod motion;
pub mod nearby;
pub mod regions;
pub mod schema;
pub mod shape;
pub mod sight;
pub mod space;
pub mod systems;
pub mod terrain;
pub mod travel;

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
    /// Thermal mass — heat stored per unit volume, kJ/(m³·K) — at which a region's temperature
    /// swing is halved (Vol. III Ch. 1 §1.9; Amendment A-6). Larger means thermal mass matters less; a region built of no
    /// thermal-mass material is undamped, exactly as before materials existed.
    pub thermal_mass_reference: i64,
    /// Gravitational acceleration, in centimetres per second squared (Amendment A-4).
    pub gravity_cm_s2: i64,
    /// The highest a body steps up without climbing, in centimetres: a body stands on a solid
    /// top no higher than this above its base, and walks around anything taller.
    pub step_height_cm: i64,
    /// The steepest ground a body walks over, as a percentage grade (100 is 45°).
    pub max_slope_percent: i64,
    /// The cell size of a travel planning grid, in centimetres — how finely a walker finds its
    /// way around obstacles.
    pub nav_cell_cm: i64,
    /// How far beyond its own body a body can reach to operate something, in centimetres — a
    /// door must be this close to be opened or shut (Amendment A-5).
    pub reach_cm: i64,
    /// How quickly a sheltered room's air follows the air outside it: the time constant, in
    /// seconds, of the exchange through its walls and openings (Amendment A-5). Larger is
    /// better insulated; a room built of heavy material is slower still.
    pub indoor_coupling_seconds: u64,
    /// How often what each sighted body can see is refreshed, in seconds of simulated time
    /// (Amendment A-8).
    pub sight_step_seconds: u64,
    /// The least illumination, in hundredths of a percent, in which a thing can be seen
    /// (Amendment A-8): below it, the place it stands in is too dark.
    pub sight_min_illumination: i64,
}

impl PhysicalConfig {
    /// The movement rules gravity and travel share, drawn from this configuration.
    pub const fn move_rules(&self) -> travel::MoveRules {
        travel::MoveRules {
            clock: self.clock,
            gravity_cm_s2: self.gravity_cm_s2,
            step_height_cm: self.step_height_cm,
            max_slope_percent: self.max_slope_percent,
            nav_cell_cm: self.nav_cell_cm,
            reach_cm: self.reach_cm,
        }
    }
}

/// Physical facts only Physical Reality's own systems may write (Amendment A-5; Appendix A,
/// Ruling 13): where a body is and how it moves, which way it faces, whether a door is open, and
/// the reports Physical Reality derives about them. Everyone else proposes intents — where to
/// travel, what to open or shut, which way to face — which are open to all.
pub const RESTRICTED: &[FactType] = &[
    schema::CONTAINED_IN,
    schema::POSITION,
    schema::MOTION_TARGET,
    schema::MOTION_START,
    schema::MOTION_END,
    schema::HEADING,
    schema::PORTAL_OPEN,
    schema::FALL_HEIGHT,
    schema::TRAVEL_BLOCKED,
    schema::ACT_REFUSED,
    schema::PORTAL_DANGER,
    schema::IN_VIEW,
];

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
    /// The ids of the systems this domain registers — the only proposers it accepts for its
    /// [`RESTRICTED`] facts.
    own: Vec<kernel::fact::SystemId>,
}

impl PhysicalDomain {
    /// Configure the domain with the given environmental rules.
    pub fn new(config: PhysicalConfig) -> Self {
        let mut domain = Self {
            config,
            own: Vec::new(),
        };
        domain.own = domain.systems().iter().map(|s| s.id()).collect();
        domain
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
            || fact_type == schema::SOLID
            || fact_type == schema::OPAQUE
            || fact_type == schema::ENCLOSED
            || fact_type == schema::PORTAL_OPEN
            || fact_type == schema::PORTAL_FAR_SIDE
            || fact_type == schema::MOBILE
            || fact_type == schema::TERRAIN_SPACING
            || fact_type == schema::TERRAIN_SAMPLE
            || fact_type == schema::TRAVEL_TO
            || fact_type == schema::TRAVEL_SPEED
            || fact_type == schema::TRAVEL_BLOCKED
            || fact_type == schema::FALL_HEIGHT
            || fact_type == schema::ACT_OPEN
            || fact_type == schema::ACT_CLOSE
            || fact_type == schema::ACT_FACE
            || fact_type == schema::ACT_REFUSED
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
            || fact_type == schema::IN_VIEW
    }

    fn accepts(&self, fact_type: FactType, system: kernel::fact::SystemId) -> bool {
        // Ruling 13, enforced: the restricted facts move only under this domain's own systems.
        !RESTRICTED.contains(&fact_type) || self.own.contains(&system)
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
            || fact_type == schema::TERRAIN_SAMPLE
            || fact_type == schema::IN_VIEW
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
            // Drops what nothing holds up, and carries out travel intents (Amendment A-4).
            Box::new(travel::Gravity::new(c.move_rules())),
            Box::new(travel::Travel::new(c.move_rules())),
            // Opens, shuts, and turns on request, within reach (Amendment A-5).
            Box::new(act::Act::new(c.reach_cm)),
            // Walled rooms' air follows the air outside them (Amendment A-5).
            Box::new(systems::Shelter::new(
                step,
                c.indoor_coupling_seconds.saturating_mul(1000),
                c.thermal_mass_reference,
            )),
            // What each body with sight could see (Amendment A-8).
            Box::new(sight::Sight::new(
                c.clock.step(c.sight_step_seconds.saturating_mul(1000)),
                c.sight_min_illumination,
            )),
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
            || fact_type == schema::FALL_HEIGHT
        {
            composition::compose_additive(current, changes)
        } else if fact_type == schema::SOLID
            || fact_type == schema::OPAQUE
            || fact_type == schema::ENCLOSED
            || fact_type == schema::PORTAL_OPEN
            || fact_type == schema::MOBILE
            || fact_type == schema::TRAVEL_BLOCKED
        {
            composition::compose_bool(current, changes)
        } else if fact_type == schema::TERRAIN_SPACING {
            composition::compose_bounded(current, changes, 1, schema::MAX_SIZE)
        } else if fact_type == schema::TRAVEL_SPEED {
            composition::compose_bounded(current, changes, 0, schema::MAX_SPEED)
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
            || fact_type == schema::PORTAL_FAR_SIDE
            || fact_type == schema::ACT_REFUSED
        {
            composition::compose_entity_ref(current, changes)
        } else if fact_type == schema::TRAVEL_TO
            || fact_type == schema::ACT_OPEN
            || fact_type == schema::ACT_CLOSE
        {
            composition::compose_intent(current, changes)
        } else if fact_type == schema::ACT_FACE {
            composition::compose_heading(current, changes)
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
            || fact_type == schema::PORTAL_FAR_SIDE
            || fact_type == schema::TRAVEL_TO
            || fact_type == schema::ACT_OPEN
            || fact_type == schema::ACT_CLOSE
            || fact_type == schema::ACT_REFUSED
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
            || fact_type == schema::IN_VIEW
        {
            for v in values {
                if !matches!(v, Value::Entity(_)) {
                    return Err(ValidationError::new(
                        "a relation set may hold only entity references",
                    ));
                }
            }
        }
        // A heightfield is a set of [column, row, height] samples; a grid has no negative
        // columns or rows, and one place holds one height (Amendment A-4).
        if fact_type == schema::TERRAIN_SAMPLE {
            let mut cells = std::collections::BTreeSet::new();
            for v in values {
                let Some([c, r, _]) = v.as_vec3() else {
                    return Err(ValidationError::new(
                        "a terrain sample is [column, row, height]",
                    ));
                };
                if c < 0 || r < 0 {
                    return Err(ValidationError::new("terrain columns and rows start at 0"));
                }
                if !cells.insert((c, r)) {
                    return Err(ValidationError::new(
                        "two terrain heights for one grid point",
                    ));
                }
            }
        }
        Ok(())
    }
}
