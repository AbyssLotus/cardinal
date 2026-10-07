//! Wind (Vol. III Ch. 1 §1.9, §1.10; Amendment A-29): a gale over the Vale tears down the timber
//! shed and Alice's timber house, but not the granite manor; a lesser wind strips the straw from the
//! hay wagon and leaves the shed standing. What a storm leaves of a house is a ruin, and what was
//! sheltered inside it is in the open from then on. Light, loose things are carried off by the
//! wind or anyone's shove, and a deep enough low brews a gale of its own (Amendment A-33).

use kernel::store::RealityStore;
use kernel::value::Value;
use packages::WorldPackage;
use physical::schema::{BROKEN, INTEGRITY, WIND_SPEED};
use reference::id::*;
use reference::{with_tick_seconds, without_minds, City};

/// The Vale.
const VALE: u64 = 1;
/// The manor's hall.
const HALL: u64 = 301;

/// Ashford without minds at minute ticks, steady weather, and a storm over the Vale from the
/// first hour for `hours`, at `wind` cm/s.
fn storm(hours: i64, wind: i64) -> WorldPackage {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg.physical_rules.humidity_variability = 0;
    pkg.storms.push((VALE, 1, hours, wind));
    pkg
}

fn broken(city: &City, thing: u64) -> bool {
    city.read(thing, BROKEN) == Some(Value::Bool(true))
}

#[test]
fn a_gale_tears_down_timber_but_not_granite() {
    // A 40 m/s gale for four hours presses at 50%: past timber's 30%, short of granite's 70%.
    let mut city = City::from(storm(4, 4_000));
    city.run(90);
    assert_eq!(city.int(VALE, WIND_SPEED), Some(4_000), "the gale blows");
    city.run(5 * 60);
    assert!(broken(&city, SHED), "the shed is down");
    assert!(broken(&city, KITCHEN), "Alice's house is a ruin");
    assert!(broken(&city, CRATE));
    // Granite stands untouched, and so does what it shelters.
    assert!(!broken(&city, HALL));
    assert_eq!(city.read(HALL, INTEGRITY), None);
    // Iron and glass bear it.
    assert!(!broken(&city, HAMMER));
    assert!(!broken(&city, BOTTLE));
    // The storm has passed.
    assert_ne!(city.int(VALE, WIND_SPEED), Some(4_000));
}

#[test]
fn what_the_walls_sheltered_is_in_the_open_once_they_fall() {
    // The kitchen table stood out the gale behind the kitchen's walls; once they fell, it bore it.
    let mut city = City::from(storm(6, 4_000));
    let mut walls_fell = None;
    let mut table_hit = None;
    for _ in 0..7 * 60 {
        city.run(1);
        if walls_fell.is_none() && broken(&city, KITCHEN) {
            walls_fell = Some(city.tick);
        }
        if table_hit.is_none() && city.read(TABLE, INTEGRITY).is_some() {
            table_hit = Some(city.tick);
        }
    }
    let fell = walls_fell.expect("the kitchen fell");
    let hit = table_hit.expect("the table bore the wind");
    assert!(hit > fell, "sheltered until the walls fell");
}

#[test]
fn a_lesser_wind_strips_the_straw_and_spares_the_shed() {
    // 25 m/s presses at about 20%: past straw, short of timber.
    let mut city = City::from(storm(4, 2_500));
    city.run(6 * 60);
    assert!(broken(&city, WAGON), "the hay is scattered");
    assert!(!broken(&city, SHED));
    assert_eq!(city.read(SHED, INTEGRITY), None, "not a mark on it");
}

#[test]
fn the_same_storm_does_the_same_damage() {
    let run = || {
        let mut city = City::from(storm(4, 4_000));
        city.run(3 * 60);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}

#[test]
fn a_gale_carries_off_what_is_too_light_to_hold_its_place() {
    // The torch, light and loose, is blown across the yard; the crate, solid, holds its ground.
    let mut city = City::from(storm(2, 4_000));
    let at = |city: &City, thing: u64| {
        physical::space::local_position(city.store(), reference::e(thing))
    };
    let (torch, crate_at) = (at(&city, 3009), at(&city, CRATE));
    city.run(60 + 30);
    let moved = |a: [i64; 3], b: [i64; 3]| {
        let (dx, dy) = ((a[0] - b[0]) as i128, (a[1] - b[1]) as i128);
        kernel::fixed::isqrt((dx * dx + dy * dy) as u128) as i64
    };
    assert!(
        moved(torch, at(&city, 3009)) >= 1_000,
        "the torch blew away"
    );
    assert_eq!(at(&city, CRATE), crate_at, "the crate stood fast");
    assert_eq!(city.room_of(3009), YARD, "still in the yard");
}

/// A shove: a push given the bottle, once, by anyone (Amendment A-33).
struct Shove;

impl kernel::system::System for Shove {
    fn id(&self) -> kernel::fact::SystemId {
        kernel::fact::SystemId::new("test.shove")
    }
    fn reads(&self) -> &'static [kernel::fact::FactType] {
        &[]
    }
    fn writes(&self) -> &'static [kernel::fact::FactType] {
        &[physical::schema::ACT_PUSH]
    }
    fn cadence(&self) -> kernel::system::Cadence {
        kernel::system::Cadence::EveryTick
    }
    fn evaluate(
        &self,
        _view: &dyn kernel::system::CommittedView,
        ctx: &kernel::system::TickContext,
    ) -> Vec<kernel::proposal::Proposal> {
        if ctx.tick() != 1 {
            return Vec::new();
        }
        vec![kernel::proposal::Proposal::new(
            self.id(),
            kernel::fact::FactKey::new(reference::e(BOTTLE), physical::schema::ACT_PUSH),
            ctx.basis_tick(),
            kernel::proposal::Change::Set(Value::Vec3([150, -40, 0])),
            kernel::fact::Cause::new("shoved"),
        )]
    }
}

#[test]
fn a_push_is_anyones_to_give() {
    let mut pkg = without_minds(with_tick_seconds(60));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    let mut city = City::from(pkg);
    let before = physical::space::local_position(city.store(), reference::e(BOTTLE));
    city.attach(Box::new(Shove));
    city.run(3);
    let after = physical::space::local_position(city.store(), reference::e(BOTTLE));
    assert_eq!(
        [after[0] - before[0], after[1] - before[1]],
        [150, -40],
        "moved as it was pushed"
    );
    assert_eq!(
        city.read(BOTTLE, physical::schema::ACT_PUSH),
        None,
        "and the push spent"
    );
}

#[test]
fn a_deep_low_brews_its_own_gale() {
    // Weather that swings hard, and a world where a low 50 daPa under normal blows: within two
    // days, a gale with no storm declared. Ashford's own weather, in the same two days, does not.
    let strongest = |swinging: bool| {
        let mut pkg = without_minds(with_tick_seconds(600));
        if swinging {
            pkg.physical_rules.pressure_variability = 300;
            pkg.physical_rules.gale_below = 50;
        }
        let mut city = City::from(pkg);
        let mut most = 0;
        for _ in 0..2 * 144 {
            city.run(1);
            most = most.max(city.int(VALE, WIND_SPEED).unwrap_or(0));
        }
        most
    };
    assert!(strongest(true) >= 1_000, "{}", strongest(true));
    assert!(strongest(false) < 1_000, "{}", strongest(false));
}
