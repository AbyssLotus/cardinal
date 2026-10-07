//! Open ground (Vol. III Ch. 1 §1.5; Amendment A-30): a place without walls — a garden plot, a
//! street, a ruin — is walked into and out of anywhere along its edge, not only through its
//! openings. A walled room is still entered only by its doors.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use packages::model::{BodySpec, PositionSpec};
use packages::WorldPackage;
use physical::schema::{INTEGRITY, TRAVEL_BLOCKED};
use physical::space::local_position;
use reference::id::*;
use reference::{e, without_minds, City};

/// A garden plot in the yard, 4 m square and open on every side, and a spot in the yard with no
/// size at all — both places, neither walled.
const PLOT: u64 = 160;
const SPOT: u64 = 161;

fn ashford() -> WorldPackage {
    let mut pkg = without_minds(reference::package());
    pkg.places.push((PLOT, Some(YARD)));
    pkg.places.push((SPOT, Some(YARD)));
    for (id, x, y) in [(PLOT, -600, 400), (SPOT, 600, -200)] {
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x,
            y,
            z: Some(0),
        });
    }
    pkg.bodies.push(BodySpec {
        entity_id: PLOT,
        half_width: 200,
        half_depth: 200,
        height: 10,
    });
    // The kitchen's front door is shut.
    for f in &mut pkg.flags {
        if f.entity_id == 1001 || f.entity_id == 1002 {
            f.flags.push(packages::model::Flag::Closed);
        }
    }
    pkg
}

fn walk(city: &mut City, who: u64, to: u64) -> u64 {
    city.go(who, to, 140);
    city.run_until(120, |c| !c.travelling(who))
}

fn crossed(city: &City, who: u64, how: &str) -> bool {
    city.chronicle
        .iter()
        .any(|c| c.subject() == e(who) && c.cause().event() == how)
}

#[test]
fn an_open_plot_is_walked_into_from_any_side_and_out_again() {
    let mut city = City::from(ashford());
    walk(&mut city, ERIN, PLOT);
    assert_eq!(city.room_of(ERIN), PLOT);
    assert!(
        crossed(&city, ERIN, "entered"),
        "across its edge, not by a door"
    );
    // She stands within it, at the edge she came to.
    let [x, y, _] = local_position(city.store(), e(ERIN));
    assert!(x.abs() <= 200 && y.abs() <= 200, "at {x}, {y}");
    walk(&mut city, ERIN, YARD);
    assert_eq!(city.room_of(ERIN), YARD);
    assert!(crossed(&city, ERIN, "left"));
}

#[test]
fn a_place_with_no_size_is_walked_to_and_entered() {
    let mut city = City::from(ashford());
    walk(&mut city, ERIN, SPOT);
    assert_eq!(city.room_of(ERIN), SPOT);
}

#[test]
fn a_walled_room_is_still_entered_only_by_its_doors() {
    // The front door is shut and the kitchen's walls stand: no way in.
    let mut city = City::from(ashford());
    city.go(ERIN, KITCHEN, 140);
    city.run(5);
    assert_eq!(city.read(ERIN, TRAVEL_BLOCKED), Some(Value::Bool(true)));
    assert_eq!(city.room_of(ERIN), YARD);
}

/// The kitchen's walls brought down on the first tick (Appendix A, Ruling 9).
struct Wrecker;

const WRECKER_WRITES: &[FactType] = &[INTEGRITY];

impl System for Wrecker {
    fn id(&self) -> SystemId {
        SystemId::new("test.wrecker")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        WRECKER_WRITES
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
            FactKey::new(e(KITCHEN), INTEGRITY),
            ctx.basis_tick(),
            Change::Delta(-10_000),
            Cause::new("wrecked"),
        )]
    }
}

#[test]
fn a_ruin_is_walked_into_where_its_wall_was() {
    // With its walls down, the kitchen is open ground: Erin walks in over the rubble — though its
    // doorway, broken open, is too narrow for her here — and up to the table inside.
    let mut pkg = ashford();
    for b in &mut pkg.bodies {
        if b.entity_id == 1001 || b.entity_id == 1002 {
            b.half_width = 10;
        }
    }
    let mut city = City::from(pkg);
    city.attach(Box::new(Wrecker));
    city.run(3);
    walk(&mut city, ERIN, TABLE);
    assert_eq!(city.room_of(ERIN), KITCHEN);
    assert!(crossed(&city, ERIN, "entered"));
    assert!(!crossed(&city, ERIN, "pass_through"), "not through a door");
}
