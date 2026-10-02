//! Moving through Ashford under its own rules (Vol. III Ch. 1 §1.11; Appendix A, Rulings 4 and
//! 13; Amendments A-4, A-5): people and animals are only ever *asked* to go somewhere or to open
//! a door; Physical Reality decides whether and how — through openings that are open, that they
//! fit, and that they can reach; around what is solid; down when nothing holds them up.

use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::tick::TickError;
use kernel::value::Value;
use physical::regions::is_within;
use physical::schema::{CONTAINED_IN, FALL_HEIGHT, LEADS_TO, PORTAL_OPEN};
use physical::shape::body_box;
use physical::space::{
    can_reach, destinations, height_above_ground, local_position, portals_in, position_in,
    reachable_regions,
};
use physical::terrain::{ground, slope_percent};
use reference::id::*;
use reference::{e, City};

fn door_open(city: &City, door: u64) -> bool {
    city.read(door, PORTAL_OPEN) != Some(Value::Bool(false))
}

#[test]
fn with_the_door_shut_the_cat_comes_in_the_window_and_lands_under_it() {
    let mut city = City::new();
    // Bob, standing by it in the kitchen, shuts the front door; both its faces swing to.
    city.close(BOB, DOOR_IN);
    city.run(2);
    assert!(!door_open(&city, DOOR_IN) && !door_open(&city, DOOR_OUT));

    city.go(CAT, KITCHEN, 300).go(COURIER, KITCHEN, 140);
    city.run(15);
    // The cat — small enough, and the sill low enough to hop — came in through the low window,
    // dropped the 30 cm from the sill, and stands on the floor right under it.
    assert_eq!(city.room_of(CAT), KITCHEN);
    assert_eq!(local_position(city.store(), e(CAT)), [-500, 0, 0]);
    assert_eq!(city.int(CAT, FALL_HEIGHT), Some(30));
    assert!(!city.travelling(CAT));
    // The courier is too broad for the window and too tall for the bulkhead: still outside, and
    // the world says so — while the intent stands, waiting for a way.
    assert_eq!(city.room_of(COURIER), YARD);
    assert!(city.blocked(COURIER) && city.travelling(COURIER));

    // Bob opens the door again; the courier goes in by it.
    city.open(BOB, DOOR_IN);
    city.run(12);
    assert_eq!(city.room_of(COURIER), KITCHEN);
    assert_eq!(
        local_position(city.store(), e(COURIER)),
        [0, 500, 0],
        "on the doormat"
    );
    assert!(!city.blocked(COURIER) && !city.travelling(COURIER));
}

#[test]
fn two_people_cannot_squeeze_through_one_doorway_at_once() {
    let mut city = City::new();
    // Finn and Gwen are both on the doorstep, and both are asked in on the same tick.
    city.go(FINN, KITCHEN, 140).go(GWEN, KITCHEN, 140);
    city.run(2);
    assert_eq!(city.room_of(FINN), KITCHEN, "the lower id goes first");
    assert_eq!(city.room_of(GWEN), YARD, "the other waits its turn");
    city.run(1);
    assert_eq!(city.room_of(GWEN), KITCHEN);
}

#[test]
fn carol_walks_off_the_shed_roof_and_falls() {
    let mut city = City::new();
    assert_eq!(
        height_above_ground(city.store(), e(CAROL)),
        250,
        "on the shed roof"
    );
    city.go(CAROL, STONE, 140);
    city.run_until(30, |c| !c.travelling(CAROL));
    // She walked level off the edge, fell the shed's height, and finished beside the stone.
    assert_eq!(city.int(CAROL, FALL_HEIGHT), Some(250));
    assert_eq!(height_above_ground(city.store(), e(CAROL)), 0);
    assert!(!city.travelling(CAROL));
    let at = local_position(city.store(), e(CAROL));
    assert!((at[1] - -400).abs() <= 30, "beside the stone: {at:?}");
}

#[test]
fn nell_steps_off_the_boulder_onto_the_hillside() {
    let mut city = City::new();
    city.go(NELL, MARKER, 140);
    city.run_until(40, |c| !c.travelling(NELL));
    // Off the 3 m boulder at the first half-spacing leg, and down onto the slope there, which
    // stands 1 m up: a 4 m fall.
    assert_eq!(city.int(NELL, FALL_HEIGHT), Some(400));
    let at = local_position(city.store(), e(NELL));
    assert_eq!(
        at[2],
        ground(city.store(), e(HILL), at[0], at[1]),
        "on the ground"
    );
    assert!(!city.travelling(NELL));
}

#[test]
fn the_wardrobe_does_not_fit_any_way_in_but_the_courier_does() {
    let mut city = City::new();
    city.go(WARDROBE, KITCHEN, 100).go(COURIER, KITCHEN, 140);
    city.run(10);
    // 1.2 m wide: wider than the door (90 cm), the window, and the bulkhead (1 m).
    assert_eq!(city.room_of(WARDROBE), YARD);
    assert!(city.blocked(WARDROBE));
    assert_eq!(city.room_of(COURIER), KITCHEN);
}

#[test]
fn bob_goes_upstairs_and_the_engine_knows_how_high_he_is() {
    let mut city = City::new();
    city.go(BOB, BEDROOM, 140);
    city.run_until(30, |c| !c.travelling(BOB));
    assert_eq!(city.room_of(BOB), BEDROOM);
    assert!(
        is_within(city.store(), e(BOB), e(HOUSE)),
        "still inside the house"
    );
    assert_eq!(height_above_ground(city.store(), e(BOB)), 300);
    assert_eq!(
        position_in(city.store(), e(BOB), e(HOUSE)),
        Some([400, -400, 300]),
        "at the top of the stairs"
    );
}

#[test]
fn bob_walks_round_the_kitchen_table_not_through_it() {
    let mut city = City::new();
    // First to the low window, then across the kitchen to the lamp — the table is in the way.
    city.go(BOB, WINDOW_IN, 140);
    city.run_until(20, |c| !c.travelling(BOB));
    let table = body_box(city.store(), e(TABLE)).unwrap();
    city.go(BOB, LAMP, 140);
    city.run(1);
    let mut ticks = 0;
    while city.travelling(BOB) && ticks < 40 {
        city.run(1);
        ticks += 1;
        let at = local_position(city.store(), e(BOB));
        assert!(
            !table.footprint_contains(at, 24),
            "walked into the table at {at:?}"
        );
    }
    assert!(!city.travelling(BOB), "reached the lamp");
    let at = local_position(city.store(), e(BOB));
    assert!(
        (at[0] - 300).abs() <= 40 && at[1].abs() <= 40,
        "by the lamp: {at:?}"
    );
}

#[test]
fn bob_steps_out_into_the_yard() {
    let mut city = City::new();
    city.go(BOB, YARD, 140);
    city.run_until(10, |c| !c.travelling(BOB));
    assert_eq!(city.room_of(BOB), YARD);
    assert!(!is_within(city.store(), e(BOB), e(HOUSE)));
    assert_eq!(
        local_position(city.store(), e(BOB)),
        [1_000, 500, 0],
        "outside the front door"
    );
}

#[test]
fn the_villager_walks_home_up_the_hill_and_the_ladder() {
    let mut city = City::new();
    let table = body_box(city.store(), e(COTTAGE_TABLE)).unwrap();
    city.go(VILLAGER, LOFT, 120);
    let mut entered_kitchen = false;
    for _ in 0..200 {
        city.run(1);
        let s = city.store();
        let at = local_position(s, e(VILLAGER));
        match city.room_of(VILLAGER) {
            HILL => assert!(
                at[2] >= ground(s, e(HILL), at[0], at[1]) - 2,
                "below the hill"
            ),
            COTTAGE_KITCHEN => {
                entered_kitchen = true;
                assert!(!table.footprint_contains(at, 24), "through the table");
            }
            _ => {}
        }
        if !city.travelling(VILLAGER) {
            break;
        }
    }
    assert!(entered_kitchen);
    assert_eq!(city.room_of(VILLAGER), LOFT, "home");
    // A storey above the hillside, though the cottage stands 3 m up the hill.
    assert_eq!(height_above_ground(city.store(), e(VILLAGER)), 260);
}

#[test]
fn the_hiker_finds_the_pass_round_the_cliff() {
    let mut city = City::new();
    city.go(HIKER, CAIRN, 200);
    let mut furthest_north = 0;
    for _ in 0..120 {
        city.run(1);
        let s = city.store();
        let at = local_position(s, e(HIKER));
        furthest_north = furthest_north.max(at[1]);
        let grade = slope_percent(s, e(HILL), at[0], at[1]).unwrap_or(0);
        assert!(
            grade <= 100,
            "on ground too steep to walk ({grade}%) at {at:?}"
        );
        if !city.travelling(HIKER) {
            break;
        }
    }
    assert!(!city.travelling(HIKER) && !city.blocked(HIKER), "arrived");
    assert!(furthest_north >= 3_000, "went round by the pass");
}

#[test]
fn the_manor_vault_cannot_be_reached_but_its_undercroft_can() {
    let mut city = City::new();
    city.go(STEWARD, VAULT, 140);
    city.run(3);
    assert!(city.blocked(STEWARD), "no opening leads into the vault");
    city.go(STEWARD, UNDERCROFT, 140);
    city.run_until(20, |c| !c.travelling(STEWARD));
    assert_eq!(city.room_of(STEWARD), UNDERCROFT);
    assert!(!city.blocked(STEWARD));
}

#[test]
fn a_door_out_of_reach_stays_as_it_is() {
    let mut city = City::new();
    // Alice is upstairs and Erin out in the yard: neither can reach the front door.
    city.close(ALICE, DOOR_IN).close(ERIN, DOOR_OUT);
    city.run(2);
    assert!(door_open(&city, DOOR_IN) && door_open(&city, DOOR_OUT));
    assert_eq!(city.refused(ALICE), Some(DOOR_IN));
    assert_eq!(city.refused(ERIN), Some(DOOR_OUT));
    // Bob, beside it, can — and a success clears nothing he was refused before.
    city.close(BOB, DOOR_IN);
    city.run(2);
    assert!(!door_open(&city, DOOR_IN) && !door_open(&city, DOOR_OUT));
    assert_eq!(city.refused(BOB), None);
}

#[test]
fn travel_that_can_never_happen_is_reported_blocked() {
    let mut city = City::new();
    // No speed given; and a shed, which does not move.
    city.go(ERIN, KITCHEN, 0).go(SHED, KITCHEN, 100);
    city.run(2);
    assert!(city.blocked(ERIN), "no speed");
    assert!(city.blocked(SHED), "not mobile");
    assert_eq!(city.room_of(ERIN), YARD);
}

/// A rogue that tries to put Alice in the manor vault by writing her containment directly.
struct Teleporter;
impl System for Teleporter {
    fn id(&self) -> SystemId {
        SystemId::new("rogue.teleporter")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[CONTAINED_IN]
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, _view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        vec![Proposal::new(
            self.id(),
            FactKey::new(e(ALICE), CONTAINED_IN),
            ctx.basis_tick(),
            Change::Set(Value::Entity(e(VAULT))),
            Cause::new("teleport"),
        )]
    }
}

#[test]
fn nothing_but_physical_reality_moves_a_body() {
    let mut city = City::new();
    city.attach(Box::new(Teleporter));
    let err = city.try_run().expect_err("the kernel refuses the rogue");
    assert_eq!(
        err,
        TickError::Refused {
            system: SystemId::new("rogue.teleporter"),
            fact_type: CONTAINED_IN
        }
    );
    assert_eq!(city.room_of(ALICE), BEDROOM, "and nothing happened");
}

fn ids<I: IntoIterator<Item = kernel::identity::EntityId>>(set: I) -> Vec<u64> {
    set.into_iter().map(|x| x.raw()).collect()
}

#[test]
fn the_undercroft_reaches_the_grounds_only_through_the_hall() {
    let city = City::new();
    let s = city.store();
    // One flight of stairs leads out of the undercroft, and only to the hall...
    assert_eq!(ids(destinations(s, e(UNDERCROFT))), vec![HALL]);
    // ...which has three ways on: the front door, the stairs down, the stairs up.
    assert_eq!(
        ids(portals_in(s, e(HALL))),
        vec![MANOR_DOOR_IN, UNDERCROFT_STAIRS, GALLERY_STAIRS]
    );
    // So the grounds are reachable from the undercroft — by way of the hall.
    assert!(can_reach(s, e(UNDERCROFT), e(GROUNDS)));
    assert_eq!(
        ids(reachable_regions(s, e(UNDERCROFT))),
        vec![GROUNDS, HALL, UNDERCROFT, GALLERY]
    );
    // The vault has no openings at all: nothing leaves it and nothing reaches it.
    assert_eq!(ids(reachable_regions(s, e(VAULT))), vec![VAULT]);
    assert!(!can_reach(s, e(UNDERCROFT), e(VAULT)));
}

/// A mason re-cuts the top of the undercroft stair so that it gives onto the vault instead.
struct Mason;
impl System for Mason {
    fn id(&self) -> SystemId {
        SystemId::new("probe.mason")
    }
    fn reads(&self) -> &'static [FactType] {
        &[]
    }
    fn writes(&self) -> &'static [FactType] {
        &[LEADS_TO]
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
            FactKey::new(e(UNDERCROFT_STAIRS_UP), LEADS_TO),
            ctx.basis_tick(),
            Change::Set(Value::Entity(e(VAULT))),
            Cause::new("recut"),
        )]
    }
}

#[test]
fn where_an_opening_leads_decides_where_you_can_get_to() {
    let mut city = City::new();
    city.attach(Box::new(Mason));
    city.run(1);
    let s = city.store();
    // Up the undercroft stair now lies the vault, not the hall: the undercroft is cut off from
    // the grounds and opens onto the vault.
    assert!(!can_reach(s, e(UNDERCROFT), e(GROUNDS)));
    assert!(can_reach(s, e(UNDERCROFT), e(VAULT)));
    // The hall's end of the stair still leads down into the undercroft.
    assert!(can_reach(s, e(HALL), e(UNDERCROFT)));
    // And the steward, asked to walk from the undercroft to the grounds, finds no way.
    city.go(STEWARD, UNDERCROFT, 140);
    city.run_until(20, |c| !c.travelling(STEWARD));
    assert_eq!(city.room_of(STEWARD), UNDERCROFT);
    city.go(STEWARD, GROUNDS, 140);
    city.run(3);
    assert!(city.blocked(STEWARD));
}
