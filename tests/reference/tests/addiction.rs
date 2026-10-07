//! A substance and the need it brings (Vol. III Ch. 2 §2.4, *Dependence*; Amendments A-12, A-13):
//! Ned, a little dependent on poppy, comes to crave it; he goes down to the manor undercroft where
//! he knows the vials are, takes one, and drinks it; each dose deepens the dependence, so the next
//! craving comes sooner; when the shelf is bare the craving climbs and withdrawal harms him; and
//! days without it end the craving. Erin, who never took any, never wants it.

use kernel::fact::{Cause, FactKey, FactType};
use kernel::system::CommittedView;
use kernel::value::Value;
use living::schema::{DEPENDENCE, HEALTH, NEED};
use minds::schema::{GOAL, REASON, REASON_DOSE};
use physical::schema::{CONSUMED, CONTAINED_IN};
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, City};

const HOUR: u64 = 6; // ten-minute ticks
const DAY: u64 = 24 * HOUR;

/// Ashford on a still day, and well fed: the habit, not hunger, moves Ned here.
fn still() -> City {
    let mut pkg = well_fed(with_tick_seconds(600));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(pkg)
}

fn pair(city: &City, holder: u64, fact: FactType, about: u64) -> Option<i64> {
    city.store()
        .read(FactKey::pair(e(holder), fact, e(about)))
        .and_then(|f| f.value.as_int())
}

fn vials_left(city: &City) -> usize {
    VIALS
        .iter()
        .filter(|v| city.read(**v, CONTAINED_IN).is_some())
        .count()
}

/// Run `ticks`, returning when Ned committed to fetching each vial.
fn doses(city: &mut City, ticks: u64) -> Vec<(u64, u64)> {
    let mut out = Vec::new();
    for _ in 0..ticks {
        city.run(1);
        let goal = city.store().read(FactKey::new(e(NED), GOAL));
        if let Some(g) = goal.filter(|g| g.provenance.tick == city.tick) {
            let reason = city.read(NED, REASON);
            if let (Value::Entity(vial), Some(Value::Int(REASON_DOSE))) = (g.value, reason) {
                out.push((city.tick, vial.raw()));
            }
        }
    }
    out
}

#[test]
fn ned_craves_fetches_and_drinks() {
    let mut city = still();
    let first = doses(&mut city, 18 * HOUR);
    let &(when, vial) = first.first().expect("he went for one");
    assert!(VIALS.contains(&vial));
    assert!(
        (8 * HOUR..15 * HOUR).contains(&when),
        "at hour {}",
        when / HOUR
    );
    // He drank it: the vial is nowhere, Physical reports it consumed, the craving has eased, and
    // the dependence has deepened.
    assert_eq!(city.read(vial, CONTAINED_IN), None);
    let consumed = city.store().read(FactKey::new(e(NED), CONSUMED)).unwrap();
    assert!(VIALS.contains(&match consumed.value {
        Value::Entity(v) => v.raw(),
        _ => 0,
    }));
    assert!(pair(&city, NED, NEED, POPPY).unwrap() < 3_000);
    assert!(pair(&city, NED, DEPENDENCE, POPPY).unwrap() > 5_000);
    // Erin never took any, and wants none.
    assert_eq!(pair(&city, ERIN, NEED, POPPY), None);
    assert_eq!(pair(&city, ERIN, DEPENDENCE, POPPY), None);
}

#[test]
fn deeper_dependence_brings_the_next_craving_sooner() {
    let mut city = still();
    let all = doses(&mut city, 2 * DAY);
    assert_eq!(all.len(), 4, "four vials, four doses: {all:?}");
    let gaps: Vec<u64> = all.windows(2).map(|w| w[1].0 - w[0].0).collect();
    assert!(
        gaps.windows(2).all(|g| g[1] < g[0]),
        "each sooner: {gaps:?}"
    );
    assert_eq!(vials_left(&city), 0);
}

#[test]
fn bare_shelves_bring_withdrawal_and_abstinence_ends_it() {
    let mut city = still();
    city.run(3 * DAY);
    // Nothing left to take: the craving is at its height, and it is costing him.
    assert_eq!(vials_left(&city), 0);
    assert_eq!(pair(&city, NED, NEED, POPPY), Some(10_000));
    let hurt = city.store().read(FactKey::new(e(NED), HEALTH)).unwrap();
    assert_eq!(hurt.provenance.cause, Cause::new("need"));
    assert!(hurt.value.as_int().unwrap() < 9_000);
    // Days without it: the dependence fades past the line, and the craving is gone.
    city.run(4 * DAY + 12 * HOUR);
    assert!(pair(&city, NED, DEPENDENCE, POPPY).unwrap_or(0) < 3_000);
    assert_eq!(pair(&city, NED, NEED, POPPY), None, "the craving has ended");
    assert!(city.int(NED, HEALTH).unwrap() > 0, "and he lived");
}
