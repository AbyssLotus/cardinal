//! A portal's danger (Vol. III Ch. 1 §1.11): world-pinned when given, otherwise derived from
//! the portal's height above the ground -- a 3rd-storey window is perilous, a ground-floor
//! door is not. Weather can raise the derived value later (a hook is left in the system).

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, FactType, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{
    CONTAINED_IN, HAS_PORTAL, PORTAL_DANGER, PORTAL_DANGER_OVERRIDE, POSITION_Z,
};
use physical::{PhysicalConfig, PhysicalDomain};

const GROUND: u64 = 2;
const SECOND: u64 = 4;

fn e(id: u64) -> EntityId {
    EntityId::from_raw(id)
}
fn prov() -> Provenance {
    Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
}
fn seed(s: &mut MemoryStore, entity: u64, ft: FactType, v: Value) {
    s.seed(FactKey::new(e(entity), ft), Fact::new(v, prov()));
}

/// A portal `id` hosted in `host` at height `z` (cm); `pin` optionally fixes its danger.
fn portal(s: &mut MemoryStore, id: u64, host: u64, z: i64, pin: Option<i64>) {
    seed(s, host, HAS_PORTAL, Value::Entity(e(id)));
    seed(s, id, CONTAINED_IN, Value::Entity(e(host)));
    seed(s, id, POSITION_Z, Value::Int(z));
    if let Some(d) = pin {
        seed(s, id, PORTAL_DANGER_OVERRIDE, Value::Int(d));
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
    }
}

fn danger(store: &MemoryStore, portal: u64) -> i64 {
    store
        .read(FactKey::new(e(portal), PORTAL_DANGER))
        .and_then(|f| f.value.as_int())
        .unwrap_or(-1)
}

fn run(store: &mut MemoryStore) {
    let domain = PhysicalDomain::new(config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    run_tick(store, &domains, &systems, 1, 0, &mut chronicle).expect("tick commits");
}

#[test]
fn danger_is_derived_from_height_when_the_world_gives_none() {
    let mut s = MemoryStore::new();
    portal(&mut s, 1000, GROUND, 0, None); // ground-floor door: no fall
    portal(&mut s, 1006, SECOND, 600, None); // 6 m window: a real drop
    run(&mut s);
    // fall_danger_per_meter = 1500: 0 m -> 0, 6 m -> 9000.
    assert_eq!(danger(&s, 1000), 0);
    assert_eq!(danger(&s, 1006), 9000);
}

#[test]
fn a_world_pinned_danger_overrides_height() {
    let mut s = MemoryStore::new();
    // A warded gate up high, but pinned as maximally dangerous...
    portal(&mut s, 1007, SECOND, 600, Some(10000));
    // ...and a padded chute up equally high, pinned harmless.
    portal(&mut s, 1008, SECOND, 600, Some(0));
    run(&mut s);
    assert_eq!(danger(&s, 1007), 10000);
    assert_eq!(danger(&s, 1008), 0);
}
