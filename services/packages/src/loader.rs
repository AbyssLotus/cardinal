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

use crate::model::{Flag, MaterialProperty, WorldPackage};
use crate::version::Version;
use information::schema as info;
use information::{InformationConfig, InformationDomain};
use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::{CommittedView, System};
use kernel::tick::{run_tick, TickError};
use kernel::time::SimClock;
use kernel::value::Value;
use living::schema::{BODY_HEAT, SIGHT_RANGE};
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
        },
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
        gravity_cm_s2: r.gravity_cm_s2,
        step_height_cm: r.step_height_cm,
        max_slope_percent: r.max_slope_percent,
        nav_cell_cm: r.nav_cell_cm,
        reach_cm: r.reach_cm,
        indoor_coupling_seconds: r.indoor_coupling_seconds,
        sight_step_seconds: r.sight_step_seconds,
        sight_min_illumination: r.sight_min_illumination,
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
    for d in &package.manifest.domains {
        match d.as_str() {
            "physical" => has_physical = true,
            "living" => has_living = true,
            "information" => has_information = true,
            "minds" => has_minds = true,
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

    // 3b. Living Systems (optional): configured from package rules. Living reads organism
    //     containment and region temperature by id — no wiring between domains is needed.
    if has_living {
        let living =
            LivingDomain::new(living_config(package).ok_or(LoadError::LivingRulesMissing)?);
        systems.extend(living.systems());
        domains.push(Box::new(living));
        for o in &package.organisms {
            store.seed(
                FactKey::new(EntityId::from_raw(o.id), BODY_HEAT),
                seeded(Value::Int(o.body_heat_centi_c)),
            );
        }
        // Sensory capability is Living's (Amendment A-8).
        for &(organism, range) in &package.senses {
            store.seed(
                FactKey::new(EntityId::from_raw(organism), SIGHT_RANGE),
                seeded(Value::Int(range)),
            );
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
    }
}
