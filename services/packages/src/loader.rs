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

use crate::model::{MaterialProperty, WorldPackage};
use crate::version::Version;
use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::System;
use kernel::tick::{run_tick, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use living::schema::BODY_HEAT;
use living::{LivingConfig, LivingDomain};
use physical::schema::{
    ADJACENT_TO, BODY_SIZE, CONTAINED_IN, ELEVATION, EXPOSURE, HAS_PORTAL, HEADING, IN_REGION,
    LEADS_TO, MADE_OF, MATERIAL_CONDUCTIVITY, MATERIAL_DENSITY, MATERIAL_FLAMMABILITY,
    MATERIAL_HARDNESS, MATERIAL_THERMAL_CAPACITY, MATERIAL_TOXICITY, MOTION_END, MOTION_START,
    MOTION_TARGET, PORTAL_DANGER_OVERRIDE, POSITION, TEMPERATURE,
};
use physical::{PhysicalConfig, PhysicalDomain};
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
    /// A selected domain has no implementation wired into the loader yet.
    UnsupportedDomain(String),
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
            LoadError::UnsupportedDomain(d) => {
                write!(f, "selected domain `{d}` is not implemented yet")
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

    /// Advance the loaded world by one tick under `seed`, appending to `chronicle`
    /// (Vol. V Ch. 3 §3.1).
    pub fn tick(
        &mut self,
        tick: u64,
        seed: u64,
        chronicle: &mut Vec<ChronicleEntry>,
    ) -> Result<(), TickError> {
        let domain_refs: Vec<&dyn Domain> = self.domains.iter().map(|d| d.as_ref()).collect();
        run_tick(
            &mut self.store,
            &domain_refs,
            &self.systems,
            tick,
            seed,
            chronicle,
        )
    }
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

    // 2. Domain selection: Physical Reality is mandatory; unknown domains are refused rather
    //    than silently ignored (Vol. IV Ch. 2, invariants 2 & 3).
    let mut has_physical = false;
    let mut has_living = false;
    for d in &package.manifest.domains {
        match d.as_str() {
            "physical" => has_physical = true,
            "living" => has_living = true,
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
    // The world's clock (Vol. II Ch. 2, Amendment A-1): both domains step in simulated time.
    let clock = SimClock::new(package.clock.tick_ms);
    let r = &package.physical_rules;
    let config = PhysicalConfig {
        clock,
        day_length_seconds: package.clock.day_seconds,
        environment_step_seconds: r.environment_step_seconds,
        diurnal_amplitude_centi_c: r.diurnal_amplitude_centi_c,
        temperature_variability_centi_c: r.temperature_variability_centi_c,
        weather_persistence_seconds: r.weather_persistence_seconds,
        illumination_peak: r.illumination_peak,
        humidity_baseline: r.humidity_baseline,
        humidity_variability: r.humidity_variability,
        pressure_sea_level: r.pressure_sea_level,
        pressure_elevation_factor: r.pressure_elevation_factor,
        pressure_variability: r.pressure_variability,
        wind_gradient_divisor: r.wind_gradient_divisor,
        fall_danger_per_meter: r.fall_danger_per_meter,
        thermal_mass_reference: r.thermal_mass_reference,
    };
    let physical = PhysicalDomain::new(config);
    systems.extend(physical.systems());
    domains.push(Box::new(physical));

    // Seed regions' initial physical state: temperature always, elevation when specified.
    for region in &package.regions {
        store.seed(
            FactKey::new(EntityId::from_raw(region.id), TEMPERATURE),
            seeded(Value::Int(region.temperature_centi_c)),
        );
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

    // 3b. Living Systems (optional): configured from package rules. Living reads organism
    //     containment and region temperature by id — no wiring between domains is needed.
    if has_living {
        let rules = package.living_rules.ok_or(LoadError::LivingRulesMissing)?;
        let living = LivingDomain::new(LivingConfig {
            clock,
            metabolism_step_seconds: rules.metabolism_step_seconds,
            set_point_centi_c: rules.set_point_centi_c,
            warm_response_seconds: rules.warm_response_seconds,
            cold_response_seconds: rules.cold_response_seconds,
        });
        systems.extend(living.systems());
        domains.push(Box::new(living));
        for o in &package.organisms {
            store.seed(
                FactKey::new(EntityId::from_raw(o.id), BODY_HEAT),
                seeded(Value::Int(o.body_heat_centi_c)),
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
    }

    Ok(LoadedWorld {
        store,
        domains,
        systems,
    })
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
    }
}
