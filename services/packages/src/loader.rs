//! Turning a validated [`WorldPackage`] into a running world (Vol. IV Ch. 1 §1.3).
//!
//! The loader is the boundary the whole volume defends: data in, world out, no engine
//! defaults. It enforces the engine-version range (invariant 10), requires Physical Reality
//! (Vol. IV Ch. 2, invariant 2), configures each enabled domain from its package rules
//! (invariant 5), and seeds initial reality through the store — the world begins as a set of
//! committed facts, exactly where a later tick would leave it.
//!
//! It is also the layer where cross-domain worlds are assembled: `physical` and `living` are
//! both wired here from package data. The domains never reference each other
//! (Vol. III Ch. 12, invariant 1) — living finds an organism's region and temperature in the
//! store by their published ids, and the loader simply enables both and seeds their facts,
//! including the physical containment links that place organisms in regions.

use crate::model::{Flag, JobWork, MaterialProperty, WorldPackage};
use crate::version::Version;
use economy::{EconomyConfig, EconomyDomain};
use information::schema as info;
use information::{InformationConfig, InformationDomain};
use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::{CommittedView, System};
use kernel::tick::{run_tick_metered, SystemMeter, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use living::schema::{
    BODY_HEAT, DEPENDENCE, FATIGUE, FULL, HEALTH, KIND_ABOVE, KIND_ARISES, KIND_EASE, KIND_HARM,
    KIND_MET_BY, KIND_RISE, SIGHT_RANGE,
};
use living::{LivingConfig, LivingDomain};
use minds::schema as mind;
use minds::{MindRules, MindsConfig, MindsDomain};
use physical::schema::{
    ADJACENT_TO, BODY_SIZE, CONTAINED_IN, ELEVATION, ENCLOSED, EXPOSURE, HAS_PORTAL, HEADING,
    IN_REGION, LEADS_TO, MADE_OF, MATERIAL_CONDUCTIVITY, MATERIAL_DENSITY, MATERIAL_FLAMMABILITY,
    MATERIAL_HARDNESS, MATERIAL_THERMAL_CAPACITY, MATERIAL_TOXICITY, MOBILE, MOTION_END,
    MOTION_START, MOTION_TARGET, OPAQUE, PORTAL_DANGER_OVERRIDE, PORTAL_FAR_SIDE, PORTAL_OPEN,
    POSITION, SOLID, TEMPERATURE, TERRAIN_SAMPLE, TERRAIN_SPACING, TRAVEL_SPEED, TRAVEL_TO,
};
use physical::{PhysicalConfig, PhysicalDomain};
use resources::{ResourcesConfig, ResourcesDomain};
use society::{SocietyConfig, SocietyDomain};
use std::fmt;

/// Why a world package could not be loaded.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LoadError {
    /// The engine version is outside the package's declared range (invariant 10).
    EngineMismatch {
        /// The range the package requires, rendered.
        required: String,
        /// The actual engine version.
        engine: Version,
    },
    /// Physical Reality was not selected, but every world requires it (Vol. IV Ch. 2).
    PhysicalNotSelected,
    /// The living domain was selected but supplied no `[rules.living]` block. A missing
    /// rule is a validation error, never an engine default (Vol. IV Ch. 2).
    LivingRulesMissing,
    /// The information layer was selected but supplied no `[rules.information]` block.
    InformationRulesMissing,
    /// Minds were selected but the package supplied no `[rules.minds]` block.
    MindsRulesMissing,
    /// Society was selected but the package supplied no `[rules.society]` block.
    SocietyRulesMissing,
    /// Resources was selected but the package supplied no `[rules.resources]` block.
    ResourcesRulesMissing,
    /// A selected domain has no implementation wired into the loader yet.
    UnsupportedDomain(String),
    /// The package failed validation (Vol. IV Ch. 7 §7.1): every problem found, each naming its
    /// layer, subject, and rule. Nothing is seeded from an invalid package.
    Invalid(Vec<crate::validate::Problem>),
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoadError::EngineMismatch { required, engine } => write!(
                f,
                "engine {engine} does not satisfy required range {required}"
            ),
            LoadError::PhysicalNotSelected => {
                write!(f, "package does not select the mandatory `physical` domain")
            }
            LoadError::LivingRulesMissing => {
                write!(f, "`living` domain selected but no [rules.living] provided")
            }
            LoadError::InformationRulesMissing => write!(
                f,
                "`information` selected but no [rules.information] provided"
            ),
            LoadError::MindsRulesMissing => {
                write!(f, "`minds` selected but no [rules.minds] provided")
            }
            LoadError::SocietyRulesMissing => {
                write!(f, "`society` selected but no [rules.society] provided")
            }
            LoadError::ResourcesRulesMissing => {
                write!(f, "`resources` selected but no [rules.resources] provided")
            }
            LoadError::UnsupportedDomain(d) => {
                write!(f, "selected domain `{d}` is not implemented yet")
            }
            LoadError::Invalid(problems) => {
                write!(f, "package is invalid ({} problems)", problems.len())?;
                for p in problems {
                    write!(f, "\n  {p}")?;
                }
                Ok(())
            }
        }
    }
}

/// A world assembled from a package: seeded committed state plus its enabled domains and
/// their systems, ready to tick.
pub struct LoadedWorld {
    store: MemoryStore,
    domains: Vec<Box<dyn Domain>>,
    systems: Vec<Box<dyn System>>,
    meters: Vec<SystemMeter>,
}

impl fmt::Debug for LoadedWorld {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LoadedWorld")
            .field("facts", &self.store.len())
            .field("domains", &self.domains.len())
            .field("systems", &self.systems.len())
            .finish()
    }
}

impl LoadedWorld {
    /// The committed reality store, for read-only inspection.
    pub fn store(&self) -> &MemoryStore {
        &self.store
    }

    /// The systems those domains contribute to every tick, for read-only inspection.
    pub fn systems(&self) -> &[Box<dyn System>] {
        &self.systems
    }

    /// Advance the loaded world by one tick under `seed`, appending to `chronicle`
    /// (Vol. V Ch. 3 §3.1).
    pub fn tick(
        &mut self,
        tick: u64,
        seed: u64,
        chronicle: &mut Vec<ChronicleEntry>,
    ) -> Result<(), TickError> {
        let domain_refs: Vec<&dyn Domain> = self.domains.iter().map(|d| d.as_ref()).collect();
        self.meters.clear();
        run_tick_metered(
            &mut self.store,
            &domain_refs,
            &self.systems,
            tick,
            seed,
            chronicle,
            &mut self.meters,
        )
    }

    /// What each system cost in the last tick run (Amendment A-21).
    pub fn meters(&self) -> &[SystemMeter] {
        &self.meters
    }

    /// The store itself, for a harness that tunes what it remembers (Amendment A-21) — never to
    /// write reality, which only ticks do.
    pub fn store_mut(&mut self) -> &mut MemoryStore {
        &mut self.store
    }

    /// Attach a front-door system to the running world: a decider standing in for a player or a
    /// mind, which proposes intents — where to go, what to open, which way to face (Appendix A,
    /// Ruling 13). It runs beside the world's own systems under the same law: declared reads and
    /// writes, a unique id, and owners' authority over what it may write (Amendment A-5).
    pub fn attach(&mut self, system: Box<dyn System>) {
        self.systems.push(system);
    }

    /// The enabled domains, for read-only inspection.
    pub fn domains(&self) -> &[Box<dyn Domain>] {
        &self.domains
    }
}

/// The Living Systems configuration a package declares, exactly as [`load`] configures the
/// domain — or `None` if the package has no `[rules.living]`. Exposed, like
/// [`physical_config`], so a harness can run living alone against a loaded world's store.
pub fn living_config(package: &WorldPackage) -> Option<LivingConfig> {
    let rules = package.living_rules?;
    Some(LivingConfig {
        clock: SimClock::new(package.clock.tick_ms),
        metabolism_step_seconds: rules.metabolism_step_seconds,
        set_point_centi_c: rules.set_point_centi_c,
        warm_response_seconds: rules.warm_response_seconds,
        cold_response_seconds: rules.cold_response_seconds,
        tire_per_hour: rules.tire_per_hour,
        rest_per_hour: rules.rest_per_hour,
        hypothermia_below_centi_c: rules.hypothermia_below_centi_c,
        cold_harm_per_degree_hour: rules.cold_harm_per_degree_hour,
        safe_fall_cm: rules.safe_fall_cm,
        fall_harm_per_metre: rules.fall_harm_per_metre,
        heal_per_hour: rules.heal_per_hour,
        dependence_fade_per_day: rules.dependence_fade_per_day,
        hunger_per_hour: rules.hunger_per_hour,
        starving_above: rules.starving_above,
        starving_harm_per_hour: rules.starving_harm_per_hour,
        resident_sight_cm: package.residents_rules.map(|r| r.sight_cm),
        strength: rules.strength,
        burn_harm_per_hour: rules.burn_harm_per_hour,
        smoke_harm_per_hour: rules.smoke_harm_per_hour,
        chill: living::systems::Chill {
            wind_cm_s: rules.chill_wind_cm_s,
            wet: rules.wet_chill_percent,
        },
    })
}

/// Resources' configuration a package declares — or `None` without `[rules.resources]`.
pub fn resources_config(package: &WorldPackage) -> Option<ResourcesConfig> {
    let r = package.resources_rules?;
    Some(ResourcesConfig {
        clock: SimClock::new(package.clock.tick_ms),
        regrow_step_seconds: r.regrow_step_seconds,
        // Households' larders in the scene (Amendment A-26), if any household has a pantry.
        pantries: pantry_rules(package).map(|(meal_material, meal_size, _)| {
            resources::systems::PantryRules {
                meal_material: EntityId::from_raw(meal_material),
                meal_size,
            }
        }),
    })
}

/// Economy's configuration for a package: it has no tunable rules yet, only the world's clock
/// (Amendment A-17).
pub fn economy_config(package: &WorldPackage) -> EconomyConfig {
    EconomyConfig {
        clock: SimClock::new(package.clock.tick_ms),
    }
}

/// Society's configuration a package declares, exactly as [`load`] configures it — or `None` if
/// the package has no `[rules.society]`.
pub fn society_config(package: &WorldPackage) -> Option<SocietyConfig> {
    let r = package.society_rules?;
    Some(SocietyConfig {
        clock: SimClock::new(package.clock.tick_ms),
        courtship_step_seconds: r.courtship_step_seconds,
        bond_above: r.bond_above,
        part_below: r.part_below,
        // The city tier's rhythm (Amendment A-23), at Living's rates: the same person hungers and
        // tires at the same pace at every tier.
        rhythm: package.residents_rules.map(|rr| {
            let living = package.living_rules;
            society::residents::RhythmRules {
                day_seconds: package.clock.day_seconds as i64,
                sleep_from_seconds: rr.sleep_hours.0 * 3600,
                sleep_to_seconds: rr.sleep_hours.1 * 3600,
                work_from_seconds: rr.work_hours.0 * 3600,
                work_to_seconds: rr.work_hours.1 * 3600,
                hungry_above: rr.hungry_above,
                meal_nutrition: rr.meal_nutrition,
                meals_per_shift: rr.meals_per_shift,
                meal_seconds: rr.meal_minutes * 60,
                walk_speed_cm_s: rr.walk_speed_cm_s,
                hours_spread_seconds: rr.hours_spread_minutes * 60,
                hunger_per_hour: living.map_or(0, |l| l.hunger_per_hour),
                tire_per_hour: living.map_or(0, |l| l.tire_per_hour),
                rest_per_hour: living.map_or(0, |l| l.rest_per_hour),
            }
        }),
    })
}

/// Decision systems' configuration a package declares, exactly as [`load`] configures them —
/// or `None` if the package has no `[rules.minds]`.
pub fn minds_config(package: &WorldPackage) -> Option<MindsConfig> {
    let r = package.minds_rules?;
    Some(MindsConfig {
        clock: SimClock::new(package.clock.tick_ms),
        day_seconds: package.clock.day_seconds,
        rules: MindRules {
            think_step_seconds: r.think_step_seconds,
            cold_below_centi_c: r.cold_below_centi_c,
            trust_half_age_seconds: r.trust_half_age_seconds,
            hop_cost: r.hop_cost,
            routine_value: r.routine_value,
            switch_margin: r.switch_margin,
            tired_above: r.tired_above,
            rested_below: r.rested_below,
            need_above: r.need_above,
            need_weight: r.need_weight,
            hungry_above: r.hungry_above,
            work_value: r.work_value,
            tired_margin: r.tired_margin,
            sleep_from_seconds: r.sleep_hours.0 * 3600,
            sleep_to_seconds: r.sleep_hours.1 * 3600,
            bare_hand_hardness: package.physical_rules.bare_hand_hardness,
            repair_value: r.repair_value,
            repair_below: r.repair_below,
        },
        // A resident crossing into the scene keeps their pace and their hours (Amendment A-24).
        residents: package.residents_rules.map(|rr| minds::seam::ResidentMind {
            walk_speed_cm_s: rr.walk_speed_cm_s,
            work_from_seconds: rr.work_hours.0 * 3600,
            work_to_seconds: rr.work_hours.1 * 3600,
            hours_spread_seconds: rr.hours_spread_minutes * 60,
        }),
    })
}

/// The information layer's configuration a package declares, exactly as [`load`] configures it
/// — or `None` if the package has no `[rules.information]`.
pub fn information_config(package: &WorldPackage) -> Option<InformationConfig> {
    let rules = package.information_rules?;
    Some(InformationConfig {
        clock: SimClock::new(package.clock.tick_ms),
        perception_step_seconds: rules.perception_step_seconds,
        warmth_resolution_centi_c: rules.warmth_resolution_centi_c,
        need_resolution: rules.need_resolution,
        affection_step_seconds: rules.affection_step_seconds,
        affection_per_hour: rules.affection_per_hour,
        affection_fade_per_day: rules.affection_fade_per_day,
        residents: package.residents_rules.is_some(),
        meal_material: pantry_rules(package).map(|(m, _, _)| EntityId::from_raw(m)),
    })
}

/// The Physical Reality configuration a package declares: its clock and every physical rule,
/// exactly as [`load`] configures the domain. Exposed so a harness can run one physical system
/// in isolation against a loaded world's store.
pub fn physical_config(package: &WorldPackage) -> PhysicalConfig {
    let r = &package.physical_rules;
    PhysicalConfig {
        clock: SimClock::new(package.clock.tick_ms),
        day_length_seconds: package.clock.day_seconds,
        environment_step_seconds: r.environment_step_seconds,
        diurnal_amplitude_centi_c: r.diurnal_amplitude_centi_c,
        diurnal_coldest_seconds: r.diurnal_coldest_seconds,
        diurnal_warmest_seconds: r.diurnal_warmest_seconds,
        temperature_variability_centi_c: r.temperature_variability_centi_c,
        weather_persistence_seconds: r.weather_persistence_seconds,
        illumination_peak: r.illumination_peak,
        humidity_baseline: r.humidity_baseline,
        humidity_variability: r.humidity_variability,
        pressure_sea_level: r.pressure_sea_level,
        pressure_elevation_factor: r.pressure_elevation_factor,
        pressure_variability: r.pressure_variability,
        wind_per_decapascal: r.wind_per_decapascal,
        fronts: physical::systems::Fronts {
            mixing_ms: r.climate_mixing_seconds.saturating_mul(1000),
            distance_cm: r.front_distance_cm,
        },
        fall_danger_per_meter: r.fall_danger_per_meter,
        thermal_mass_reference: r.thermal_mass_reference,
        gravity_cm_s2: r.gravity_cm_s2,
        step_height_cm: r.step_height_cm,
        max_slope_percent: r.max_slope_percent,
        nav_cell_cm: r.nav_cell_cm,
        reach_cm: r.reach_cm,
        indoor_coupling_seconds: r.indoor_coupling_seconds,
        sight_step_seconds: r.sight_step_seconds,
        sight_min_illumination: r.sight_min_illumination,
        carry_limit_kg: r.carry_limit_kg,
        resident_body: package.residents_rules.map(|rr| rr.body),
        pantry_body: pantry_rules(package).map(|(_, _, body)| body),
        strike: physical::act::StrikeRules {
            hand_percent: r.hand_strike_percent,
            tool_percent_per_kg: r.tool_strike_percent_per_kg,
            scale: r.strike_scale,
            bare_hand_hardness: r.bare_hand_hardness,
            tool_wear: r.tool_wear,
        },
        pit_half_width_cm: r.pit_half_width_cm,
        wreckage_height_cm: r.wreckage_height_cm,
        mend_scale: r.mend_scale,
        fire: physical::fire::FireRules {
            step_seconds: r.fire_step_seconds,
            grow_per_hour: r.fire_grow_per_hour,
            burn_per_hour: r.burn_per_hour,
            reach_cm: r.fire_reach_cm,
            ignition_heat: r.ignition_heat,
            ignite_intensity: r.ignite_intensity,
            douse_per_hour: r.douse_per_hour,
            wall_thickness_cm: r.wall_thickness_cm,
            warmth_per_hour: r.fire_warmth_per_hour,
            smoke_per_hour: r.smoke_per_hour,
            smoke_clear_per_hour: r.smoke_clear_per_hour,
            fan_wind_cm_s: r.fire_fan_wind_cm_s,
            dry_by_heat_per_hour: r.dry_by_heat_per_hour,
        },
        wind: physical::wind::WindRules {
            force_divisor: r.wind_force_divisor,
            damage_scale: r.wind_damage_scale,
            hold_per_kg: r.wind_hold_per_kg,
            carry_cm_per_hour: r.wind_carry_cm_per_hour,
        },
        gale: physical::systems::Gale {
            below: r.gale_below,
            per_decapascal: r.gale_per_decapascal,
        },
        sky: physical::systems::SkyRules {
            cloud_above: r.cloud_above,
            rain_above: r.rain_above,
            cloud_shade: r.cloud_shade,
            cloud_damp: r.cloud_damp,
            rain_per_hour: r.rain_per_hour,
            snow_below_centi_c: r.snow_below_centi_c,
            fog_above: r.fog_above,
            fog_wind_below_cm_s: r.fog_wind_below_cm_s,
            humidity_per_degree: r.humidity_per_degree,
        },
        wet: physical::wet::WetRules {
            soak_per_mm: r.soak_per_mm,
            ground_drying_per_hour: r.ground_drying_per_hour,
            snow_melt_mm_per_degree_hour: r.snow_melt_mm_per_degree_hour,
            wetting_per_hour: r.wetting_per_hour,
            drying_per_hour: r.drying_per_hour,
            hardest_rain: r.rain_per_hour,
        },
        water: physical::water::WaterRules {
            tide_amplitude_cm: r.tide_amplitude_cm,
            tidal_period_ms: r.tidal_period_seconds.saturating_mul(1000),
            ice_mm_per_degree_hour: r.ice_mm_per_degree_hour,
            wade_depth_cm: r.wade_depth_cm,
            ice_bears_mm: r.ice_bears_mm,
            snow_melt_mm_per_degree_hour: r.snow_melt_mm_per_degree_hour,
        },
        air: physical::air::AirRules {
            opening_speed_cm_s: r.opening_air_speed_cm_s,
            draught: r.draught_percent,
            open_air_height_cm: r.open_air_height_cm,
        },
        calendar: physical::calendar::Calendar {
            day_ms: package.clock.day_seconds.saturating_mul(1000),
            year_days: r.year_days,
            start_day: r.start_day,
            tilt: r.axial_tilt_centi_deg,
            latitude: r.latitude_centi_deg,
            seasonal_amplitude_centi_c: r.seasonal_amplitude_centi_c,
            seasonal_lag_days: r.seasonal_lag_days,
            lunar_month_ms: r.lunar_month_seconds.saturating_mul(1000),
            moon_phase_at_start: r.moon_phase_at_start,
            moonlight_peak: r.moonlight_peak,
        },
    }
}

/// What households' pantries need (Amendment A-26) — the meal material, a meal's size, and a
/// pantry's — if any household has one and the rules declare all three.
fn pantry_rules(package: &WorldPackage) -> Option<(u64, [i64; 3], [i64; 3])> {
    let rr = package.residents_rules?;
    if package.pantries.is_empty() {
        return None;
    }
    Some((rr.meal_material?, rr.meal_size?, rr.pantry_body?))
}

/// The engine version this build presents to packages (from the crate version).
pub fn engine_version() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("crate version is valid semver")
}

/// Load a package into a runnable world, enforcing the package contract (Vol. IV Ch. 1 §1.3).
///
/// `engine` is the version checked against the package's declared range; most callers pass
/// [`engine_version`].
pub fn load(package: &WorldPackage, engine: Version) -> Result<LoadedWorld, LoadError> {
    // 1. Enforce the engine-version range — not advisory (Vol. IV Ch. 1, invariant 10).
    if !package.manifest.engine.accepts(engine) {
        return Err(LoadError::EngineMismatch {
            required: format!(
                ">={}, <{}",
                package.manifest.engine.min, package.manifest.engine.max
            ),
            engine,
        });
    }

    // 1b. Validate the package before anything is seeded (Vol. IV Ch. 7 §7.1): references
    //     resolve, nothing is declared twice, containment is a hierarchy.
    let problems = crate::validate::validate(package);
    if !problems.is_empty() {
        return Err(LoadError::Invalid(problems));
    }

    // 2. Domain selection: Physical Reality is mandatory; unknown domains are refused rather
    //    than silently ignored (Vol. IV Ch. 2, invariants 2 & 3).
    let mut has_physical = false;
    let mut has_living = false;
    let mut has_information = false;
    let mut has_minds = false;
    let mut has_society = false;
    let mut has_resources = false;
    let mut has_economy = false;
    for d in &package.manifest.domains {
        match d.as_str() {
            "physical" => has_physical = true,
            "living" => has_living = true,
            "information" => has_information = true,
            "minds" => has_minds = true,
            "society" => has_society = true,
            "resources" => has_resources = true,
            "economy" => has_economy = true,
            other => return Err(LoadError::UnsupportedDomain(other.to_string())),
        }
    }
    if !has_physical {
        return Err(LoadError::PhysicalNotSelected);
    }

    let mut domains: Vec<Box<dyn Domain>> = Vec::new();
    let mut systems: Vec<Box<dyn System>> = Vec::new();
    let mut store = MemoryStore::new();

    // 3a. Physical Reality: configured from package rules (invariant 5). The domain needs no
    //     region list — its systems discover regions from the seeded temperature facts below
    //     (Vol. V Ch. 2 §2.1, clause 5).
    // Both domains step in simulated time on the world's clock (Vol. II Ch. 2, Amendment A-1).
    let config = physical_config(package);
    let physical = PhysicalDomain::new(config);
    systems.extend(physical.systems());
    domains.push(Box::new(physical));

    // Seed regions' elevation when specified; their temperature, once what they are built of
    // and how open they are is seeded (below).
    for region in &package.regions {
        if let Some(elev) = region.elevation {
            store.seed(
                FactKey::new(EntityId::from_raw(region.id), ELEVATION),
                seeded(Value::Int(elev)),
            );
        }
    }

    // Seed containment (a Physical fact): organisms within their regions, plus any extra
    // links the package declares (e.g. a region within a continent) — Vol. III Ch. 1 §1.8.
    for o in &package.organisms {
        store.seed(
            FactKey::new(EntityId::from_raw(o.id), CONTAINED_IN),
            seeded(Value::Entity(EntityId::from_raw(o.region_id))),
        );
    }
    // Places that only hold other places (a continent, a town): their own containment link, if
    // they lie within something.
    for &(place, within) in &package.places {
        if let Some(parent) = within {
            store.seed(
                FactKey::new(EntityId::from_raw(place), CONTAINED_IN),
                seeded(Value::Entity(EntityId::from_raw(parent))),
            );
        }
    }
    for c in &package.containment {
        store.seed(
            FactKey::new(EntityId::from_raw(c.child_id), CONTAINED_IN),
            seeded(Value::Entity(EntityId::from_raw(c.parent_id))),
        );
    }

    // Seed local positions (Physical facts): where each entity's base sits within its
    // container — one three-component fact each (Amendment A-3).
    for p in &package.positions {
        store.seed(
            FactKey::new(EntityId::from_raw(p.entity_id), POSITION),
            seeded(Value::Vec3([p.x, p.y, p.z.unwrap_or(0)])),
        );
    }

    // Seed bodies, facings, and motion under way (Physical facts, Amendment A-3). Motion is a
    // segment from the body's seeded position, leaving at tick 0 and arriving after the given
    // simulated time, rounded up to a whole tick.
    for b in &package.bodies {
        store.seed(
            FactKey::new(EntityId::from_raw(b.entity_id), BODY_SIZE),
            seeded(Value::Vec3([b.half_width, b.half_depth, b.height])),
        );
    }
    for f in &package.facing {
        store.seed(
            FactKey::new(EntityId::from_raw(f.entity_id), HEADING),
            seeded(Value::Int(f.heading.rem_euclid(36_000))),
        );
    }
    for m in &package.motion {
        let e = EntityId::from_raw(m.entity_id);
        let tick_ms = package.clock.tick_ms;
        let ticks = m.seconds.saturating_mul(1000).div_ceil(tick_ms);
        store.seed(
            FactKey::new(e, MOTION_TARGET),
            seeded(Value::Vec3(m.target)),
        );
        store.seed(FactKey::new(e, MOTION_START), seeded(Value::Int(0)));
        store.seed(
            FactKey::new(e, MOTION_END),
            seeded(Value::Int(ticks as i64)),
        );
    }

    // Seed portals (Physical facts): each portal is an entity located in its host region
    // (contained_in + position) that leads to its destination; the host region hosts it
    // (has_portal, cardinality-many). Connectivity, distinct from adjacency (§1.5).
    for portal in &package.portals {
        let pid = EntityId::from_raw(portal.portal_id);
        let host = EntityId::from_raw(portal.host_region);
        store.seed(FactKey::new(pid, CONTAINED_IN), seeded(Value::Entity(host)));
        store.seed(
            FactKey::new(pid, POSITION),
            seeded(Value::Vec3([portal.x, portal.y, portal.z.unwrap_or(0)])),
        );
        store.seed(
            FactKey::new(pid, LEADS_TO),
            seeded(Value::Entity(EntityId::from_raw(portal.dest_region))),
        );
        store.seed(FactKey::new(host, HAS_PORTAL), seeded(Value::Entity(pid)));
    }

    // Seed world-pinned portal danger (Physical facts). Portals not listed have their danger
    // derived from height (and later weather) by the danger system (§1.11).
    for d in &package.portal_danger {
        store.seed(
            FactKey::new(EntityId::from_raw(d.portal_id), PORTAL_DANGER_OVERRIDE),
            seeded(Value::Int(d.danger)),
        );
    }

    // Seed per-region exposure to the sky (a Physical fact). Regions absent here are fully
    // exposed; the weather systems attenuate their effect by this (Vol. III Ch. 1 §1.6).
    for x in &package.exposure {
        store.seed(
            FactKey::new(EntityId::from_raw(x.region_id), EXPOSURE),
            seeded(Value::Int(x.exposure)),
        );
    }

    // Seed adjacency (a cardinality-many Physical fact) in both directions per edge, so the
    // topology is symmetric -- a region borders its neighbour and vice versa (§1.5).
    for e in &package.adjacency {
        store.seed(
            FactKey::new(EntityId::from_raw(e.a), ADJACENT_TO),
            seeded(Value::Entity(EntityId::from_raw(e.b))),
        );
        store.seed(
            FactKey::new(EntityId::from_raw(e.b), ADJACENT_TO),
            seeded(Value::Entity(EntityId::from_raw(e.a))),
        );
    }

    // Seed materials (Physical facts, Vol. III Ch. 1 §1.9): each material entity carries only
    // the property facts the package declares — properties over names, no engine default.
    for m in &package.materials {
        let material = EntityId::from_raw(m.id);
        for (property, value) in &m.properties {
            store.seed(
                FactKey::new(material, material_fact(*property)),
                seeded(Value::Int(*value)),
            );
        }
    }

    // Seed composition (a cardinality-many Physical fact): each object is linked to every
    // material it is made of (§1.9, composites are the rule).
    for link in &package.made_of {
        store.seed(
            FactKey::new(EntityId::from_raw(link.object_id), MADE_OF),
            seeded(Value::Entity(EntityId::from_raw(link.material_id))),
        );
    }

    // Seed constraint flags, linked opening faces, terrain, and travel intents (Physical facts,
    // Amendment A-4; Ruling 13).
    for spec in &package.flags {
        let e = EntityId::from_raw(spec.entity_id);
        for flag in &spec.flags {
            let (fact, value) = match flag {
                Flag::Solid => (SOLID, true),
                Flag::Opaque => (OPAQUE, true),
                Flag::Enclosed => (ENCLOSED, true),
                Flag::Mobile => (MOBILE, true),
                Flag::Closed => (PORTAL_OPEN, false),
                Flag::Paved => (physical::schema::PAVED, true),
            };
            store.seed(FactKey::new(e, fact), seeded(Value::Bool(value)));
        }
    }
    for &(a, b) in &package.portal_pairs {
        let (a, b) = (EntityId::from_raw(a), EntityId::from_raw(b));
        store.seed(FactKey::new(a, PORTAL_FAR_SIDE), seeded(Value::Entity(b)));
        store.seed(FactKey::new(b, PORTAL_FAR_SIDE), seeded(Value::Entity(a)));
    }
    for t in &package.terrain {
        let region = EntityId::from_raw(t.region_id);
        store.seed(
            FactKey::new(region, TERRAIN_SPACING),
            seeded(Value::Int(t.spacing)),
        );
        for (i, h) in t.heights.iter().enumerate() {
            let (column, row) = ((i % t.columns) as i64, (i / t.columns) as i64);
            store.seed(
                FactKey::new(region, TERRAIN_SAMPLE),
                seeded(Value::Vec3([column, row, *h])),
            );
        }
    }
    for t in &package.travel {
        let e = EntityId::from_raw(t.entity_id);
        store.seed(
            FactKey::new(e, TRAVEL_TO),
            seeded(Value::Entity(EntityId::from_raw(t.target))),
        );
        store.seed(
            FactKey::new(e, TRAVEL_SPEED),
            seeded(Value::Int(t.speed_cm_s)),
        );
    }

    // Seed overlapping region memberships (a cardinality-many Physical fact, Vol. III Ch. 1
    // §1.7): each location is linked to every region it lies in beyond its container. The
    // region entities need nothing else seeded -- a classification region is the places that
    // name it, and with no temperature fact it is (correctly) not a place the weather runs on.
    for m in &package.in_region {
        store.seed(
            FactKey::new(EntityId::from_raw(m.location_id), IN_REGION),
            seeded(Value::Entity(EntityId::from_raw(m.region_id))),
        );
    }

    // Places one can be in, rather than things (Amendment A-30): every climate, every declared
    // place, every walled room, and everywhere an opening is in or leads to.
    let mut places: std::collections::BTreeSet<u64> = std::collections::BTreeSet::new();
    places.extend(package.regions.iter().map(|r| r.id));
    places.extend(package.places.iter().map(|(p, _)| *p));
    for spec in &package.flags {
        if spec.flags.contains(&Flag::Enclosed) {
            places.insert(spec.entity_id);
        }
    }
    for portal in &package.portals {
        places.insert(portal.host_region);
        places.insert(portal.dest_region);
    }
    for place in places {
        store.seed(
            FactKey::new(EntityId::from_raw(place), physical::schema::PLACE),
            seeded(Value::Bool(true)),
        );
    }

    // Storms the world declares (Amendment A-29): from when to when, in seconds, and how hard.
    for &(region, from_hour, hours, wind) in &package.storms {
        let (from, to) = (from_hour * 3600, (from_hour + hours) * 3600);
        store.seed(
            FactKey::new(EntityId::from_raw(region), physical::schema::STORM),
            seeded(Value::Vec3([from, to, wind])),
        );
    }

    // What each place's ground is made of (Amendment A-42).
    for &(place, material) in &package.ground {
        store.seed(
            FactKey::new(EntityId::from_raw(place), physical::schema::GROUND),
            seeded(Value::Entity(EntityId::from_raw(material))),
        );
    }

    // How fast each workplace wears in use (Amendment A-46).
    for &(workplace, per_hour) in &package.wear {
        store.seed(
            FactKey::new(
                EntityId::from_raw(workplace),
                physical::schema::WEAR_PER_HOUR,
            ),
            seeded(Value::Int(per_hour)),
        );
    }

    // How wet each climate's ground begins (Amendment A-41).
    for &(climate, wet) in &package.ground_water {
        store.seed(
            FactKey::new(EntityId::from_raw(climate), physical::schema::GROUND_WATER),
            seeded(Value::Int(wet)),
        );
    }

    // Climates' normal humidity, where it differs from the world's (Amendment A-35).
    for &(climate, normal) in &package.humidities {
        store.seed(
            FactKey::new(
                EntityId::from_raw(climate),
                physical::schema::HUMIDITY_NORMAL,
            ),
            seeded(Value::Int(normal)),
        );
    }

    // The world's water (Amendment A-39): what each body of water is, standing at rest; where it
    // flows on; what it floods.
    for w in &package.waters {
        let id = EntityId::from_raw(w.id);
        for (fact, value) in [
            (physical::schema::WATER_KIND, w.kind),
            (physical::schema::WATER_NORMAL, w.depth_cm),
            (physical::schema::WATER_BANK, w.bank_cm),
            (physical::schema::CATCHMENT, w.catchment_m2),
            (physical::schema::WATER_DRAIN_SECONDS, w.drain_seconds),
            (physical::schema::WATER_LEVEL, w.depth_cm),
        ] {
            store.seed(FactKey::new(id, fact), seeded(Value::Int(value)));
        }
    }
    for &(from, to) in &package.drains {
        store.seed(
            FactKey::new(EntityId::from_raw(from), physical::schema::DRAINS_TO),
            seeded(Value::Entity(EntityId::from_raw(to))),
        );
    }
    for (water, places) in &package.floods {
        for &p in places {
            store.seed(
                FactKey::new(EntityId::from_raw(*water), physical::schema::FLOODS),
                seeded(Value::Entity(EntityId::from_raw(p))),
            );
        }
    }

    // Latitudes, where a place's differs from the world's (Amendment A-34).
    for &(place, latitude) in &package.latitudes {
        store.seed(
            FactKey::new(EntityId::from_raw(place), physical::schema::LATITUDE),
            seeded(Value::Int(latitude)),
        );
    }

    // Fires burning at the start (Amendment A-28).
    for &(thing, fierce, kept) in &package.fires {
        let thing = EntityId::from_raw(thing);
        store.seed(
            FactKey::new(thing, physical::schema::BURNING),
            seeded(Value::Int(fierce)),
        );
        if kept {
            store.seed(
                FactKey::new(thing, physical::schema::KEPT_FIRE),
                seeded(Value::Bool(true)),
            );
        }
    }

    // Regions' temperature (Amendment A-35): a climate open to the sky is declared by the coldest
    // of its day, and begins as warm above that as the day's swing stands at the first moment —
    // as exposed and as heavy as it is. A walled room begins at what it declares.
    let physical = physical_config(package);
    for region in &package.regions {
        let id = EntityId::from_raw(region.id);
        let share = if physical::climate::is_sheltered(&store, id) {
            0
        } else {
            physical::systems::day_share(
                &store,
                id,
                &physical.day_curve(),
                physical.thermal_mass_reference,
                physical.clock.ms_at(0),
            )
        };
        store.seed(
            FactKey::new(id, TEMPERATURE),
            seeded(Value::Int(region.temperature_centi_c + share)),
        );
    }

    // The world begins with its light (Amendment A-16): each region lit as the sun stands at the
    // first moment, so a world that opens at midnight opens dark, and nobody sees by a light
    // that is not there.
    for (region, light) in physical::systems::light_by_region(
        &store,
        physical.clock.ms_at(0),
        &physical.calendar,
        physical.illumination_peak,
        physical.sky.cloud_shade,
    ) {
        store.seed(
            FactKey::new(region, physical::schema::ILLUMINATION),
            seeded(Value::Int(light)),
        );
    }

    // 3b. Living Systems (optional): configured from package rules. Living reads organism
    //     containment and region temperature by id — no wiring between domains is needed.
    if has_living {
        let living =
            LivingDomain::new(living_config(package).ok_or(LoadError::LivingRulesMissing)?);
        systems.extend(living.systems());
        domains.push(Box::new(living));
        let rules = package.living_rules.ok_or(LoadError::LivingRulesMissing)?;
        for o in &package.organisms {
            let organism = EntityId::from_raw(o.id);
            store.seed(
                FactKey::new(organism, BODY_HEAT),
                seeded(Value::Int(o.body_heat_centi_c)),
            );
            // Every organism begins in full health, and rested unless the package says it
            // starts tired (Amendment A-10).
            let tired = package
                .fatigue
                .iter()
                .find(|(who, _)| *who == o.id)
                .map_or(0, |(_, f)| *f);
            // Hunger and fatigue are levels (Amendment A-20): each begins moving at the world's
            // rate — awake, so tiring.
            let level = |v: i64, rate: i64| kernel::level::Level::new(v, rate, 0).to_value();
            store.seed(
                FactKey::new(organism, FATIGUE),
                seeded(level(tired, rules.tire_per_hour)),
            );
            store.seed(FactKey::new(organism, HEALTH), seeded(Value::Int(FULL)));
            let hungry = package
                .hunger
                .iter()
                .find(|(who, _)| *who == o.id)
                .map_or(0, |(_, h)| *h);
            store.seed(
                FactKey::new(organism, living::schema::HUNGER),
                seeded(level(hungry, rules.hunger_per_hour)),
            );
        }
        // The kinds of need that can arise, each an entity carrying its rules (Amendment A-11).
        for k in &package.need_kinds {
            let kind = EntityId::from_raw(k.id);
            for (fact, v) in [
                (KIND_ARISES, k.arises),
                (KIND_MET_BY, k.met_by),
                (KIND_RISE, k.rise),
                (KIND_EASE, k.ease),
                (KIND_HARM, k.harm),
                (KIND_ABOVE, k.above),
            ] {
                store.seed(FactKey::new(kind, fact), seeded(Value::Int(v)));
            }
        }
        // Dependences the world's people start with (Amendment A-13).
        for &(organism, material, level) in &package.dependence {
            store.seed(
                FactKey::pair(
                    EntityId::from_raw(organism),
                    DEPENDENCE,
                    EntityId::from_raw(material),
                ),
                seeded(Value::Int(level)),
            );
        }
        // Strength, a capability (Amendment A-27): declared, or the world's default for every
        // organism.
        for o in &package.organisms {
            let declared = package
                .strength
                .iter()
                .find(|(who, _)| *who == o.id)
                .map(|(_, s)| *s);
            if let Some(strength) = declared.or(rules.strength) {
                store.seed(
                    FactKey::new(EntityId::from_raw(o.id), living::schema::STRENGTH),
                    seeded(Value::Int(strength)),
                );
            }
        }
        // Sensory capability is Living's (Amendment A-8).
        for &(organism, range) in &package.senses {
            store.seed(
                FactKey::new(EntityId::from_raw(organism), SIGHT_RANGE),
                seeded(Value::Int(range)),
            );
        }
    }

    // Deposits are seeded before anyone's starting knowledge, so a mind that knows a tree knows
    // what it bears and how much (Amendment A-16).
    if has_resources {
        // What a pit dug in each ground holds (Amendment A-42).
        for &(material, units, size, work) in &package.quarries {
            let material = EntityId::from_raw(material);
            for (fact, v) in [
                (resources::schema::QUARRY_RESERVE, Value::Int(units)),
                (resources::schema::QUARRY_SIZE, Value::Vec3(size)),
                (resources::schema::QUARRY_DIFFICULTY, Value::Int(work)),
            ] {
                store.seed(FactKey::new(material, fact), seeded(v));
            }
        }
        for d in &package.deposits {
            let deposit = EntityId::from_raw(d.id);
            let unit = resources::schema::UNIT;
            for (fact, v) in [
                (resources::schema::STOCK, Value::Int(d.stock * unit)),
                (
                    resources::schema::YIELD_MADE_OF,
                    Value::Entity(EntityId::from_raw(d.made_of)),
                ),
                (resources::schema::YIELD_SIZE, Value::Vec3(d.size)),
                (resources::schema::YIELD_PER_DAY, Value::Int(d.per_day)),
                (resources::schema::YIELD_CAP, Value::Int(d.cap)),
            ] {
                store.seed(FactKey::new(deposit, fact), seeded(v));
            }
            // How it renews, how poor it is, how hard it is to work, and how it grows (A-41).
            for (fact, v) in [
                (resources::schema::RENEWAL, d.renewal.map(Value::Int)),
                (resources::schema::FLOOR, d.floor.map(Value::Int)),
                (resources::schema::GRADE, d.grade.map(Value::Int)),
                (resources::schema::DIFFICULTY, d.difficulty.map(Value::Int)),
                (resources::schema::GROWS, d.grows.map(Value::Vec3)),
            ] {
                if let Some(v) = v {
                    store.seed(FactKey::new(deposit, fact), seeded(v));
                }
            }
        }
    }

    // Recipes too, so a mind told of a recipe knows what it needs, makes, and where
    // (Amendment A-18); and owners, so a mind told of a thing knows whose it is (Amendment A-19).
    if has_economy {
        for &(thing, owner) in &package.owners {
            store.seed(
                FactKey::new(EntityId::from_raw(thing), economy::schema::OWNER),
                seeded(Value::Entity(EntityId::from_raw(owner))),
            );
        }
        for r in &package.recipes {
            let recipe = EntityId::from_raw(r.id);
            for &(material, count) in &r.needs {
                store.seed(
                    FactKey::pair(
                        recipe,
                        economy::schema::RECIPE_NEEDS,
                        EntityId::from_raw(material),
                    ),
                    seeded(Value::Int(count)),
                );
            }
            for (fact, v) in [
                (
                    economy::schema::RECIPE_MAKES,
                    Value::Entity(EntityId::from_raw(r.makes)),
                ),
                (economy::schema::RECIPE_SIZE, Value::Vec3(r.size)),
                (
                    economy::schema::RECIPE_AT,
                    Value::Entity(EntityId::from_raw(r.at)),
                ),
                (economy::schema::RECIPE_TAKES, Value::Int(r.takes_seconds)),
            ] {
                store.seed(FactKey::new(recipe, fact), seeded(v));
            }
            // What else the making needs, and how many it makes (Amendment A-43).
            for (fact, v) in [
                (
                    economy::schema::RECIPE_HEAT,
                    r.heat.then_some(Value::Bool(true)),
                ),
                (
                    economy::schema::RECIPE_POWER,
                    r.power.map(|w| Value::Entity(EntityId::from_raw(w))),
                ),
                (economy::schema::RECIPE_WIND, r.wind.map(Value::Int)),
                (economy::schema::RECIPE_TOOL, r.tool.map(Value::Int)),
                (economy::schema::RECIPE_COUNT, r.count.map(Value::Int)),
            ] {
                if let Some(v) = v {
                    store.seed(FactKey::new(recipe, fact), seeded(v));
                }
            }
        }
    }

    // 3c. The information layer (optional; Amendment A-8): what each mind knows at the start.
    if has_information {
        let information = InformationDomain::new(
            information_config(package).ok_or(LoadError::InformationRulesMissing)?,
        );
        systems.extend(information.systems());
        domains.push(Box::new(information));
        for (mind, things) in &package.knows {
            for (key, value) in starting_knowledge(&store, EntityId::from_raw(*mind), things) {
                store.seed(key, known(value));
            }
        }
    }

    // 3d. Decision systems (optional; Amendment A-9): who has a mind, how fast it walks, and the
    //     routines the world gives it.
    if has_minds {
        let minds = MindsDomain::new(minds_config(package).ok_or(LoadError::MindsRulesMissing)?);
        systems.extend(minds.systems());
        domains.push(Box::new(minds));
        for &(mind, speed) in &package.minds {
            store.seed(
                FactKey::new(EntityId::from_raw(mind), mind::WALK_SPEED),
                seeded(Value::Int(speed)),
            );
        }
        // Wants (Amendment A-19): curiosity, likes, and a home.
        for &(who, worth) in &package.curiosity {
            store.seed(
                FactKey::new(EntityId::from_raw(who), mind::CURIOSITY),
                seeded(Value::Int(worth)),
            );
        }
        for &(who, material, worth) in &package.likes {
            store.seed(
                FactKey::pair(
                    EntityId::from_raw(who),
                    mind::LIKES,
                    EntityId::from_raw(material),
                ),
                seeded(Value::Int(worth)),
            );
        }
        for &(who, place) in &package.home {
            store.seed(
                FactKey::new(EntityId::from_raw(who), mind::HOME),
                seeded(Value::Entity(EntityId::from_raw(place))),
            );
        }
        for &(who, [a, b, c]) in &package.temperament {
            store.seed(
                FactKey::new(EntityId::from_raw(who), mind::TEMPERAMENT),
                seeded(Value::Vec3([a, b, c])),
            );
        }
        for &(who, from, to, target) in &package.routines {
            store.seed(
                FactKey::pair(
                    EntityId::from_raw(who),
                    mind::ROUTINE,
                    EntityId::from_raw(target),
                ),
                seeded(Value::Vec3([from * 3600, to * 3600, 0])),
            );
        }
    }

    // 3f. Resources (optional; Amendment A-15): deposits that regrow and yield.
    if has_resources {
        let resources = ResourcesDomain::new(
            resources_config(package).ok_or(LoadError::ResourcesRulesMissing)?,
        );
        systems.extend(resources.systems());
        domains.push(Box::new(resources));
    }

    // 3g. Economy (optional; Amendment A-17): making what the recipes seeded above describe.
    if has_economy {
        let economy = EconomyDomain::new(economy_config(package));
        systems.extend(economy.systems());
        domains.push(Box::new(economy));
    }

    // 3e. Society (optional; Amendment A-11): bonds between persons.
    if has_society {
        let society =
            SocietyDomain::new(society_config(package).ok_or(LoadError::SocietyRulesMissing)?);
        systems.extend(society.systems());
        domains.push(Box::new(society));
        // Residents at the city tier and their larders (Amendment A-23). Each first attends to
        // its day within the first hour, staggered by id, so a city does not all wake at once.
        let rr = society_config(package).and_then(|c| c.rhythm);
        let hour = (3_600_000 / package.clock.tick_ms.max(1)).max(1);
        for r in &package.residents {
            let who = EntityId::from_raw(r.id);
            let home = EntityId::from_raw(r.home);
            let level = |v: i64, rate: i64| kernel::level::Level::new(v, rate, 0).to_value();
            let mut facts = vec![
                (society::schema::RESIDENT_HOME, Value::Entity(home)),
                (society::schema::RESIDENT_AT, Value::Entity(home)),
                (
                    society::schema::RESIDENT_DOING,
                    Value::Int(society::schema::AT_HOME),
                ),
                (
                    society::schema::RESIDENT_UNTIL,
                    Value::Int(1 + (r.id % hour) as i64),
                ),
                (
                    society::schema::RESIDENT_HUNGER,
                    level(r.hunger, rr.map_or(0, |x| x.hunger_per_hour)),
                ),
                (
                    society::schema::RESIDENT_FATIGUE,
                    level(0, rr.map_or(0, |x| x.tire_per_hour)),
                ),
            ];
            if let Some(work) = r.work {
                facts.push((
                    society::schema::RESIDENT_WORK,
                    Value::Entity(EntityId::from_raw(work)),
                ));
            }
            for (fact, v) in facts {
                store.seed(FactKey::new(who, fact), seeded(v));
            }
        }
        for &(home, meals) in &package.larders {
            store.seed(
                FactKey::new(EntityId::from_raw(home), society::schema::LARDER),
                seeded(Value::Int(meals)),
            );
        }
        // Households' pantries (Amendment A-26): which entity each larder becomes in the scene,
        // owned by the household — its home.
        for &(home, pantry) in &package.pantries {
            store.seed(
                FactKey::new(EntityId::from_raw(home), society::schema::PANTRY),
                seeded(Value::Entity(EntityId::from_raw(pantry))),
            );
            if has_economy {
                store.seed(
                    FactKey::new(EntityId::from_raw(pantry), economy::schema::OWNER),
                    seeded(Value::Entity(EntityId::from_raw(home))),
                );
            }
        }
        // Places' rungs on the residency ladder (Amendment A-24).
        for &(place, rung) in &package.residency {
            store.seed(
                FactKey::new(EntityId::from_raw(place), society::schema::RESIDENCY),
                seeded(Value::Int(rung)),
            );
        }
        // Jobs and who holds them (Amendment A-18).
        for job in &package.jobs {
            let id = EntityId::from_raw(job.id);
            let at = |raw: u64| Value::Entity(EntityId::from_raw(raw));
            let mut facts = vec![
                (society::schema::JOB_TO, at(job.to)),
                (society::schema::JOB_KEEP, Value::Int(job.keep)),
                (
                    society::schema::JOB_HOURS,
                    Value::Vec3([job.hours.0 * 3600, job.hours.1 * 3600, 0]),
                ),
            ];
            match job.work {
                JobWork::Carry { material, from } => {
                    facts.push((society::schema::JOB_CARRIES, at(material)));
                    facts.push((society::schema::JOB_FROM, at(from)));
                }
                JobWork::Make { recipe } => facts.push((society::schema::JOB_MAKES, at(recipe))),
            }
            for (fact, v) in facts {
                store.seed(FactKey::new(id, fact), seeded(v));
            }
        }
        for &(who, job) in &package.roles {
            store.seed(
                FactKey::new(EntityId::from_raw(who), society::schema::ROLE),
                seeded(Value::Entity(EntityId::from_raw(job))),
            );
        }
    }

    // 4. Install the spatial index (Vol. V Ch. 2 §2.1, Amendment A-2) from the placement rule
    //    of the domain that owns space, now that initial reality is seeded: one full build,
    //    after which every commit keeps it current.
    for domain in &domains {
        if let Some(projector) = domain.spatial_projector() {
            store.install_spatial_index(projector);
        }
        // And each domain's agenda (Amendment A-22).
        store.install_agenda(&domain.agenda());
    }

    Ok(LoadedWorld {
        store,
        domains,
        systems,
        meters: Vec::new(),
    })
}

/// What `mind` knows at the start about each of `things`, read from the seeded world: where it
/// is; for an opening, where it leads and whether it is open; for a place, how warm its air is
/// (its own, or the nearest enclosing place's). Beliefs as facts about a pair (Amendment A-7).
fn starting_knowledge(
    store: &MemoryStore,
    mind: EntityId,
    things: &[u64],
) -> Vec<(FactKey, Value)> {
    let read = |e: EntityId, ft| store.read(FactKey::new(e, ft)).map(|f| f.value);
    let mut out = Vec::new();
    for thing in things.iter().map(|t| EntityId::from_raw(*t)) {
        if let Some(place) = read(thing, CONTAINED_IN) {
            out.push((FactKey::pair(mind, info::PLACE_OF, thing), place));
        }
        if let Some(held) = store
            .read(FactKey::new(thing, resources::schema::STOCK))
            .and_then(|f| f.value.as_int())
        {
            let units = Value::Int(held / resources::schema::UNIT);
            out.push((FactKey::pair(mind, info::STOCK, thing), units));
        }
        for m in store.read_all(FactKey::new(thing, MADE_OF)) {
            out.push((FactKey::pair(mind, info::MADE_OF, thing), m.value));
            if let Value::Entity(material) = m.value {
                if let Some(fed) = read(material, physical::schema::MATERIAL_NUTRITION) {
                    out.push((FactKey::pair(mind, info::NUTRITION, material), fed));
                }
                // How hard it is (Amendment A-45).
                if let Some(hard) = read(material, physical::schema::MATERIAL_HARDNESS) {
                    out.push((FactKey::pair(mind, info::HARDNESS, material), hard));
                }
            }
        }
        // How whole it is, and whether it is a fixture; for a place, whether it is walled and
        // what its ground is (A-45).
        if let Some(whole) = read(thing, physical::schema::INTEGRITY) {
            out.push((FactKey::pair(mind, info::INTEGRITY, thing), whole));
        }
        if read(thing, SOLID) == Some(Value::Bool(true)) {
            out.push((FactKey::pair(mind, info::FIXTURE, thing), Value::Bool(true)));
        }
        if is_place(store, thing) || read(thing, physical::schema::PLACE).is_some() {
            let walled = read(thing, ENCLOSED) == Some(Value::Bool(true));
            out.push((
                FactKey::pair(mind, info::WALLED, thing),
                Value::Bool(walled),
            ));
            if let Some(ground) = ancestry(store, thing, CONTAINED_IN)
                .into_iter()
                .find_map(|p| read(p, physical::schema::GROUND))
            {
                out.push((FactKey::pair(mind, info::GROUND, thing), ground));
            }
        }
        if let Some(yields) = read(thing, resources::schema::YIELD_MADE_OF) {
            out.push((FactKey::pair(mind, info::YIELDS, thing), yields));
            if let Value::Entity(material) = yields {
                if let Some(hard) = read(material, physical::schema::MATERIAL_HARDNESS) {
                    out.push((FactKey::pair(mind, info::HARDNESS, material), hard));
                }
                if let Some(fed) = read(material, physical::schema::MATERIAL_NUTRITION) {
                    out.push((FactKey::pair(mind, info::NUTRITION, material), fed));
                }
            }
        }
        if let Some(owner) = read(thing, economy::schema::OWNER) {
            out.push((FactKey::pair(mind, info::OWNER, thing), owner));
        }
        if let Some(makes) = read(thing, economy::schema::RECIPE_MAKES) {
            out.push((FactKey::pair(mind, info::RECIPE_MAKES, thing), makes));
            if let Some(at) = read(thing, economy::schema::RECIPE_AT) {
                out.push((FactKey::pair(mind, info::RECIPE_AT, thing), at));
            }
            for (material, count) in store.read_about(thing, economy::schema::RECIPE_NEEDS) {
                let needs = [material.raw() as i64, count.value.as_int().unwrap_or(0), 0];
                out.push((
                    FactKey::pair(mind, info::RECIPE_NEEDS, thing),
                    Value::Vec3(needs),
                ));
            }
        }
        if let Some(leads) = read(thing, LEADS_TO) {
            out.push((FactKey::pair(mind, info::LEADS_TO, thing), leads));
            let open = !matches!(read(thing, PORTAL_OPEN), Some(Value::Bool(false)));
            out.push((FactKey::pair(mind, info::OPEN, thing), Value::Bool(open)));
        }
        let air = ancestry(store, thing, CONTAINED_IN)
            .into_iter()
            .find_map(|p| read(p, TEMPERATURE));
        if let (true, Some(air)) = (is_place(store, thing), air) {
            out.push((FactKey::pair(mind, info::WARMTH_OF, thing), air));
        }
    }
    out
}

/// Whether `entity` is a place one can be in: something is in it, or it is walled, or it is a
/// climate.
fn is_place(store: &MemoryStore, entity: EntityId) -> bool {
    store.read(FactKey::new(entity, ENCLOSED)).is_some()
        || store.read(FactKey::new(entity, TEMPERATURE)).is_some()
        || store.read(FactKey::new(entity, HAS_PORTAL)).is_some()
}

/// A belief seeded as starting knowledge: formed at tick 0, known from the start.
fn known(value: Value) -> Fact {
    Fact::new(
        value,
        Provenance::new(SystemId::new("worldgen"), 0, Cause::new("known")),
    )
}

/// A fact seeded at world construction (generation), attributed to worldgen at tick 0.
fn seeded(value: Value) -> Fact {
    Fact::new(
        value,
        Provenance::new(SystemId::new("worldgen"), 0, Cause::new("package_seed")),
    )
}

/// The Physical Reality fact type a material property is stored as (Vol. III Ch. 1 §1.9). The
/// package's closed property set maps one-to-one onto the domain's material schema.
fn material_fact(property: MaterialProperty) -> FactType {
    match property {
        MaterialProperty::Density => MATERIAL_DENSITY,
        MaterialProperty::Hardness => MATERIAL_HARDNESS,
        MaterialProperty::ThermalCapacity => MATERIAL_THERMAL_CAPACITY,
        MaterialProperty::Flammability => MATERIAL_FLAMMABILITY,
        MaterialProperty::Conductivity => MATERIAL_CONDUCTIVITY,
        MaterialProperty::Toxicity => MATERIAL_TOXICITY,
        MaterialProperty::Edible => physical::schema::MATERIAL_EDIBLE,
        MaterialProperty::Potency => physical::schema::MATERIAL_POTENCY,
        MaterialProperty::Habit => physical::schema::MATERIAL_HABIT,
        MaterialProperty::Nutrition => physical::schema::MATERIAL_NUTRITION,
    }
}
