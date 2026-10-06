//! Hunger (Vol. III Ch. 2 §2.4; Amendment A-16): it rises by the hour at the same pace at any
//! tick length; a hungry mind goes for food it believes in — here, the apple tree in the yard —
//! picks, and eats; what it has never seen it cannot go for; and with nothing to eat, hunger
//! becomes starvation, and starvation harms.

use kernel::fact::{Cause, FactKey};
use kernel::system::{CommittedView, RUNTIME_ID_FLOOR};
use kernel::value::Value;
use living::schema::{HEALTH, HUNGER};
use minds::schema::{GOAL, REASON, REASON_HUNGER};
use physical::schema::CONSUMED;
use reference::id::*;
use reference::{e, with_tick_seconds, without_minds, City};

const HOUR: u64 = 6; // ten-minute ticks

fn still(change: impl FnOnce(&mut packages::WorldPackage)) -> City {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    change(&mut pkg);
    City::from(pkg)
}

/// When `who` first set out to eat, and for what, within `ticks`.
fn first_meal(city: &mut City, who: u64, ticks: u64) -> Option<(u64, u64)> {
    for _ in 0..ticks {
        city.run(1);
        let goal = city.store().read(FactKey::new(e(who), GOAL));
        if let Some(g) = goal.filter(|g| g.provenance.tick == city.tick) {
            if let (Value::Entity(t), Some(REASON_HUNGER)) = (g.value, city.int(who, REASON)) {
                return Some((city.tick, t.raw()));
            }
        }
    }
    None
}

#[test]
fn hungry_erin_picks_an_apple_and_eats_it() {
    let mut city = still(|p| p.hunger.push((ERIN, 5_000)));
    let (when, target) = first_meal(&mut city, ERIN, 2 * HOUR).expect("she went to eat");
    assert_eq!(target, TREE, "the tree she knows");
    assert!(when < HOUR);
    let before = city.int(ERIN, HUNGER).unwrap();
    city.run(3 * HOUR);
    // She ate a new apple from it: hunger down by an apple's worth, less the hours' rise.
    let eaten = match city.read(ERIN, CONSUMED) {
        Some(Value::Entity(a)) => a.raw(),
        other => panic!("nothing eaten: {other:?}"),
    };
    assert!(eaten >= RUNTIME_ID_FLOOR, "an apple the tree grew");
    assert!(city.int(ERIN, HUNGER).unwrap() < before - 2_000);
}

#[test]
fn hunger_keeps_the_same_pace_at_any_tick_length() {
    for tick_seconds in [60, 600] {
        let mut city = City::from(without_minds(with_tick_seconds(tick_seconds)));
        city.run(10 * 3_600 / tick_seconds);
        // Ten hours at 200 an hour; short ticks carry fractions by unbiased rounding.
        let hunger = city.int(FINN, HUNGER).unwrap();
        assert!(
            (1_980..=2_020).contains(&hunger),
            "ticks of {tick_seconds} s: {hunger}"
        );
    }
}

#[test]
fn with_nothing_to_eat_hunger_starves() {
    // Hal, alone on the moor, nearly starving — and this year the bilberries bore nothing.
    let mut city = still(|p| {
        p.hunger.push((HAL, 8_800));
        p.deposits.retain(|d| d.id != BILBERRIES);
    });
    city.run(4 * HOUR);
    assert!(city.int(HAL, HUNGER).unwrap() > 9_000);
    let hurt = city.store().read(FactKey::new(e(HAL), HEALTH)).unwrap();
    assert_eq!(hurt.provenance.cause, Cause::new("starving"));
    assert!(hurt.value.as_int().unwrap() < 10_000);
}

#[test]
fn what_one_has_never_seen_one_cannot_go_for() {
    // The courier is hungry from midnight, but has never seen the tree: in the dark he has no idea
    // there is food in the yard. At dawn he sees it, and goes.
    let mut city = still(|p| p.hunger.push((COURIER, 5_000)));
    let (when, target) = first_meal(&mut city, COURIER, 12 * HOUR).expect("he found it");
    assert_eq!(target, TREE);
    assert!(when >= 6 * HOUR, "not before dawn: hour {}", when / HOUR);
}
