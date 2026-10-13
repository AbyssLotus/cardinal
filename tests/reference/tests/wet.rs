//! Wet and snow (Vol. III Ch. 1 §1.10, "soil moisture"; Amendment A-38): rain soaks the Vale's
//! ground and the sun dries it; snow lies in frost and melts into the ground in a thaw; what
//! stands in the rain is soaked, and soaked timber holds off a fire until the fire's heat dries
//! it; and wind and wet chill whoever stands out in them, but not those behind walls.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use packages::WorldPackage;
use physical::schema::{BURNING, GROUND_WATER, SNOW_DEPTH, WETNESS};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const VALE: u64 = 1;
const TORCH: u64 = 3009;
/// Minute ticks.
const HOUR: u64 = 60;

/// Ashford without minds at minute ticks, in steady weather — no wind — with the Vale's air at
/// `humidity`.
fn ashford(humidity: i64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg.physical_rules.humidity_baseline = humidity;
    // The ground begins dry, so what the weather does to it shows plainly (Ashford's own
    // begins damp: Amendment A-41).
    pkg.ground_water.clear();
    pkg
}

/// On the first tick, `by` added to `fact` of `entity`.
struct Douse {
    entity: u64,
    fact: FactType,
    by: i64,
}

const DOUSE_WRITES: &[FactType] = &[GROUND_WATER, SNOW_DEPTH, WETNESS];

impl System for Douse {
    fn id(&self) -> SystemId {
        SystemId::new("test.douse")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        DOUSE_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(self.entity), self.fact),
            ctx.basis_tick(),
            Change::Delta(self.by),
            Cause::new("doused"),
        )]
    }
}

fn int(city: &City, id: u64, fact: FactType) -> i64 {
    city.int(id, fact).unwrap_or(0)
}

#[test]
fn rain_soaks_the_ground_and_the_sun_dries_it() {
    // Three hours of 5 mm rain wet the Vale's ground 3% a millimetre.
    let mut city = City::from(ashford(9_500));
    city.run(3 * HOUR);
    let soaked = int(&city, VALE, GROUND_WATER);
    assert!((4_200..=4_600).contains(&soaked), "{soaked}");
    // Ground half wet, on a dry spring day: it dries, faster under the noon sun than at night.
    let mut city = City::from(ashford(6_000));
    city.attach(Box::new(Douse {
        entity: VALE,
        fact: GROUND_WATER,
        by: 5_000,
    }));
    city.run(6 * HOUR);
    let by_dawn = 5_000 - int(&city, VALE, GROUND_WATER);
    city.run(6 * HOUR);
    let at_noon = city.int(VALE, GROUND_WATER).unwrap();
    city.run(6 * HOUR);
    let by_afternoon = at_noon - int(&city, VALE, GROUND_WATER);
    assert!(by_dawn > 0, "it dries even at night");
    assert!(
        by_afternoon > by_dawn,
        "faster in the sun: {by_afternoon} > {by_dawn}"
    );
}

#[test]
fn snow_lies_in_frost_and_melts_in_a_thaw() {
    // Declared five below and saturated, the Vale is snowed on: 5 mm of water an hour lies as
    // 50 mm of snow, and the frozen ground takes none of it.
    let mut pkg = ashford(9_500);
    pkg.regions
        .iter_mut()
        .find(|r| r.id == VALE)
        .unwrap()
        .temperature_centi_c = -500;
    let mut city = City::from(pkg);
    city.run(2 * HOUR);
    let depth = int(&city, VALE, SNOW_DEPTH);
    assert!((95..=105).contains(&depth), "{depth} mm");
    assert_eq!(city.read(VALE, GROUND_WATER), None);
    // Twenty centimetres lying on the Vale's spring ground: it is gone within the morning, into
    // the ground.
    let mut city = City::from(ashford(6_000));
    city.attach(Box::new(Douse {
        entity: VALE,
        fact: SNOW_DEPTH,
        by: 200,
    }));
    city.run(6 * HOUR);
    assert_eq!(city.read(VALE, SNOW_DEPTH), None, "melted");
    assert!(int(&city, VALE, GROUND_WATER) > 3_000, "into the ground");
}

#[test]
fn what_stands_in_the_rain_is_soaked_and_what_is_indoors_is_not() {
    let mut city = City::from(ashford(9_500));
    city.run(HOUR);
    assert!(int(&city, CRATE, WETNESS) > 9_000, "the crate in the yard");
    assert_eq!(city.read(TABLE, WETNESS), None, "the kitchen table");
}

/// Whether the packing crate catches from the torch burning beside it within forty minutes,
/// soaked or dry.
fn crate_catches(soaked: bool) -> bool {
    let mut pkg = ashford(6_000);
    pkg.fires.push((TORCH, 3_000, false));
    let mut city = City::from(pkg);
    if soaked {
        city.attach(Box::new(Douse {
            entity: CRATE,
            fact: WETNESS,
            by: 10_000,
        }));
    }
    let mut caught = false;
    for _ in 0..40 {
        city.run(1);
        caught |= city.read(CRATE, BURNING).is_some();
    }
    caught
}

#[test]
fn soaked_timber_holds_off_a_fire_until_it_dries() {
    assert!(crate_catches(false), "dry, it catches from the torch");
    assert!(
        !crate_catches(true),
        "soaked, the torch's heat is still drying it"
    );
}

#[test]
fn wind_and_wet_chill_erin_and_spare_bob() {
    // Three hours: in still dry air; in a 20 m/s gale; in that gale and rain. Erin, out in the
    // yard, is colder each time; Bob, in the kitchen with the house shut up — no draught through
    // the door (Amendment A-37) — feels no wind.
    let after = |wind: i64, humidity: i64| {
        let mut pkg = ashford(humidity);
        reference::shut(&mut pkg, &reference::ALICE_OUTER);
        if wind > 0 {
            pkg.storms.push((VALE, 0, 4, wind));
        }
        let mut city = City::from(pkg);
        city.run(3 * HOUR);
        let heat = |who| city.int(who, living::schema::BODY_HEAT).unwrap();
        (heat(ERIN), heat(BOB))
    };
    let (still, bob_still) = after(0, 6_000);
    let (windy, bob_windy) = after(2_000, 6_000);
    let (soaked, _) = after(2_000, 9_500);
    assert!(
        windy < still - 100,
        "the gale chills: {windy} against {still}"
    );
    assert!(
        soaked < windy - 100,
        "and wet, more: {soaked} against {windy}"
    );
    assert_eq!(bob_windy, bob_still, "behind walls");
}
