//! Fire in the air (Vol. III Ch. 1 §1.10; Amendment A-32): a kept hearth warms the kitchen; a fire
//! indoors fills the room with smoke, which clears faster the more doors stand open, shortens sight,
//! and harms whoever breathes it; and in the open the wind fans a fire fiercer than its fuel alone.

use kernel::fact::FactKey;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::model::Flag;
use packages::WorldPackage;
use physical::schema::{BURNING, IN_VIEW, SMOKE, TEMPERATURE};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const VALE: u64 = 1;

fn ashford(seconds: u64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(seconds));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg
}

/// Shut the kitchen's front door and its low window.
fn shut_up(pkg: &mut WorldPackage) {
    for f in &mut pkg.flags {
        if [1001, 1002, 1003, 1004].contains(&f.entity_id) {
            f.flags.push(Flag::Closed);
        }
    }
}

#[test]
fn a_kept_hearth_warms_the_kitchen_without_smoke() {
    let kitchen_after_six_hours = |kept: bool| {
        let mut pkg = ashford(60);
        if kept {
            pkg.fires.push((HEARTH, 3_000, true));
        }
        let mut city = City::from(pkg);
        city.run(6 * 60);
        (city.int(KITCHEN, TEMPERATURE).unwrap(), city)
    };
    let (cold, _) = kitchen_after_six_hours(false);
    let (warm, city) = kitchen_after_six_hours(true);
    assert!(
        warm - cold >= 300,
        "the hearth warmed it {} centidegrees",
        warm - cold
    );
    assert_eq!(
        city.read(KITCHEN, SMOKE),
        None,
        "its smoke goes up the chimney"
    );
}

#[test]
fn a_fire_indoors_fills_the_room_and_open_doors_clear_it() {
    let smoke_after = |shut: bool| {
        let mut pkg = ashford(10);
        if shut {
            shut_up(&mut pkg);
        }
        pkg.fires.push((TABLE, 3_000, false));
        let mut city = City::from(pkg);
        city.run(6 * 15); // a quarter of an hour
        city.int(KITCHEN, SMOKE).unwrap_or(0)
    };
    let open = smoke_after(false);
    let shut = smoke_after(true);
    assert!(open > 0, "smoke rises");
    assert!(shut > open, "shut in, it is thicker: {shut} against {open}");
}

#[test]
fn smoke_blinds_and_chokes() {
    // Midnight, the kitchen shut up, the table alight: by its light Bob sees the lamp, until the
    // smoke thickens; and breathing it, he is hurt.
    let mut pkg = ashford(10);
    shut_up(&mut pkg);
    pkg.fires.push((TABLE, 3_000, false));
    let mut city = City::from(pkg);
    let sees_lamp = |city: &City| {
        city.store()
            .read_all(FactKey::new(e(BOB), IN_VIEW))
            .iter()
            .any(|f| f.value == Value::Entity(e(LAMP)))
    };
    city.run(2);
    assert!(sees_lamp(&city), "by firelight");
    city.run(6 * 30);
    assert!(city.int(KITCHEN, SMOKE).unwrap_or(0) > 5_000);
    assert!(!sees_lamp(&city), "lost in the smoke");
    assert!(city.int(BOB, living::schema::HEALTH).unwrap() < 10_000);
}

#[test]
fn the_wind_fans_a_fire_in_the_open() {
    let fiercest = |wind: i64| {
        let mut pkg = ashford(60);
        if wind > 0 {
            pkg.storms.push((VALE, 0, 2, wind));
        }
        pkg.fires.push((WAGON, 3_000, false));
        let mut city = City::from(pkg);
        let mut most = 0;
        for _ in 0..40 {
            city.run(1);
            most = most.max(city.int(WAGON, BURNING).unwrap_or(0));
        }
        most
    };
    // Straw burns to 95% of full; the Vale's own breeze fans it a little past that, a 10 m/s
    // wind all the way.
    let breeze = fiercest(0);
    let windy = fiercest(1_000);
    assert!(breeze < 10_000, "in a breeze, {breeze}");
    assert_eq!(windy, 10_000, "fanned, it burns at full");
}
