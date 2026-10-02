//! What can be seen in Ashford (Vol. III Ch. 1 §1.6, §1.11; Amendment A-4): walls hide, open
//! doors and glass do not, shut doors and drawn curtains do, a wagon in the way does, and a
//! ridge hides one valley from the next.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use physical::schema::OPAQUE;
use physical::sight::{line_of_sight, visible};
use reference::id::*;
use reference::{e, City};

fn sees(city: &City, who: u64, what: u64) -> bool {
    line_of_sight(city.store(), e(who), e(what))
}

#[test]
fn dave_at_the_window_sees_the_yard_but_not_through_the_floor() {
    let city = City::new();
    // Through the bedroom window — shut, but glass — down to Erin in the yard, and back.
    assert!(sees(&city, DAVE, ERIN));
    assert!(sees(&city, ERIN, DAVE));
    // Not through the floor to Bob in the kitchen, nor down to the raccoon in the cellar.
    assert!(!sees(&city, DAVE, BOB));
    assert!(!sees(&city, DAVE, RACCOON));
    // What Dave can see within 20 m includes Erin and nobody downstairs.
    let seen: Vec<u64> = visible(city.store(), e(DAVE), 2_000)
        .into_iter()
        .map(|(x, _)| x.raw())
        .collect();
    assert!(seen.contains(&ERIN));
    assert!(!seen.contains(&BOB) && !seen.contains(&RACCOON));
}

#[test]
fn an_open_door_shows_the_kitchen_and_a_shut_one_hides_it() {
    let mut city = City::new();
    // Finn, on the doorstep, sees Bob just inside the open front door.
    assert!(sees(&city, FINN, BOB));
    // Bob shuts the door — an opaque door — and Finn can no longer see him.
    city.close(BOB, DOOR_IN);
    city.run(2);
    assert!(!sees(&city, FINN, BOB));
}

/// Draws the curtains across the bedroom window on its first tick.
struct Curtains;
impl System for Curtains {
    fn id(&self) -> SystemId {
        SystemId::new("probe.curtains")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[OPAQUE]
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
            FactKey::new(e(BEDROOM_WINDOW), OPAQUE),
            ctx.basis_tick(),
            Change::Set(Value::Bool(true)),
            Cause::new("curtains_drawn"),
        )]
    }
}

#[test]
fn drawn_curtains_hide_the_yard() {
    let mut city = City::new();
    city.attach(Box::new(Curtains));
    city.run(1);
    assert!(!sees(&city, DAVE, ERIN));
}

#[test]
fn the_hay_wagon_hides_the_courier_from_erin() {
    let city = City::new();
    // The wagon stands between Erin and the courier; the cat is off to one side of it.
    assert!(!sees(&city, ERIN, COURIER));
    assert!(sees(&city, ERIN, CAT));
}

#[test]
fn the_ridge_hides_one_valley_from_the_next() {
    let city = City::new();
    assert!(!sees(&city, JORY, KIT), "the ridge is in the way");
    assert!(
        sees(&city, LENA, JORY),
        "from the ridgeline, the west valley"
    );
    assert!(sees(&city, LENA, KIT), "and the east");
}

#[test]
fn the_cottage_walls_hide_its_kitchen() {
    let city = City::new();
    assert!(!sees(&city, VILLAGER, COTTAGE_TABLE));
}
