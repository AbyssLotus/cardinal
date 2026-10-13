//! A week in Ashford with nobody directed (`docs/audits/autonomous-agents.md`, phase 7;
//! Amendments A-14 to A-19): no new rules, the whole city left to itself. Everyone eats what they
//! know of and nobody starves; Carol works the orchard and Bob bakes every day, and the hungry eat
//! the pies; the household sleeps every night; every choice carries its reason; and the same seed
//! makes the same week.
//!
//! And, since every store is filled by someone (`docs/audits/resources.md` §5 and §9; Amendment
//! A-47), with nothing made filling itself: every trade is worked; the iron seam is smaller at the
//! week's end and never regrows; the millstone wears as it grinds.

use kernel::fact::FactKey;
use kernel::store::RealityStore;
use kernel::system::CommittedView;
use kernel::value::Value;
use living::schema::{HEALTH, HUNGER};
use minds::schema::{GOAL, REASON, REASON_WORK, RESTING, SCORE};
use physical::schema::{CONSUMED, MADE_OF};
use reference::id::*;
use reference::{e, with_tick_seconds, City};
use std::collections::{BTreeMap, BTreeSet};

const HOUR: u64 = 6; // ten-minute ticks
const DAY: u64 = 24 * HOUR;
const IRON_SEAM: u64 = 3120;
const MILLSTONE: u64 = 620;
/// Who keeps each store (Amendment A-47): the farmer, the miller, the baker, the miner, the
/// charcoal burner, the quarrywoman, the woodcutter, the steward, and Gwen.
const TRADES: [u64; 9] = [2060, 2062, 2063, 2080, 2081, 2084, 2083, 2015, 2014];

fn ashford() -> City {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    City::from(pkg)
}

/// What the week showed.
#[derive(Default)]
struct Week {
    /// What each organism ate: (eater, material).
    meals: Vec<(u64, u64)>,
    /// Harm to health, by cause.
    harms: BTreeSet<(u64, &'static str)>,
    /// (day, who) for every day someone worked.
    worked: BTreeSet<(u64, u64)>,
    /// The iron seam's stock at the end of each day.
    seam: Vec<i64>,
    /// (day, who) for every night someone slept.
    slept: BTreeSet<(u64, u64)>,
    /// Goals chosen without a reason or a score.
    unexplained: Vec<(u64, u64)>,
}

fn watch(city: &mut City, week: &mut Week, seen: &mut usize) {
    city.run(1);
    let day = (city.tick - 1) / DAY;
    for c in &city.chronicle[*seen..] {
        let who = c.subject().raw();
        if c.fact_type() == CONSUMED {
            if let Some(Value::Entity(t)) = city.read(who, CONSUMED) {
                for m in city.store().read_all(FactKey::new(t, MADE_OF)) {
                    if let Value::Entity(m) = m.value {
                        week.meals.push((who, m.raw()));
                    }
                }
            }
        }
        if c.fact_type() == HEALTH && c.cause().event() != "healing" {
            week.harms.insert((who, c.cause().event()));
        }
        if c.fact_type() == GOAL && city.read(who, GOAL).is_some() {
            let explained = city.int(who, REASON).is_some_and(|r| r > 0)
                && city.store().read(FactKey::new(e(who), SCORE)).is_some();
            if !explained {
                week.unexplained.push((city.tick, who));
            }
        }
    }
    *seen = city.chronicle.len();
    for who in [CAROL, BOB].into_iter().chain(TRADES) {
        if city.int(who, REASON) == Some(REASON_WORK) {
            week.worked.insert((day, who));
        }
    }
    if city.tick % DAY == 0 {
        week.seam
            .push(city.int(IRON_SEAM, resources::schema::STOCK).unwrap_or(0) / 100);
    }
    for who in [ALICE, BOB, CAROL, DAVE, ERIN] {
        if city.read(who, RESTING) == Some(Value::Bool(true)) {
            week.slept.insert((day, who));
        }
    }
}

#[test]
fn a_week_in_ashford_with_nobody_directed() {
    let (mut city, mut again) = (ashford(), ashford());
    let (mut week, mut shadow) = (Week::default(), Week::default());
    let (mut seen, mut shadow_seen) = (0, 0);
    for _ in 0..7 * DAY {
        watch(&mut city, &mut week, &mut seen);
        watch(&mut again, &mut shadow, &mut shadow_seen);
    }

    // Nobody starves: every organism is alive, ate something, and was never harmed by hunger.
    let organisms: Vec<u64> = city
        .store()
        .entities_with(HUNGER)
        .into_iter()
        .map(|o| o.raw())
        .collect();
    assert!(organisms.len() >= 25, "{} organisms", organisms.len());
    let mut meals: BTreeMap<u64, usize> = BTreeMap::new();
    for (who, _) in &week.meals {
        *meals.entry(*who).or_default() += 1;
    }
    for who in &organisms {
        assert!(city.int(*who, HEALTH).unwrap() > 0, "{who} died");
        assert!(
            meals.get(who).copied().unwrap_or(0) >= 3,
            "{who} barely ate: {meals:?}"
        );
    }
    let hungry_harms: Vec<_> = week
        .harms
        .iter()
        .filter(|(_, why)| *why == "starving" || *why == "died_of_hunger")
        .collect();
    assert!(hungry_harms.is_empty(), "{hungry_harms:?}");

    // Work, every day: Carol on the orchard round, Bob at the baking; and the pies get eaten.
    for day in 0..7 {
        for who in [CAROL, BOB] {
            assert!(week.worked.contains(&(day, who)), "{who} idle on day {day}");
        }
    }
    let pies = week.meals.iter().filter(|(_, m)| *m == PIE).count();
    assert!(pies >= 7, "{pies} pies eaten in a week");

    // Every trade is worked, with nothing made filling itself (Amendment A-47).
    for who in TRADES {
        assert!(
            week.worked.iter().any(|(_, w)| *w == who),
            "{who} never worked"
        );
    }
    // The iron seam: worked, and never regrowing.
    assert_eq!(week.seam.len(), 7);
    assert!(week.seam[6] < 2_000, "the seam was worked: {:?}", week.seam);
    assert!(
        week.seam.windows(2).all(|w| w[1] <= w[0]),
        "it never regrows: {:?}",
        week.seam
    );
    // The millstone wears as it grinds.
    let millstone = city
        .int(MILLSTONE, physical::schema::INTEGRITY)
        .unwrap_or(10_000);
    assert!(millstone < 10_000, "the millstone wore: {millstone}");

    // Sleep, every night, for the whole household.
    for day in 0..7 {
        for who in [ALICE, BOB, CAROL, DAVE, ERIN] {
            assert!(
                week.slept.contains(&(day, who)),
                "{who} never slept on day {day}"
            );
        }
    }

    // Every choice has its reason in the trace.
    assert!(week.unexplained.is_empty(), "{:?}", week.unexplained);

    // The same seed, the same week.
    assert_eq!(week.meals, shadow.meals);
    assert_eq!(city.store().state_hash(), again.store().state_hash());
}
