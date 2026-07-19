//! The new environmental fields and spatial containment (Vol. III Ch. 1 §1.8, §1.10).

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use physical::schema::{CONTAINED_IN, HUMIDITY, ILLUMINATION, PERCENT_FULL};
use physical::{PhysicalConfig, PhysicalDomain};

const REGION: EntityId = EntityId::from_raw(1);

fn config() -> PhysicalConfig {
    PhysicalConfig {
        ticks_per_day: 24,
        diurnal_amplitude_centi_c: 500,
        weather_max_swing_centi_c: 50,
        illumination_peak: 10000,
        humidity_baseline: 6000,
        humidity_swing: 100,
        humidity_drying_divisor: 8,
        pressure_sea_level: 10130,
        pressure_elevation_factor: 1,
        pressure_weather_swing: 20,
        pressure_settle_divisor: 8,
        wind_gradient_divisor: 10,
    }
}

fn illumination_after(ticks: u64) -> i64 {
    let mut store = MemoryStore::new();
    let domain = PhysicalDomain::new(vec![REGION], config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 1, &mut chronicle).expect("tick");
    }
    store
        .read(FactKey::new(REGION, ILLUMINATION))
        .expect("illumination created")
        .value
        .as_int()
        .unwrap()
}

#[test]
fn illumination_follows_the_sun() {
    // Peaks at midday (tick 12 of a 24-tick day) and is dark at midnight (tick 24).
    assert_eq!(illumination_after(12), 10000);
    assert_eq!(illumination_after(24), 0);
    // Dawn (tick 6) sits between dark and full.
    let dawn = illumination_after(6);
    assert!(dawn > 0 && dawn < 10000, "dawn={dawn}");
}

#[test]
fn humidity_is_created_at_baseline_and_stays_bounded() {
    let mut store = MemoryStore::new();
    let domain = PhysicalDomain::new(vec![REGION], config());
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();
    for t in 1..=200 {
        run_tick(&mut store, &domains, &systems, t, 42, &mut chronicle).expect("tick");
    }
    let h = store
        .read(FactKey::new(REGION, HUMIDITY))
        .expect("humidity created")
        .value
        .as_int()
        .unwrap();
    // Never leaves its range, and the drift keeps it in the neighbourhood of the baseline.
    assert!((0..=PERCENT_FULL).contains(&h), "humidity {h} out of range");
    assert!(
        (4000..=8000).contains(&h),
        "humidity {h} drifted far from baseline 6000"
    );
}

/// Walk the containment chain from `start` upward, returning each successive container.
fn containment_chain(store: &MemoryStore, start: EntityId) -> Vec<u64> {
    let mut chain = Vec::new();
    let mut here = start;
    while let Some(fact) = store.read(FactKey::new(here, CONTAINED_IN)) {
        if let Value::Entity(parent) = fact.value {
            chain.push(parent.raw());
            here = parent;
        } else {
            break;
        }
    }
    chain
}

#[test]
fn containment_forms_a_walkable_hierarchy() {
    // organism(100) in region(1) in continent(1000): a seeded containment hierarchy.
    let mut store = MemoryStore::new();
    let prov = Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"));
    store.seed(
        FactKey::new(EntityId::from_raw(100), CONTAINED_IN),
        Fact::new(Value::Entity(EntityId::from_raw(1)), prov),
    );
    store.seed(
        FactKey::new(EntityId::from_raw(1), CONTAINED_IN),
        Fact::new(Value::Entity(EntityId::from_raw(1000)), prov),
    );

    // Walking upward from the organism yields its region, then its continent.
    assert_eq!(
        containment_chain(&store, EntityId::from_raw(100)),
        vec![1, 1000]
    );
}
