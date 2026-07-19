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

use crate::model::WorldPackage;
use crate::version::Version;
use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::System;
use kernel::tick::{run_tick, TickError};
use kernel::value::Value;
use living::schema::BODY_HEAT;
use living::LivingDomain;
use physical::schema::{ADJACENT_TO, CONTAINED_IN, ELEVATION, TEMPERATURE};
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

    // 3a. Physical Reality: configured from package rules (invariant 5).
    let region_ids: Vec<EntityId> = package
        .regions
        .iter()
        .map(|r| EntityId::from_raw(r.id))
        .collect();
    let config = PhysicalConfig {
        ticks_per_day: package.physical_rules.ticks_per_day,
        diurnal_amplitude_centi_c: package.physical_rules.diurnal_amplitude_centi_c,
        weather_max_swing_centi_c: package.physical_rules.weather_max_swing_centi_c,
        illumination_peak: package.physical_rules.illumination_peak,
        humidity_baseline: package.physical_rules.humidity_baseline,
        humidity_swing: package.physical_rules.humidity_swing,
        humidity_drying_divisor: package.physical_rules.humidity_drying_divisor,
    };
    let physical = PhysicalDomain::new(region_ids, config);
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

    // 3b. Living Systems (optional): configured from package rules. Living reads organism
    //     containment and region temperature by id — no wiring between domains is needed.
    if has_living {
        let rules = package.living_rules.ok_or(LoadError::LivingRulesMissing)?;
        let organism_ids: Vec<EntityId> = package
            .organisms
            .iter()
            .map(|o| EntityId::from_raw(o.id))
            .collect();
        let living = LivingDomain::new(
            organism_ids,
            rules.set_point_centi_c,
            rules.warm_response,
            rules.cold_response,
        );
        systems.extend(living.systems());
        domains.push(Box::new(living));
        for o in &package.organisms {
            store.seed(
                FactKey::new(EntityId::from_raw(o.id), BODY_HEAT),
                seeded(Value::Int(o.body_heat_centi_c)),
            );
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
