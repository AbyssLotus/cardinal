//! Portals: navigational connectivity distinct from adjacency (Vol. III Ch. 1 §1.5). A portal
//! is a located connection from a spot in one region to another region; a region may host
//! several; a portal's destination can change. Reachability walks the portal graph -- so a
//! sealed room cannot be left, and a basement reaches the yard only THROUGH the ground floor.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::Provenance;
use kernel::fact::{Cause, Fact, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::store::MemoryStore;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{CONTAINED_IN, HAS_PORTAL, LEADS_TO, POSITION_X, POSITION_Y};
use physical::space::{can_reach, destinations, portals_in, position_in, reachable_regions};
use physical::{PhysicalConfig, PhysicalDomain};
use std::collections::BTreeSet;

// Regions.
const YARD: u64 = 1;
const GROUND: u64 = 2;
const BASEMENT: u64 = 3;
const SECOND: u64 = 4;
const VAULT: u64 = 5;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}
fn prov() -> Provenance {
    Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
}
fn seed(store: &mut MemoryStore, entity: u64, ft: FactType, v: Value) {
    store.seed(FactKey::new(e(entity), ft), Fact::new(v, prov()));
}

/// Add a portal `id` in `host`, leading to `dest`, at local spot (x, y).
fn portal(store: &mut MemoryStore, id: u64, host: u64, dest: u64, x: i64, y: i64) {
    seed(store, id, CONTAINED_IN, Value::Entity(e(host))); // the portal is located in its host
    seed(store, id, POSITION_X, Value::Int(x));
    seed(store, id, POSITION_Y, Value::Int(y));
    seed(store, id, LEADS_TO, Value::Entity(e(dest))); // ...and leads to its far side
    seed(store, host, HAS_PORTAL, Value::Entity(e(id))); // the region hosts it (many-valued)
}

/// A manor: yard, ground floor, basement, second floor, and a sealed vault. Doors and stairs
/// are one-way portals seeded in pairs; the vault has none.
fn manor() -> MemoryStore {
    let mut s = MemoryStore::new();
    portal(&mut s, 1000, GROUND, YARD, 300, 0); // front door (inside)
    portal(&mut s, 1001, YARD, GROUND, 0, 0); // front door (outside)
    portal(&mut s, 1002, BASEMENT, GROUND, 100, 0); // basement stairs up
    portal(&mut s, 1003, GROUND, BASEMENT, 100, 0); // stairs down to basement
    portal(&mut s, 1004, SECOND, GROUND, 200, 0); // second-floor stairs down
    portal(&mut s, 1005, GROUND, SECOND, 200, 0); // stairs up to second floor
    s
}

fn regions(store: &MemoryStore, set: BTreeSet<EntityId>) -> BTreeSet<u64> {
    let _ = store;
    set.into_iter().map(|x| x.raw()).collect()
}

#[test]
fn basement_reaches_the_yard_only_through_the_ground_floor() {
    let s = manor();
    // One hop from the basement reaches ONLY the ground floor -- not the yard.
    assert_eq!(destinations(&s, e(BASEMENT)), vec![e(GROUND)]);
    // But the yard IS reachable overall -- by going up to the ground floor first.
    assert!(can_reach(&s, e(BASEMENT), e(YARD)));
    assert_eq!(
        regions(&s, reachable_regions(&s, e(BASEMENT))),
        BTreeSet::from([YARD, GROUND, BASEMENT, SECOND])
    );
}

#[test]
fn a_sealed_region_cannot_be_left() {
    let s = manor();
    // The vault hosts no portals: nowhere to go, and no way in.
    assert_eq!(reachable_regions(&s, e(VAULT)), BTreeSet::from([e(VAULT)]));
    assert!(!can_reach(&s, e(VAULT), e(YARD)));
    assert!(!can_reach(&s, e(BASEMENT), e(VAULT)));
}

#[test]
fn a_region_can_host_several_portals() {
    let s = manor();
    // The ground floor has three portals: the front door, stairs down, stairs up.
    let mut ps: Vec<u64> = portals_in(&s, e(GROUND)).iter().map(|p| p.raw()).collect();
    ps.sort_unstable();
    assert_eq!(ps, vec![1000, 1003, 1005]);
}

#[test]
fn a_portal_is_a_locatable_spot() {
    let s = manor();
    // The front door sits at a real position within the ground floor.
    assert_eq!(position_in(&s, e(1000), e(GROUND)), Some([300, 0, 0]));
}

/// A system that re-targets a portal by Setting its destination -- a gangplank swung to a new
/// dock, or a gate re-attuned.
struct RerouteGate {
    portal: EntityId,
    new_dest: EntityId,
}
impl System for RerouteGate {
    fn id(&self) -> SystemId {
        SystemId::new("test.reroute_gate")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[LEADS_TO]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _v: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(self.portal, LEADS_TO),
            ctx.basis_tick(),
            Change::Set(Value::Entity(self.new_dest)),
            Cause::new("reroute"),
        )]
    }
}

fn config() -> PhysicalConfig {
    PhysicalConfig {
        ticks_per_day: 24,
        diurnal_amplitude_centi_c: 400,
        weather_max_swing_centi_c: 40,
        illumination_peak: 10000,
        humidity_baseline: 5500,
        humidity_swing: 80,
        humidity_drying_divisor: 8,
        pressure_sea_level: 10130,
        pressure_elevation_factor: 1,
        pressure_weather_swing: 20,
        pressure_settle_divisor: 8,
        wind_gradient_divisor: 10,
        fall_danger_per_meter: 1500,
        thermal_mass_reference: 1000,
    }
}

#[test]
fn changing_a_portals_destination_reroutes_connectivity() {
    let mut s = manor();
    // Before: the basement's stairs lead up to the ground floor, so the yard is reachable.
    assert!(can_reach(&s, e(BASEMENT), e(YARD)));
    assert!(!can_reach(&s, e(BASEMENT), e(VAULT)));

    // Re-target the basement stairs (portal 1002) to open onto the vault instead, through the
    // real tick loop (owner composition Sets the single-valued destination). The domain
    // discovers portals and regions from committed reality, so it needs no region list.
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut systems = domain.systems();
    systems.push(Box::new(RerouteGate {
        portal: e(1002),
        new_dest: e(VAULT),
    }));
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    run_tick(&mut s, &domains, &systems, 1, 0, &mut chronicle).expect("tick commits");

    // After: the basement now opens onto the sealed vault -- and can no longer reach the yard.
    assert!(!can_reach(&s, e(BASEMENT), e(YARD)));
    assert!(can_reach(&s, e(BASEMENT), e(VAULT)));
}
