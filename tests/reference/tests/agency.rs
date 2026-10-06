//! Everyone acts, unless directed (Vol. V Ch. 9 §9.3; Amendment A-14): every person and animal in
//! Ashford has a mind of its own; a controller may direct one, and while it does, that mind stands
//! aside; released, it decides for itself again. Left alone for a night, the city decides for
//! itself — every choice with a reason in its trace.

use kernel::fact::FactKey;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use minds::schema::{GOAL, REASON, RESTING, SCORE};
use reference::id::*;
use reference::{e, with_tick_seconds, City};
use std::collections::BTreeSet;

const HOUR: u64 = 6; // ten-minute ticks

fn still() -> City {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(pkg)
}

#[test]
fn a_directed_person_does_as_told_and_their_mind_stands_aside() {
    let mut city = City::new();
    // Commanding Bob directs him first; the command arrives the tick after.
    city.go(BOB, BEDROOM, 140);
    city.run(1);
    assert!(city.directed(BOB));
    city.run_until(40, |c| c.room_of(BOB) == BEDROOM && !c.travelling(BOB));
    assert_eq!(city.room_of(BOB), BEDROOM);
    // While directed, his own mind proposed nothing at all.
    let minded = city
        .chronicle
        .iter()
        .filter(|c| c.subject() == e(BOB) && c.fact_type().name().starts_with("mind."))
        .filter(|c| c.fact_type() != minds::schema::DIRECTED)
        .count();
    assert_eq!(minded, 0);
    // Released, he is his own again.
    city.release(BOB);
    city.run(2);
    assert!(!city.directed(BOB));
}

/// Every mind that chose somewhere to go or lay down to rest in `ticks`, with the reasons given.
fn choices(city: &mut City, ticks: u64) -> (BTreeSet<u64>, Vec<(u64, u64, i64, i64)>) {
    let mut who = BTreeSet::new();
    let mut made = Vec::new();
    let minds: Vec<u64> = city
        .store()
        .entities_with(minds::schema::WALK_SPEED)
        .into_iter()
        .map(|m| m.raw())
        .collect();
    for _ in 0..ticks {
        city.run(1);
        for &m in &minds {
            let fresh = |fact| {
                city.store()
                    .read(FactKey::new(e(m), fact))
                    .filter(|f| f.provenance.tick == city.tick)
            };
            if let Some(goal) = fresh(GOAL) {
                let reason = city.int(m, REASON).unwrap_or(0);
                let score = city.int(m, SCORE).unwrap_or(0);
                let Value::Entity(target) = goal.value else {
                    continue;
                };
                made.push((city.tick, m, reason, score));
                assert!(
                    reason > 0 && score > 0,
                    "{m} chose {target:?} for no reason"
                );
                who.insert(m);
            }
            if fresh(RESTING).is_some_and(|f| f.value == Value::Bool(true)) {
                who.insert(m);
            }
        }
    }
    (who, made)
}

#[test]
fn a_day_in_ashford_with_nobody_directed() {
    let mut city = still();
    let (who, made) = choices(&mut city, 24 * HOUR);
    // Most of Ashford chose something for itself: somewhere to go, or to lie down.
    assert!(
        who.len() >= 15,
        "{} minds chose anything: {who:?}",
        who.len()
    );
    assert!(!made.is_empty());
    // Nobody was directed.
    let directed = city.store().entities_with(minds::schema::DIRECTED).len();
    assert_eq!(directed, 0);
    // The same seed, the same day.
    let mut again = still();
    let (_, made_again) = choices(&mut again, 24 * HOUR);
    assert_eq!(made, made_again);
    assert_eq!(city.store().state_hash(), again.store().state_hash());
}
