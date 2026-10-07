//! Ashford's minds (Appendix A, Ruling 14; Amendment A-9): they choose from what they believe,
//! act through the intents a player would use, keep the hours the world gives them, open a shut
//! door that stands in their way, and leave a trace of why. Two minds in the same cold yard
//! choose differently because they know different things.

use kernel::fact::FactKey;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use minds::schema::{AT, GOAL, REASON, REASON_ROUTINE, REASON_WARMTH, SCORE, STEP, STEP_APPROACH};
use physical::schema::PORTAL_OPEN;
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, City};

/// Ashford at ten-minute ticks on a still day — no weather, only the sun — well fed, and with no
/// one curious or wanting anything, so what the minds feel follows the day and nothing else.
fn still() -> packages::WorldPackage {
    let mut pkg = well_fed(with_tick_seconds(600));
    pkg.curiosity.clear();
    pkg.likes.clear();
    pkg.physical_rules.temperature_variability_centi_c = 0;
    pkg
}

const HOUR: u64 = 6; // ten-minute ticks

fn own(city: &City, who: u64, fact: kernel::fact::FactType) -> Option<Value> {
    city.store()
        .read(FactKey::new(e(who), fact))
        .map(|f| f.value)
}

fn felt_body(city: &City, who: u64) -> i64 {
    own(city, who, information::schema::FELT_BODY_HEAT)
        .and_then(|v| v.as_int())
        .unwrap()
}

/// A decision, as its trace recorded it: when, what, why, and how strongly.
#[derive(Debug, PartialEq, Eq)]
struct Decision {
    tick: u64,
    goal: u64,
    reason: i64,
    score: i64,
}

/// Run `city` for `ticks`, recording every new goal `who` commits to.
fn watch(city: &mut City, who: u64, ticks: u64) -> Vec<Decision> {
    let mut seen = Vec::new();
    for _ in 0..ticks {
        city.run(1);
        let goal = city.store().read(FactKey::new(e(who), GOAL));
        if let Some(goal) = goal.filter(|g| g.provenance.tick == city.tick) {
            let Value::Entity(target) = goal.value else {
                continue;
            };
            seen.push(Decision {
                tick: city.tick,
                goal: target.raw(),
                reason: own(city, who, REASON).and_then(|v| v.as_int()).unwrap(),
                score: own(city, who, SCORE).and_then(|v| v.as_int()).unwrap(),
            });
        }
    }
    seen
}

#[test]
fn erin_comes_in_from_the_cold() {
    let mut city = City::from(still());
    let choices = watch(&mut city, ERIN, 36 * HOUR);
    // One choice in a day and a half: in the late evening, cold, she makes for the kitchen she
    // remembers warm.
    assert_eq!(choices.len(), 1, "{choices:?}");
    let choice = &choices[0];
    assert_eq!((choice.goal, choice.reason), (KITCHEN, REASON_WARMTH));
    assert!(choice.score > 0);
    assert!(
        (19 * HOUR..30 * HOUR).contains(&choice.tick),
        "decided at hour {}",
        choice.tick / HOUR
    );
    // She went in, and having arrived, let the goal go — and stayed, warmer inside.
    assert_eq!(city.room_of(ERIN), KITCHEN);
    assert_eq!(own(&city, ERIN, GOAL), None);
}

#[test]
fn the_courier_shivers_in_the_yard_he_knows_no_better() {
    // Cold as Erin, in the same yard — but he has never seen the kitchen, and knows of nowhere
    // warmer. Nothing he believes offers him a choice.
    let mut city = City::from(still());
    let mut coldest = i64::MAX;
    for _ in 0..30 * HOUR {
        city.run(1);
        coldest = coldest.min(felt_body(&city, COURIER));
        assert_eq!(own(&city, COURIER, GOAL), None);
    }
    assert!(coldest < 2_300, "he was cold ({coldest})");
    assert_eq!(city.room_of(COURIER), YARD);
}

#[test]
fn erin_opens_the_shut_door_that_stands_in_her_way() {
    // Bob shuts the front door as the day begins — and it is a day off, so his baking does not
    // take him out through it. That night, cold, Erin makes for the kitchen; her way is shut, so
    // she walks to the door, opens it, and goes in.
    let mut pkg = still();
    pkg.roles.clear();
    let mut city = City::from(pkg);
    city.close(BOB, DOOR_IN);
    city.run(3); // directed, then commanded, then the door swings to
    assert_eq!(city.read(DOOR_OUT, PORTAL_OPEN), Some(Value::Bool(false)));
    let mut approached = false;
    for _ in 0..36 * HOUR {
        city.run(1);
        approached |= own(&city, ERIN, STEP) == Some(Value::Int(STEP_APPROACH));
    }
    assert!(approached, "she went to the door");
    assert_eq!(city.read(DOOR_OUT, PORTAL_OPEN), Some(Value::Bool(true)));
    assert_eq!(city.room_of(ERIN), KITCHEN);
}

#[test]
fn the_guard_keeps_his_hours() {
    let mut city = City::from(still());
    // At midnight no routine holds him: he stays where he is.
    let start = city.read(GUARD, physical::schema::POSITION);
    city.run(5 * HOUR);
    assert_eq!(city.read(GUARD, physical::schema::POSITION), start);
    // By day he keeps the gate...
    let day = watch(&mut city, GUARD, 3 * HOUR);
    assert_eq!(day.len(), 1, "{day:?}");
    assert_eq!((day[0].goal, day[0].reason), (GATE, REASON_ROUTINE));
    assert_eq!(own(&city, GUARD, AT), Some(Value::Entity(e(GATE))));
    // ...and in the evening the well, where he stays into the night.
    city.run(10 * HOUR - 1);
    let evening = watch(&mut city, GUARD, 2 * HOUR);
    assert_eq!(evening.len(), 1, "{evening:?}");
    assert_eq!(evening[0].goal, WELL);
    city.run(4 * HOUR);
    assert_eq!(own(&city, GUARD, AT), Some(Value::Entity(e(WELL))));
    assert_eq!(own(&city, GUARD, GOAL), None);
}

#[test]
fn a_mind_never_heads_for_what_it_does_not_know() {
    // Ashford as before, except that Erin was never told of the kitchen or its door: as cold as
    // ever, she has nowhere to go.
    let mut pkg = still();
    for (mind, things) in &mut pkg.knows {
        if *mind == ERIN {
            things.retain(|t| *t != KITCHEN && *t != DOOR_OUT && *t != DOOR_IN);
        }
    }
    let mut city = City::from(pkg);
    let choices = watch(&mut city, ERIN, 30 * HOUR);
    assert!(choices.is_empty(), "{choices:?}");
    assert_eq!(city.room_of(ERIN), YARD);
}

#[test]
fn the_same_seed_makes_the_same_choices() {
    let run = || {
        let mut city = City::from(still());
        city.close(BOB, DOOR_IN);
        city.run(30 * HOUR);
        city.store().state_hash()
    };
    assert_eq!(run(), run());
}
