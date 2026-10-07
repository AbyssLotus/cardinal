//! Fire (Vol. III Ch. 1 §1.9; Amendment A-28): the hay wagon alight against Alice's house takes
//! the kitchen and the bedroom above it, but not the granite cellar, and burns whoever stays
//! inside; a kept hearth lights the kitchen at night without burning it; a torch lit at a flame
//! sets the packing crate alight; rain puts out what burns in the open.

use kernel::fact::FactKey;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::WorldPackage;
use physical::schema::{BROKEN, BURNING, ENCLOSED, IN_VIEW};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const TORCH: u64 = 3009;

/// Ashford without minds, at ticks of `seconds`, its weather steady and dry.
fn ashford(seconds: u64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(seconds));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg
}

fn burning(city: &City, thing: u64) -> bool {
    city.read(thing, BURNING).is_some()
}

fn broken(city: &City, thing: u64) -> bool {
    city.read(thing, BROKEN) == Some(Value::Bool(true))
}

#[test]
fn the_hay_wagon_fire_takes_the_house_but_not_the_cellar() {
    let mut pkg = ashford(60);
    pkg.fires.push((WAGON, 3_000, false));
    let mut city = City::from(pkg);
    let (mut kitchen_caught, mut bedroom_caught, mut cellar_caught) = (None, None, false);
    let mut bob_burned = false;
    for _ in 0..8 * 60 {
        city.run(1);
        if kitchen_caught.is_none() && burning(&city, KITCHEN) {
            kitchen_caught = Some(city.tick);
        }
        if bedroom_caught.is_none() && burning(&city, BEDROOM) {
            bedroom_caught = Some(city.tick);
        }
        cellar_caught |= burning(&city, CELLAR);
        bob_burned |= city.int(BOB, living::schema::HEALTH).unwrap_or(10_000) < 10_000;
    }
    // Straw piled three metres high against the timber house: the kitchen wall catches, and the
    // bedroom over it.
    let kitchen = kitchen_caught.expect("the kitchen caught");
    let bedroom = bedroom_caught.expect("the bedroom caught");
    assert!(kitchen <= bedroom, "the kitchen, nearer, catches first");
    // Granite does not burn.
    assert!(!cellar_caught);
    // Burnt down: the wagon to wreckage, the kitchen to a ruin open to the sky.
    assert!(broken(&city, WAGON));
    assert!(broken(&city, KITCHEN), "the kitchen burned down");
    assert_eq!(city.read(KITCHEN, ENCLOSED), Some(Value::Bool(false)));
    assert!(!broken(&city, CELLAR));
    // Bob, with no mind of his own to flee, was burned where he stood.
    assert!(bob_burned);
    assert!(city
        .chronicle
        .iter()
        .any(|c| c.subject() == e(BOB) && c.cause().event().contains("burns")));
}

#[test]
fn a_kept_hearth_lights_the_kitchen_at_night_and_burns_nothing() {
    let sees_the_table = |kept: bool| {
        let mut pkg = ashford(1);
        if kept {
            pkg.fires.push((HEARTH, 3_000, true));
        }
        let mut city = City::from(pkg);
        city.run(5); // a few seconds past midnight: dark, but for the fire
        let seen = city
            .store()
            .read_all(FactKey::new(e(BOB), IN_VIEW))
            .iter()
            .any(|f| f.value == Value::Entity(e(TABLE)));
        (seen, city)
    };
    let (dark, _) = sees_the_table(false);
    let (lit, mut city) = sees_the_table(true);
    assert!(!dark, "in the dark Bob cannot see the table");
    assert!(lit, "by the hearth's light he can");
    // Half an hour by the fire: the hearth burns on, and nothing near it catches.
    city.run(30 * 60);
    assert!(burning(&city, HEARTH));
    for thing in [FLOUR_BIN, TABLE, KITCHEN] {
        assert!(!burning(&city, thing), "{thing} caught");
    }
    assert_eq!(
        city.read(HEARTH, physical::schema::INTEGRITY),
        None,
        "a kept fire eats nothing"
    );
}

#[test]
fn a_torch_lit_at_the_hearth_sets_the_crate_alight() {
    let mut pkg = ashford(10);
    pkg.fires.push((HEARTH, 3_000, true));
    let mut city = City::from(pkg);
    // No flame to hand: refused.
    city.go(ERIN, CRATE, 140);
    city.run_until(30, |c| !c.travelling(ERIN));
    city.ignite(ERIN, CRATE);
    city.run(2);
    assert_eq!(city.refused(ERIN), Some(CRATE));
    assert!(!burning(&city, CRATE));
    // The torch, taken to the hearth and lit there.
    city.go(ERIN, TORCH, 140);
    city.run_until(30, |c| !c.travelling(ERIN));
    city.take(ERIN, TORCH);
    city.run(2);
    city.go(ERIN, HEARTH, 140);
    city.run_until(60, |c| !c.travelling(ERIN));
    city.ignite(ERIN, TORCH);
    city.run(2);
    assert!(burning(&city, TORCH), "lit at the hearth");
    // Back out to the crate, and set it alight.
    city.go(ERIN, CRATE, 140);
    city.run_until(60, |c| !c.travelling(ERIN));
    city.ignite(ERIN, CRATE);
    city.run(2);
    assert!(burning(&city, CRATE));
    // It burns down to wreckage; its apple, too wet to burn, rolls out.
    city.run(2 * 360);
    assert!(broken(&city, CRATE), "burned down");
    assert!(!burning(&city, CRATE), "and out");
    assert_eq!(city.room_of(CRATE_APPLE), YARD);
    assert!(!burning(&city, CRATE_APPLE));
}

#[test]
fn rain_puts_out_a_fire_in_the_open_but_not_indoors() {
    let mut pkg = ashford(60);
    pkg.physical_rules.humidity_baseline = 9_500;
    pkg.fires.push((WAGON, 3_000, false));
    pkg.fires.push((TABLE, 3_000, false));
    let mut city = City::from(pkg);
    city.run(15);
    assert!(!burning(&city, WAGON), "rained out");
    assert!(!broken(&city, WAGON));
    assert!(
        burning(&city, TABLE),
        "under a roof the rain does not reach it"
    );
}

#[test]
fn the_same_fire_burns_the_same_way() {
    let run = || {
        let mut pkg = ashford(60);
        pkg.fires.push((WAGON, 3_000, false));
        let mut city = City::from(pkg);
        city.run(3 * 60);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}
