//! Work that keeps time (Amendment A-48): a go of work lasts five seconds, and a tick of work is
//! as many goes as fit — so a minute's picking brings the same apples, and ten minutes at the seam
//! the same ore, at any tick length. The hands hold no more than the body can carry, all it holds
//! counted together; full, they are refused. A load is set down in one go.

use kernel::fact::FactKey;
use kernel::system::CommittedView;
use kernel::value::Value;
use packages::model::PositionSpec;
use packages::WorldPackage;
use physical::schema::{CONTAINED_IN, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};
use resources::schema::{STOCK, UNIT};

const GALLERY: u64 = 401;
const IRON_SEAM: u64 = 3120;
const IRON_ORE: u64 = 715;
const APPLE: u64 = 704;

/// Ashford without minds, in steady weather, at ticks of `seconds`.
fn ashford(seconds: u64) -> WorldPackage {
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

/// What `who` holds that is made of `material`.
fn holding(city: &City, who: u64, material: u64) -> Vec<u64> {
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
        .map(|t| t.raw())
        .collect()
}

/// `who` works `deposit` for `seconds`, a tick's work at a time, asking for as much as comes —
/// each tick's work seen to before the next, as a mind does (Amendment A-48).
fn work_for(city: &mut City, who: u64, deposit: u64, seconds: u64, tick_seconds: u64) {
    for _ in 0..seconds / tick_seconds {
        city.pick_up_to(who, deposit, 1_000);
        city.run(4);
    }
}

#[test]
fn a_minutes_picking_is_a_minute_at_any_tick_length() {
    // Carol at the apple tree, laden with thirty: a minute's picking is twelve handfuls, at
    // five-second ticks or at one-minute ticks.
    let apples = |tick_seconds: u64| {
        let mut pkg = ashford(tick_seconds);
        pkg.deposits
            .iter_mut()
            .find(|d| d.id == TREE)
            .unwrap()
            .stock = 30;
        stand(&mut pkg, CAROL, YARD, [0, 400], false);
        let tree = pkg.positions.iter().find(|p| p.entity_id == TREE).unwrap();
        let at = [tree.x, tree.y - 80];
        stand(&mut pkg, CAROL, YARD, at, false);
        let mut city = City::from(pkg);
        work_for(&mut city, CAROL, TREE, 60, tick_seconds);
        holding(&city, CAROL, APPLE).len()
    };
    assert_eq!(apples(60), 12);
    assert_eq!(apples(5), 12);
}

#[test]
fn ten_minutes_at_the_seam_are_ten_minutes_at_any_tick_length() {
    // Carol with the hammer: what ten minutes at the iron seam bring away is the same at
    // one-minute and ten-minute ticks — as much as her hands can hold, either way.
    let ore = |tick_seconds: u64| {
        let mut pkg = ashford(tick_seconds);
        stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
        let mut city = City::from(pkg);
        work_for(&mut city, CAROL, IRON_SEAM, 600, tick_seconds);
        (
            holding(&city, CAROL, IRON_ORE).len(),
            2_000 - city.int(IRON_SEAM, STOCK).unwrap_or(0) / UNIT,
        )
    };
    let (minute, taken) = ore(60);
    assert!(minute >= 1, "ore at minute ticks");
    assert_eq!(ore(600), (minute, taken), "the same at ten-minute ticks");
}

#[test]
fn the_hands_hold_so_much_and_full_are_refused() {
    // Lumps of ore are three and a half kilos; with the hammer, six fit in a 25-kilo carry. An
    // hour at the seam brings six, and then the hands are full: refused.
    let mut pkg = ashford(60);
    stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
    let mut city = City::from(pkg);
    work_for(&mut city, CAROL, IRON_SEAM, 3_600, 60);
    assert_eq!(holding(&city, CAROL, IRON_ORE).len(), 6);
    city.pick_up_to(CAROL, IRON_SEAM, 1_000);
    city.run(3);
    assert_eq!(city.refused(CAROL), Some(CAROL), "her hands are full");
}

#[test]
fn a_load_is_set_down_in_one_go() {
    // Six lumps of ore put down together, in one tick, where she stands.
    let mut pkg = ashford(60);
    stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
    let mut city = City::from(pkg);
    work_for(&mut city, CAROL, IRON_SEAM, 3_600, 60);
    let load = holding(&city, CAROL, IRON_ORE);
    assert_eq!(load.len(), 6);
    for lump in &load {
        city.drop_(CAROL, *lump);
    }
    city.run(2);
    assert!(holding(&city, CAROL, IRON_ORE).is_empty(), "all set down");
    for lump in &load {
        assert_eq!(
            city.read(*lump, CONTAINED_IN),
            Some(Value::Entity(e(GALLERY)))
        );
    }
}
