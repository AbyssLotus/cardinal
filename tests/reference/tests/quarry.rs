//! Ground and quarries (Vol. III Ch. 1 §1.9; Vol. III Ch. 3; Amendment A-42): wherever the ground is
//! stone, a quarry can be cut — the first dig breaks ground, and the pit yields blocks to a tool
//! hard enough, blow by blow. Clay is dug by hand. There is no digging indoors, on cobbles, or in
//! the river. A pit dug bare is left, and the next dig breaks a new one beside it.

use kernel::fact::FactKey;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::model::PositionSpec;
use packages::WorldPackage;
use physical::schema::{CONTAINED_IN, MADE_OF, PIT};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};
use resources::schema::{STOCK, UNIT};

const MINE_MOUTH: u64 = 400;
const LANES: u64 = 150;
const BRACK: u64 = 605;
const GRANITE: u64 = 703;
const CLAY: u64 = 713;

/// Ashford without minds at minute ticks, in steady weather.
fn ashford() -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg
}

/// Put `who` in `place` at its middle, and the hammer in their hand if `hammer`.
fn stand(pkg: &mut WorldPackage, who: u64, place: u64, hammer: bool) {
    for c in &mut pkg.containment {
        if c.child_id == who {
            c.parent_id = place;
        }
        if hammer && c.child_id == HAMMER {
            c.parent_id = who;
        }
    }
    for o in &mut pkg.organisms {
        if o.id == who {
            o.region_id = place;
        }
    }
    pkg.positions.retain(|p| p.entity_id != who);
    pkg.positions.push(PositionSpec {
        entity_id: who,
        x: 0,
        y: 0,
        z: Some(0),
    });
}

/// How many things of `material` `who` holds.
fn holding(city: &City, who: u64, material: u64) -> usize {
    let s = city.store();
    s.entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            s.read(FactKey::new(*t, CONTAINED_IN)).map(|f| f.value) == Some(Value::Entity(e(who)))
        })
        .filter(|t| {
            s.read_all(FactKey::new(*t, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(material)))
        })
        .count()
}

fn pits(city: &City) -> Vec<kernel::identity::EntityId> {
    city.store().entities_with(PIT)
}

/// Dig until `who` holds `count` things of `material`, at most `goes` times; how many goes it took.
/// Each go: the order, the dig, the yield judged, the thing in hand.
fn dig_for(city: &mut City, who: u64, material: u64, count: usize, goes: u32) -> u32 {
    for go in 1..=goes {
        city.dig(who);
        city.run(4);
        if holding(city, who, material) >= count {
            return go;
        }
    }
    goes + 1
}

#[test]
fn a_hammer_cuts_granite_from_the_hill() {
    // At the mine mouth, on the Hill's granite: the first dig breaks ground, and the pit gives a
    // block for every few blows of the hammer.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, MINE_MOUTH, true);
    let mut city = City::from(pkg);
    let goes = dig_for(&mut city, CAROL, GRANITE, 1, 30);
    assert!((2..=30).contains(&goes), "{goes} goes");
    let pit = pits(&city);
    assert_eq!(pit.len(), 1, "one pit");
    assert_eq!(
        city.store()
            .read(FactKey::new(pit[0], CONTAINED_IN))
            .map(|f| f.value),
        Some(Value::Entity(e(MINE_MOUTH)))
    );
    assert_eq!(city.int(pit[0].raw(), STOCK), Some(4_999 * UNIT));
}

#[test]
fn clay_is_dug_by_hand_and_granite_is_not() {
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, MINE_MOUTH, false);
    let mut city = City::from(pkg);
    city.dig(CAROL);
    city.run(3);
    assert_eq!(
        city.refused(CAROL),
        Some(MINE_MOUTH),
        "granite is too hard for a hand"
    );
    assert!(pits(&city).is_empty(), "no ground broken");
    // Erin, in the yard: the Vale's clay comes up by the handful.
    let mut city = City::from(ashford());
    let goes = dig_for(&mut city, ERIN, CLAY, 1, 5);
    assert_eq!(
        goes, 2,
        "the first dig breaks ground, the second brings up clay"
    );
}

#[test]
fn no_digging_indoors_on_the_cobbles_or_in_the_river() {
    for (place, why) in [
        (KITCHEN, "a floor"),
        (LANES, "the cobbles"),
        (BRACK, "the river"),
    ] {
        let mut pkg = ashford();
        stand(&mut pkg, ERIN, place, false);
        let mut city = City::from(pkg);
        city.dig(ERIN);
        city.run(3);
        assert_eq!(city.refused(ERIN), Some(place), "{why}");
        assert!(pits(&city).is_empty(), "{why}");
    }
}

#[test]
fn a_pit_dug_bare_is_left_for_a_new_one() {
    // Pits in clay that hold two spadefuls: the third dig breaks a second pit beside the first.
    let mut pkg = ashford();
    for q in &mut pkg.quarries {
        if q.0 == CLAY {
            q.1 = 2;
        }
    }
    let mut city = City::from(pkg);
    assert!(dig_for(&mut city, ERIN, CLAY, 2, 6) <= 6);
    assert_eq!(pits(&city).len(), 1);
    assert!(dig_for(&mut city, ERIN, CLAY, 3, 6) <= 6);
    assert_eq!(pits(&city).len(), 2, "a second pit");
}
