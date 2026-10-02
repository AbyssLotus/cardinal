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
use kernel::time::SimClock;
use kernel::value::Value;
use living::schema::{AMBIENT_TEMPERATURE, BODY_HEAT, BODY_HEAT_FLOOR_CENTI_C, CONTAINED_IN};
use living::{LivingConfig, LivingDomain};

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

/// Shared metabolic rules: a 37.00 C set point, a 6-hour pull toward it, and a 3-hour pull
/// toward the air, stepping every tick of `tick_seconds`.
fn domain(tick_seconds: u64) -> LivingDomain {
    LivingDomain::new(LivingConfig {
        clock: SimClock::new(tick_seconds * 1000),
        metabolism_step_seconds: tick_seconds,
        set_point_centi_c: 3700,
        warm_response_seconds: 6 * 3600,
        cold_response_seconds: 3 * 3600,
    })
}

/// Settle an organism (starting at 37.00 C body heat, placed in a region held at `ambient`)
/// for `ticks` hour-long ticks, and return its final body heat.
fn settled_body_heat(ambient: i64, ticks: u64) -> i64 {
    settled_at(ambient, 3600, ticks)
}

/// As [`settled_body_heat`], with ticks of `tick_seconds`.
fn settled_at(ambient: i64, tick_seconds: u64, ticks: u64) -> i64 {
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

    let domain = domain(tick_seconds);
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
    let domain = domain(3600);
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

#[test]
fn a_body_settles_at_the_same_temperature_at_any_tick_length() {
    // Time has units (Amendment A-1): the rules are time constants, so a day in the cold ends
    // in the same place whether the world ticks hourly or every minute. The equilibrium is
    // (S·τc + A·τw)/(τc + τw) = (3700·3 + 500·6)/9 ≈ 1567 centidegrees.
    let hourly = settled_at(500, 3600, 24 * 3);
    let by_minute = settled_at(500, 60, 60 * 24 * 3);
    assert!((hourly - 1567).abs() <= 25, "hourly {hourly}");
    assert!((by_minute - 1567).abs() <= 25, "by the minute {by_minute}");
    // Before the fix, a per-minute body stalled where each step's change rounded to zero; the
    // unbiased rounding lets it reach equilibrium like the hourly one.
    assert!((hourly - by_minute).abs() <= 25, "{hourly} vs {by_minute}");
}
