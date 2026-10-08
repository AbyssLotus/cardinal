//! Taking from deposits (Vol. III Ch. 3; Amendment A-41): what is hard is worked only with a tool
//! no softer than it, go after go, until a unit comes away — sooner for a stronger arm and a
//! richer seam. Ore never regrows; timber felled below its floor never regrows either; wheat grows
//! only when it is warm and the ground is wet; a spring refills.

use kernel::fact::FactKey;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::model::PositionSpec;
use packages::WorldPackage;
use physical::schema::{CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};
use resources::schema::{COLLAPSED, STOCK, UNIT};

const IRON_SEAM: u64 = 3120;
const TIMBER: u64 = 3122;
const WHEAT: u64 = 3123;
const BRAMBLES: u64 = 3110;
const GALLERY: u64 = 401;
const WOOD: u64 = 603;
const IRON_ORE: u64 = 715;
const VALE: u64 = 1;

/// Ashford without minds at minute ticks, in steady weather.
fn ashford() -> WorldPackage {
    at_ticks(60)
}

/// As [`ashford`], at ticks of `seconds`: days pass faster at ten-minute ticks.
fn at_ticks(seconds: u64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(seconds));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.physical_rules.pressure_variability = 0;
    pkg
}

/// Put `who` in `place` at `at`, and the hammer in their hand if `hammer`.
fn stand(pkg: &mut WorldPackage, who: u64, place: u64, at: [i64; 2], hammer: bool) {
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
        x: at[0],
        y: at[1],
        z: Some(0),
    });
}

fn stock(city: &City, deposit: u64) -> i64 {
    city.int(deposit, STOCK).unwrap_or(0) / UNIT
}

/// What `who` holds that is made of `material`.
fn holding(city: &City, who: u64, material: u64) -> usize {
    city.store()
        .entities_with(CONTAINED_IN)
        .into_iter()
        .filter(|t| {
            city.store()
                .read(FactKey::new(*t, CONTAINED_IN))
                .map(|f| f.value)
                == Some(Value::Entity(e(who)))
        })
        .filter(|t| {
            city.store()
                .read_all(FactKey::new(*t, MADE_OF))
                .iter()
                .any(|m| m.value == Value::Entity(e(material)))
        })
        .count()
}

/// How many goes `who` takes at `deposit` before a unit of `material` comes away (at most 50).
fn goes_for_one(city: &mut City, who: u64, deposit: u64, material: u64) -> u32 {
    for go in 1..=50 {
        city.pick(who, deposit);
        city.run(2);
        if holding(city, who, material) > 0 {
            return go;
        }
    }
    51
}

#[test]
fn bare_hands_cannot_work_iron_ore_and_a_hammer_can() {
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, GALLERY, [300, 80], false);
    let mut city = City::from(pkg);
    city.pick(CAROL, IRON_SEAM);
    city.run(2);
    assert_eq!(
        city.refused(CAROL),
        Some(IRON_SEAM),
        "ore is too hard for a hand"
    );
    // With the hammer — iron, harder than the ore — a few blows bring a lump away.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
    let mut city = City::from(pkg);
    let goes = goes_for_one(&mut city, CAROL, IRON_SEAM, IRON_ORE);
    assert!((2..=20).contains(&goes), "{goes} blows");
    assert_eq!(stock(&city, IRON_SEAM), 1_999);
}

#[test]
fn a_stronger_arm_works_faster_and_a_richer_seam_too() {
    let goes = |who: u64, grade: i64| {
        let mut pkg = ashford();
        stand(&mut pkg, who, GALLERY, [300, 80], true);
        pkg.deposits
            .iter_mut()
            .find(|d| d.id == IRON_SEAM)
            .unwrap()
            .grade = Some(grade);
        let mut city = City::from(pkg);
        goes_for_one(&mut city, who, IRON_SEAM, IRON_ORE)
    };
    let carol = goes(CAROL, 6_000);
    let erin = goes(ERIN, 6_000);
    assert!(carol < erin, "Carol {carol} blows, Erin {erin}");
    let poor = goes(CAROL, 3_000);
    assert!(
        poor >= 2 * carol - 1,
        "half the grade, twice the work: {poor} against {carol}"
    );
}

#[test]
fn ore_never_regrows() {
    let mut pkg = at_ticks(600);
    stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
    let mut city = City::from(pkg);
    goes_for_one(&mut city, CAROL, IRON_SEAM, IRON_ORE);
    city.run(3 * 144);
    assert_eq!(stock(&city, IRON_SEAM), 1_999, "three days on");
}

#[test]
fn timber_felled_below_its_floor_never_regrows() {
    // The stand regrows a log a day in spring, down to its floor of ten. Felled to nine, it is
    // spent for good.
    let after_felling = |logs: u32| {
        let mut pkg = at_ticks(600);
        stand(&mut pkg, CAROL, WOOD, [500, 640], true);
        pkg.deposits
            .iter_mut()
            .find(|d| d.id == TIMBER)
            .unwrap()
            .stock = 11;
        let mut city = City::from(pkg);
        for _ in 0..logs {
            assert!(goes_for_one(&mut city, CAROL, TIMBER, 700) <= 50);
            // Put the log down, so the next one is counted afresh.
            let log = city
                .store()
                .entities_with(CONTAINED_IN)
                .into_iter()
                .find(|t| {
                    city.store()
                        .read(FactKey::new(*t, CONTAINED_IN))
                        .map(|f| f.value)
                        == Some(Value::Entity(e(CAROL)))
                        && *t != e(HAMMER)
                })
                .unwrap();
            city.drop_(CAROL, log.raw());
            city.run(2);
        }
        let felled = stock(&city, TIMBER);
        city.run(3 * 144);
        (
            felled,
            stock(&city, TIMBER),
            city.read(TIMBER, COLLAPSED).is_some(),
        )
    };
    let (felled, later, collapsed) = after_felling(1);
    assert_eq!(felled, 10);
    assert!(later >= 12 && !collapsed, "it regrows: {later}");
    let (felled, later, collapsed) = after_felling(2);
    assert_eq!(felled, 9);
    assert_eq!(later, 9, "spent");
    assert!(collapsed);
}

#[test]
fn wheat_grows_when_it_is_warm_and_wet() {
    // Three spring days: the Vale's wheat ripens as warm as it stands and as wet as its ground is,
    // by half again at least.
    // In frost, nothing; on dry ground, nothing.
    let after_three_days = |change: &dyn Fn(&mut WorldPackage)| {
        let mut pkg = at_ticks(600);
        change(&mut pkg);
        let mut city = City::from(pkg);
        city.run(3 * 144);
        stock(&city, WHEAT)
    };
    let start = at_ticks(600)
        .deposits
        .iter()
        .find(|d| d.id == WHEAT)
        .unwrap()
        .stock;
    let spring = after_three_days(&|_| {});
    let frost = after_three_days(&|p| {
        p.regions
            .iter_mut()
            .find(|r| r.id == VALE)
            .unwrap()
            .temperature_centi_c = -1_000;
    });
    let drought = after_three_days(&|p| p.ground_water.retain(|(c, _)| *c != VALE));
    assert!(spring >= start * 3 / 2, "it grew: {start} to {spring}");
    assert_eq!(frost, start, "not in frost");
    assert_eq!(drought, start, "not on dry ground");
}

#[test]
fn a_flowing_deposit_refills() {
    // Brambles declared to flow, as a spring does: picked bare, they are back to their full
    // eight at the next renewal.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, WOOD, [60, 80], false);
    pkg.deposits
        .iter_mut()
        .find(|d| d.id == BRAMBLES)
        .unwrap()
        .renewal = Some(4);
    let mut city = City::from(pkg);
    city.pick(CAROL, BRAMBLES);
    city.run(3);
    assert_eq!(stock(&city, BRAMBLES), 5);
    city.run(60);
    assert_eq!(stock(&city, BRAMBLES), 8);
}
