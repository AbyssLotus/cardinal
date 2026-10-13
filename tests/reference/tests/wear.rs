//! Use wears things (Amendment A-46): every go of work wears the tool that does it, the more for
//! harder stuff, and a tool worn to nothing is gone. A millstone wears while it grinds, and one
//! worn through grinds nothing. Bare hands mend what bare hands work. A mind lets a little wear
//! be, but mends what is worn past its world's line — and once begun, mends it whole.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::model::{BodySpec, ContainmentSpec, MadeOfSpec, PositionSpec};
use packages::WorldPackage;
use physical::schema::{BROKEN, CONTAINED_IN, INTEGRITY};
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, without_minds, City};

const GALLERY: u64 = 401;
const WOOD: u64 = 603;
const MILL: u64 = 604;
const IRON_SEAM: u64 = 3120;
const TIMBER_STAND: u64 = 3122;
const MILLSTONE: u64 = 620;
const OVEN: u64 = 621;
const GRIND: u64 = 951;
const WHEAT: u64 = 717;
const GRANITE: u64 = 703;
const CLAY: u64 = 713;
const BAKER: u64 = 2063;

/// Ashford without minds at minute ticks, in steady weather.
fn ashford() -> WorldPackage {
    at_ticks(60)
}

/// As [`ashford`], at ticks of `seconds`.
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
    place_at(pkg, who, at);
}

fn place_at(pkg: &mut WorldPackage, thing: u64, at: [i64; 2]) {
    pkg.positions.retain(|p| p.entity_id != thing);
    pkg.positions.push(PositionSpec {
        entity_id: thing,
        x: at[0],
        y: at[1],
        z: Some(0),
    });
}

/// `count` loose pieces of `material`, `size` half-extents, in `place` around `at`, ids from
/// `first`.
fn lay(
    pkg: &mut WorldPackage,
    first: u64,
    count: u64,
    material: u64,
    size: [i64; 3],
    place: u64,
    at: [i64; 2],
) {
    for i in 0..count {
        let id = first + i;
        pkg.containment.push(ContainmentSpec {
            child_id: id,
            parent_id: place,
        });
        place_at(pkg, id, [at[0] + 10 * i as i64, at[1]]);
        pkg.bodies.push(BodySpec {
            entity_id: id,
            half_width: size[0],
            half_depth: size[1],
            height: size[2],
        });
        pkg.made_of.push(MadeOfSpec {
            object_id: id,
            material_id: material,
        });
    }
}

/// Takes `by` from `thing`'s integrity at the first tick.
struct Knock {
    thing: u64,
    by: i64,
}

const KNOCK_WRITES: &[FactType] = &[INTEGRITY];

impl System for Knock {
    fn id(&self) -> SystemId {
        SystemId::new("test.knock")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        KNOCK_WRITES
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
            FactKey::new(e(self.thing), INTEGRITY),
            ctx.basis_tick(),
            Change::Delta(-self.by),
            Cause::new("knocked"),
        )]
    }
}

fn worn(city: &City, thing: u64) -> i64 {
    10_000 - city.int(thing, INTEGRITY).unwrap_or(10_000)
}

#[test]
fn a_tool_wears_with_each_go_and_faster_on_harder_stuff() {
    // A minute of the hammer at the iron seam — twelve five-second blows (Amendment A-48) — and a
    // minute at the stand of timber. Ore is twice as hard as timber against iron: twice the wear.
    let a_minute = |place: u64, at: [i64; 2], deposit: u64| {
        let mut pkg = ashford();
        stand(&mut pkg, CAROL, place, at, true);
        let mut city = City::from(pkg);
        city.pick(CAROL, deposit);
        city.run(3);
        worn(&city, HAMMER)
    };
    let ore = a_minute(GALLERY, [300, 80], IRON_SEAM);
    let timber = a_minute(WOOD, [500, 640], TIMBER_STAND);
    // The world's rule: 0.3% a go on stuff as hard as iron; ore is two-thirds as hard, timber a
    // third.
    assert_eq!(ore, 12 * 30 * 6_000 / 9_000, "ore");
    assert_eq!(timber, 12 * 30 * 3_000 / 9_000, "timber");
}

#[test]
fn a_tool_worn_to_nothing_is_gone() {
    // A hammer nearly worn out: one more go at the seam spends it. It is nowhere now, not wreckage
    // in her hand, and the next go is refused: ore is too hard for a hand.
    let mut pkg = ashford();
    stand(&mut pkg, CAROL, GALLERY, [300, 80], true);
    let mut city = City::from(pkg);
    city.attach(Box::new(Knock {
        thing: HAMMER,
        by: 9_990,
    }));
    city.run(1);
    city.pick(CAROL, IRON_SEAM);
    city.run(2);
    assert_eq!(city.read(HAMMER, CONTAINED_IN), None, "worn out");
    assert_eq!(city.read(HAMMER, BROKEN), None, "spent, not broken");
    city.pick(CAROL, IRON_SEAM);
    city.run(2);
    assert_eq!(city.refused(CAROL), Some(IRON_SEAM));
}

/// Ashford with a bin of wheat beside the miller, his millstone wearing at `per_hour`.
fn mill(per_hour: i64) -> WorldPackage {
    mill_at(60, per_hour)
}

/// As [`mill`], at ticks of `seconds`.
fn mill_at(seconds: u64, per_hour: i64) -> WorldPackage {
    let mut pkg = at_ticks(seconds);
    pkg.wear.retain(|(w, _)| *w != MILLSTONE);
    pkg.wear.push((MILLSTONE, per_hour));
    for c in &mut pkg.containment {
        if c.child_id == FLOUR_BIN {
            c.parent_id = MILL;
        }
    }
    place_at(&mut pkg, FLOUR_BIN, [0, 80]);
    let bin = pkg.deposits.iter_mut().find(|d| d.id == FLOUR_BIN).unwrap();
    bin.made_of = WHEAT;
    pkg
}

/// The miller takes two sheaves from the bin.
fn sheaves(city: &mut City) {
    for _ in 0..2 {
        city.pick(MILLER, FLOUR_BIN);
        city.run(3);
    }
}

#[test]
fn the_millstone_wears_while_it_grinds() {
    // Worn 6% an hour in use, ten minutes' grinding takes 1% from it. Standing idle, it does not
    // wear.
    let mut city = City::from(mill(600));
    city.run(60);
    assert_eq!(worn(&city, MILLSTONE), 0, "idle");
    sheaves(&mut city);
    city.make(MILLER, GRIND);
    city.run(14);
    assert!(
        (95..=105).contains(&worn(&city, MILLSTONE)),
        "{}",
        worn(&city, MILLSTONE)
    );
}

#[test]
fn the_same_wear_at_any_tick_length() {
    // An hour's grinding at minute ticks and at ten-second ticks wears the stone alike.
    let at = |seconds: u64| {
        let mut city = City::from(mill_at(seconds, 600));
        let ticks = |minutes: u64| minutes * 60 / seconds;
        city.run(ticks(60));
        for _ in 0..2 {
            city.pick(MILLER, FLOUR_BIN);
            city.run(ticks(3));
        }
        city.make(MILLER, GRIND);
        city.run(ticks(14));
        worn(&city, MILLSTONE)
    };
    let (minute, fine) = (at(60), at(10));
    assert!((minute - fine).abs() <= 10, "{minute} against {fine}");
}

#[test]
fn a_millstone_worn_through_grinds_nothing() {
    // A millstone all but worn through breaks in its next grinding, and the miller can grind on
    // it no more.
    let mut pkg = mill(600);
    pkg.deposits
        .iter_mut()
        .find(|d| d.id == FLOUR_BIN)
        .unwrap()
        .stock = 20;
    let mut city = City::from(pkg);
    city.attach(Box::new(Knock {
        thing: MILLSTONE,
        by: 9_950,
    }));
    city.run(60);
    sheaves(&mut city);
    city.make(MILLER, GRIND);
    city.run(14);
    assert_eq!(city.read(MILLSTONE, BROKEN), Some(Value::Bool(true)));
    sheaves(&mut city);
    city.make(MILLER, GRIND);
    city.run(3);
    assert_eq!(
        city.read(MILLER, economy::schema::MAKE_REFUSED),
        Some(Value::Entity(e(GRIND)))
    );
}

#[test]
fn bare_hands_mend_clay() {
    // The oven cracked, 3% of it; three lumps of clay beside it, fourteen kilos each. The baker,
    // holding nothing, works them in by hand: a minute's mending — twelve goes (Amendment A-48) —
    // would restore it all, and uses all the clay there is for what it allows; the next minute is
    // refused, the clay gone.
    let mut pkg = ashford();
    stand(&mut pkg, BAKER, MILL, [-160, -160], false);
    lay(&mut pkg, 9_100, 3, CLAY, [10, 10, 20], MILL, [-150, -120]);
    let mut city = City::from(pkg);
    city.attach(Box::new(Knock {
        thing: OVEN,
        by: 300,
    }));
    city.run(2);
    assert_eq!(worn(&city, OVEN), 300);
    city.mend(BAKER, OVEN);
    city.run(2);
    assert_eq!(city.refused(BAKER), None, "by hand");
    // 43 kilos of clay in a 3,110-kilo oven: 1.39% of it.
    assert!(
        (159..=163).contains(&worn(&city, OVEN)),
        "{}",
        worn(&city, OVEN)
    );
    city.mend(BAKER, OVEN);
    city.run(2);
    assert_eq!(city.refused(BAKER), Some(OVEN), "no clay left");
}

#[test]
fn a_little_wear_is_let_be_and_more_is_mended_whole() {
    // The millstone is the miller's. Granite blocks and the hammer lie in the mill. Worn 3%, he
    // leaves it; worn 8%, past his world's 5% line, he mends it — and goes on until it is whole,
    // some two dozen ten-kilo blocks later.
    let after = |by: i64| {
        let mut pkg = well_fed(with_tick_seconds(60));
        pkg.owners.push((MILLSTONE, MILLER));
        pkg.roles.retain(|(w, _)| *w != MILLER);
        lay(&mut pkg, 9_200, 30, GRANITE, [12, 10, 8], MILL, [-150, 200]);
        for c in &mut pkg.containment {
            if c.child_id == HAMMER {
                c.parent_id = MILL;
            }
        }
        place_at(&mut pkg, HAMMER, [100, 150]);
        let mut city = City::from(pkg);
        city.attach(Box::new(Knock {
            thing: MILLSTONE,
            by,
        }));
        city.run(12 * 60);
        worn(&city, MILLSTONE)
    };
    assert_eq!(after(300), 300, "let be");
    assert_eq!(after(800), 0, "mended whole");
}
