//! Living Systems in isolation: body heat responds to the temperature of the region an
//! organism inhabits (Vol. III Ch. 2, the "warmth" need tracks the environment).
//!
//! Physical Reality's containment and temperature facts are seeded directly here by their
//! published ids, standing in for a running physical domain — this crate has no dependency
//! on `physical` (Vol. III Ch. 12, invariant 1). The full cross-domain run (physical
//! actually producing temperature) is exercised in the `packages` tests.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use living::schema::{AMBIENT_TEMPERATURE, BODY_HEAT, BODY_HEAT_FLOOR_CENTI_C, CONTAINED_IN};
use living::LivingDomain;

const REGION: EntityId = EntityId::from_raw(1);
const ORGANISM: EntityId = EntityId::from_raw(100);

fn seed_int(store: &mut MemoryStore, key: FactKey, v: i64) {
    store.seed(key, fact(Value::Int(v)));
}

fn fact(v: Value) -> Fact {
    Fact::new(
        v,
        Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
    )
}

/// Settle an organism (starting at 37.00 C body heat, placed in a region held at `ambient`)
/// for `ticks` ticks, and return its final body heat.
fn settled_body_heat(ambient: i64, ticks: u64) -> i64 {
    let mut store = MemoryStore::new();
    // Physical facts, seeded by id: the organism lives in the region, held at `ambient`.
    store.seed(
        FactKey::new(ORGANISM, CONTAINED_IN),
        fact(Value::Entity(REGION)),
    );
    seed_int(
        &mut store,
        FactKey::new(REGION, AMBIENT_TEMPERATURE),
        ambient,
    );
    seed_int(&mut store, FactKey::new(ORGANISM, BODY_HEAT), 3700);

    let domain = LivingDomain::new(3700, 6, 3);
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();

    for t in 1..=ticks {
        run_tick(&mut store, &domains, &systems, t, 0, &mut chronicle).expect("tick commits");
    }
    store
        .read(FactKey::new(ORGANISM, BODY_HEAT))
        .unwrap()
        .value
        .as_int()
        .unwrap()
}

#[test]
fn body_heat_settles_colder_in_a_colder_region() {
    let cold = settled_body_heat(-500, 300);
    let warm = settled_body_heat(2500, 300);
    assert!(cold < warm, "cold={cold} should be < warm={warm}");
    assert!(
        warm < 3700,
        "environment holds body heat below the 37.00 C set point"
    );
    assert!(cold > BODY_HEAT_FLOOR_CENTI_C);
}

#[test]
fn no_proposal_without_a_region() {
    // An organism with no committed containment cannot sense its environment, so body heat
    // is left untouched -- living reads two Physical facts and needs both.
    let mut store = MemoryStore::new();
    seed_int(&mut store, FactKey::new(ORGANISM, BODY_HEAT), 3700);
    let domain = LivingDomain::new(3700, 6, 3);
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();
    for t in 1..=20 {
        run_tick(&mut store, &domains, &systems, t, 0, &mut chronicle).unwrap();
    }
    assert!(chronicle.is_empty(), "no region -> no body-heat change");
    assert_eq!(
        store
            .read(FactKey::new(ORGANISM, BODY_HEAT))
            .unwrap()
            .value
            .as_int()
            .unwrap(),
        3700
    );
}
