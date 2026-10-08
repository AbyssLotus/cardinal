//! Air between places (Vol. III Ch. 1 §1.10, "smoke diffuses"; Amendment A-37): air trades
//! through every open opening. An open door lets the night into Alice's kitchen; a hearth's
//! warmth climbs the stairs to the bedroom; the burning table's smoke drifts up them; and a fire
//! out in the yard smokes the still air over it, which drifts in at the open door — and a wind
//! carries off.

use packages::WorldPackage;
use physical::schema::{SMOKE, TEMPERATURE};
use reference::id::*;
use reference::{with_tick_seconds, without_minds, City};

const VALE: u64 = 1;
const CELLAR: u64 = 102;
/// Minute ticks.
const MINUTE: u64 = 1;
const HOUR: u64 = 60;

/// Ashford without minds at minute ticks, in steady weather: no wind.
fn ashford() -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg
}

/// Shut every way from Alice's house to the yard: the front door, the low window, the bulkhead.
fn shut_up(pkg: &mut WorldPackage) {
    reference::shut(pkg, &reference::ALICE_OUTER);
}

fn int(city: &City, id: u64, fact: kernel::fact::FactType) -> i64 {
    city.int(id, fact).unwrap_or(0)
}

#[test]
fn an_open_door_lets_the_night_in() {
    // Alice's kitchen begins the night at 20 °C, warm from the stove; the yard is at 16. Two
    // hours on, with the front door and the low window open, it has given up its warmth to the
    // night far faster than shut up, where only its timber walls let it go.
    let kitchen_after = |shut: bool| {
        let mut pkg = ashford();
        if shut {
            shut_up(&mut pkg);
        }
        let mut city = City::from(pkg);
        city.run(2 * HOUR);
        int(&city, KITCHEN, TEMPERATURE) - int(&city, VALE, TEMPERATURE)
    };
    let open = kitchen_after(false);
    let shut = kitchen_after(true);
    assert!(
        open >= 0 && shut > open + 50,
        "above the yard: open {open}, shut {shut}"
    );
}

#[test]
fn a_hearths_warmth_climbs_the_stairs() {
    // The house shut up for the night, a kept fire in the kitchen hearth: in six hours the warmth
    // has climbed the open stair to the bedroom and gone down to the cellar.
    let after_six_hours = |hearth: bool| {
        let mut pkg = ashford();
        shut_up(&mut pkg);
        if hearth {
            pkg.fires.push((HEARTH, 3_000, true));
        }
        let mut city = City::from(pkg);
        city.run(6 * HOUR);
        [BEDROOM, CELLAR].map(|room| int(&city, room, TEMPERATURE))
    };
    let cold = after_six_hours(false);
    let warm = after_six_hours(true);
    assert!(
        warm[0] - cold[0] >= 100,
        "the bedroom: {cold:?} -> {warm:?}"
    );
    assert!(warm[1] - cold[1] >= 100, "the cellar: {cold:?} -> {warm:?}");
}

#[test]
fn the_tables_smoke_drifts_up_the_stairs() {
    let mut pkg = ashford();
    shut_up(&mut pkg);
    pkg.fires.push((TABLE, 3_000, false));
    let mut city = City::from(pkg);
    city.run(15 * MINUTE);
    let kitchen = int(&city, KITCHEN, SMOKE);
    let bedroom = int(&city, BEDROOM, SMOKE);
    assert!(bedroom > 0, "upstairs too");
    assert!(
        kitchen > bedroom,
        "thickest where it burns: {kitchen} > {bedroom}"
    );
}

/// The smoke over the yard and in the kitchen after twenty minutes of the packing crate burning
/// in the yard, the house open or shut, in a wind of `wind` cm/s.
fn crate_fire(shut: bool, wind: i64) -> (i64, i64) {
    let mut pkg = ashford();
    if shut {
        shut_up(&mut pkg);
    }
    if wind > 0 {
        pkg.storms.push((VALE, 0, 2, wind));
    }
    pkg.fires.push((CRATE, 5_000, false));
    let mut city = City::from(pkg);
    city.run(20 * MINUTE);
    (int(&city, YARD, SMOKE), int(&city, KITCHEN, SMOKE))
}

#[test]
fn a_fire_in_the_yard_smokes_the_still_air_and_it_drifts_indoors() {
    let (yard, kitchen) = crate_fire(false, 0);
    assert!(yard > 0, "the yard is smoky");
    assert!(
        kitchen > 0 && kitchen < yard,
        "in at the open door: {kitchen} of {yard}"
    );
    // Shut up, the house keeps it out.
    let (_, shut) = crate_fire(true, 0);
    assert_eq!(shut, 0);
}

#[test]
fn a_wind_carries_the_smoke_off_the_yard() {
    let (still, _) = crate_fire(false, 0);
    let (windy, _) = crate_fire(false, 500);
    assert!(windy * 4 < still, "a 5 m/s wind: {windy} against {still}");
}
