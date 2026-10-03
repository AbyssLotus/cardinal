//! Ashford's living things (Vol. III Ch. 2; Vol. III Ch. 12): every body defends its heat
//! against the air of whatever encloses it — the moor, the fen, a bedroom, a moving cart — at
//! the same pace whatever the world's tick length; and Living only ever reads Physical
//! Reality, never moves it.

use kernel::domain::Domain;
use kernel::events::ChronicleEntry;
use kernel::fact::FactKey;
use kernel::store::{CommitBatch, RealityStore, Resolution};
use kernel::system::{CommittedView, System};
use kernel::tick::run_tick;
use living::schema::BODY_HEAT;
use living::LivingDomain;
use physical::schema::{POSITION, TEMPERATURE};
use reference::id::*;
use reference::{e, package, with_tick_seconds, City};

fn heat(city: &City, who: u64) -> i64 {
    city.int(who, BODY_HEAT).unwrap()
}

/// Living alone over a copy of Ashford's reality — the air held where the world seeds it — for
/// `days` days of ticks of `tick_seconds`; the body heat of each of `who` at the end.
fn settled(tick_seconds: u64, days: u64, who: &[u64]) -> Vec<i64> {
    let pkg = with_tick_seconds(tick_seconds);
    let mut store = City::from(pkg.clone()).store().clone();
    let domain = LivingDomain::new(packages::living_config(&pkg).expect("Ashford has living"));
    let domains: [&dyn Domain; 1] = [&domain];
    let systems: Vec<Box<dyn System>> = domain.systems();
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    for t in 1..=days * 86_400 / tick_seconds {
        run_tick(&mut store, &domains, &systems, t, 7, &mut chronicle).expect("commits");
        chronicle.clear();
    }
    who.iter()
        .map(|id| {
            store
                .read(FactKey::new(e(*id), BODY_HEAT))
                .and_then(|f| f.value.as_int())
                .unwrap()
        })
        .collect()
}

#[test]
fn a_body_settles_where_its_air_puts_it_at_any_tick_length() {
    // Pulled toward 37 °C over six hours and toward the air over three, a body settles a third
    // of the way from the air to 37 °C: Hal on Highmoor (6 °C) at 16.33 °C, Erin in the Vale
    // (14 °C) at 21.67 °C, Ida in Southfen (18 °C) at 24.33 °C. Alice upstairs feels the Vale's
    // air too, inherited through her house. The same, ticking by the hour or by the minute.
    for tick_seconds in [3_600, 60] {
        let got = settled(tick_seconds, 10, &[HAL, ERIN, IDA, ALICE]);
        for (g, want) in got.iter().zip([1_633, 2_167, 2_433, 2_167]) {
            assert!((g - want).abs() <= 2, "ticks of {tick_seconds} s: {got:?}");
        }
    }
}

#[test]
fn hal_on_the_moor_feels_the_cold_and_ida_in_the_fen_does_not() {
    let mut city = City::from(with_tick_seconds(600));
    city.run(2 * 144);
    assert!(heat(&city, HAL) < heat(&city, FINN));
    assert!(heat(&city, FINN) < heat(&city, IDA));
}

#[test]
fn the_rider_feels_the_air_the_cart_rolls_through() {
    // The rider is in the cart, not in any place with a climate of its own: they feel the air
    // of Old Town, which is the Vale's — as Finn, standing in the yard, does.
    let mut city = City::from(with_tick_seconds(600));
    city.run(2 * 144);
    assert!(
        (heat(&city, RIDER) - heat(&city, FINN)).abs() <= 2,
        "rider {} vs Finn {}",
        heat(&city, RIDER),
        heat(&city, FINN)
    );
}

#[test]
fn living_never_disturbs_physical_reality() {
    // Non-interference (Vol. III Ch. 12, invariant 7): with Living running or not, Ashford's
    // air and everything in it moves identically — Living reads temperature and containment
    // and writes only body heat.
    fn trajectory(pkg: &packages::WorldPackage) -> Vec<Vec<Option<kernel::value::Value>>> {
        let mut city = City::from(pkg.clone());
        (0..120)
            .map(|_| {
                city.run(1);
                let mut at: Vec<_> = [VALE, HIGHMOOR, KITCHEN, HALL]
                    .iter()
                    .map(|id| city.read(*id, TEMPERATURE))
                    .collect();
                at.extend([CART, RIDER, HAL].iter().map(|id| city.read(*id, POSITION)));
                at
            })
            .collect()
    }
    let mut with_living = package();
    with_living.clock.tick_ms = 600_000;
    let mut physical_only = with_living.clone();
    physical_only.manifest.domains = vec!["physical".to_string()];
    physical_only.living_rules = None;
    physical_only.organisms = Vec::new();
    assert_eq!(trajectory(&with_living), trajectory(&physical_only));
}

#[test]
fn a_body_with_no_air_around_it_is_left_alone() {
    // Erin, taken out of the world (her containment cleared): no air reaches her, so Living has
    // nothing to say about her body heat — no proposal, no change.
    let pkg = with_tick_seconds(600);
    let mut store = City::from(pkg.clone()).store().clone();
    let mut batch = CommitBatch::new(1);
    batch.resolutions.push(Resolution::Clear {
        key: FactKey::new(e(ERIN), physical::schema::CONTAINED_IN),
    });
    store.apply(batch);
    let domain = LivingDomain::new(packages::living_config(&pkg).unwrap());
    let domains: [&dyn Domain; 1] = [&domain];
    let mut chronicle: Vec<ChronicleEntry> = Vec::new();
    run_tick(
        &mut store,
        &domains,
        &domain.systems(),
        2,
        7,
        &mut chronicle,
    )
    .expect("commits");
    let body_heat = |id: u64| {
        store
            .read(FactKey::new(e(id), BODY_HEAT))
            .and_then(|f| f.value.as_int())
    };
    assert_eq!(body_heat(ERIN), Some(3_700));
    assert!(chronicle.iter().all(|c| c.subject() != e(ERIN)));
    assert_ne!(
        body_heat(ALICE),
        Some(3_700),
        "while everyone else carries on"
    );
}
