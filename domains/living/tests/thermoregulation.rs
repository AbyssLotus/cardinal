//! Living Systems in isolation: body heat responds to a region's ambient temperature
//! (Vol. III Ch. 2, the "warmth" need tracks the environment).
//!
//! Physical Reality's temperature fact is seeded directly here by its published id, standing
//! in for a running physical domain — this crate has no dependency on `physical`
//! (Vol. III Ch. 12, invariant 1). The full cross-domain run (physical actually producing
//! temperature) is exercised in the `packages` loader tests.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::{Cause, Fact, FactKey, Provenance, SystemId};
use kernel::identity::EntityId;
use kernel::store::MemoryStore;
use kernel::system::CommittedView;
use kernel::tick::run_tick;
use kernel::value::Value;
use living::schema::{AMBIENT_TEMPERATURE, BODY_HEAT, BODY_HEAT_FLOOR_CENTI_C};
use living::{LivingDomain, OrganismPlacement};

const REGION: EntityId = EntityId::from_raw(1);
const ORGANISM: EntityId = EntityId::from_raw(100);

fn seed(store: &mut MemoryStore, key: FactKey, v: i64) {
    store.seed(
        key,
        Fact::new(
            Value::Int(v),
            Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed")),
        ),
    );
}

/// Settle an organism (starting at 37.00 C body heat) in a region held at `ambient`, for
/// `ticks` ticks, and return its final body heat.
fn settled_body_heat(ambient: i64, ticks: u64) -> i64 {
    let mut store = MemoryStore::new();
    seed(
        &mut store,
        FactKey::new(REGION, AMBIENT_TEMPERATURE),
        ambient,
    );
    seed(&mut store, FactKey::new(ORGANISM, BODY_HEAT), 3700);

    let domain = LivingDomain::new(
        vec![OrganismPlacement {
            organism: ORGANISM,
            region: REGION,
        }],
        3700, // set point 37.00 C
        6,    // warm_response
        3,    // cold_response
    );
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
    let cold = settled_body_heat(-500, 300); // -5.00 C ambient
    let warm = settled_body_heat(2500, 300); //  25.00 C ambient

    // Colder ambient pulls body heat lower; the environment keeps both below the set point;
    // neither falls through the physical floor.
    assert!(cold < warm, "cold={cold} should be < warm={warm}");
    assert!(
        warm < 3700,
        "environment holds body heat below the 37.00 C set point"
    );
    assert!(cold > BODY_HEAT_FLOOR_CENTI_C);
}

#[test]
fn temperature_fact_is_never_written_by_living() {
    // Living only reads ambient temperature; it must never write it (Vol. III Ch. 12 §12.1).
    let mut store = MemoryStore::new();
    seed(&mut store, FactKey::new(REGION, AMBIENT_TEMPERATURE), 1000);
    seed(&mut store, FactKey::new(ORGANISM, BODY_HEAT), 3700);
    let domain = LivingDomain::new(
        vec![OrganismPlacement {
            organism: ORGANISM,
            region: REGION,
        }],
        3700,
        6,
        3,
    );
    let domains: [&dyn Domain; 1] = [&domain];
    let systems = domain.systems();
    let mut chronicle = Vec::new();
    for t in 1..=50 {
        run_tick(&mut store, &domains, &systems, t, 0, &mut chronicle).unwrap();
    }
    // Ambient temperature is untouched by the living domain.
    assert_eq!(
        store
            .read(FactKey::new(REGION, AMBIENT_TEMPERATURE))
            .unwrap()
            .value
            .as_int()
            .unwrap(),
        1000
    );
}
