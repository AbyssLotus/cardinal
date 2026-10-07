//! Love, and the need it brings (Vol. III Ch. 2 §2.4, Ch. 5; Vol. II Ch. 4; Amendment A-11):
//! fondness grows with time in each other's sight, never past how compatible two people are;
//! mutual fondness makes lovers; a bond makes a longing arise; apart, the longing grows until it
//! moves a mind to go where it believes its beloved is; lovers kept apart cool, part, and the
//! longing ends with the bond.

use information::schema::{AFFECTION, PLACE_OF};
use kernel::fact::{Cause, Fact, FactKey, FactType};
use kernel::system::CommittedView;
use kernel::value::Value;
use living::schema::NEED;
use minds::schema::{GOAL, REASON, REASON_NEED};
use reference::id::*;
use reference::{e, well_fed, with_tick_seconds, City};
use society::schema::{BOND, LOVERS};

const HOUR: u64 = 6; // ten-minute ticks
const DAY: u64 = 24 * HOUR;

/// Ashford on a still day, and well fed: love, not hunger, moves them here.
fn still(change: impl FnOnce(&mut packages::WorldPackage)) -> City {
    let mut pkg = well_fed(with_tick_seconds(600));
    pkg.physical_rules.temperature_variability_centi_c = 0;
    change(&mut pkg);
    City::from(pkg)
}

fn pair(city: &City, holder: u64, fact: FactType, about: u64) -> Option<Fact> {
    city.store().read(FactKey::pair(e(holder), fact, e(about)))
}

fn fond(city: &City, holder: u64, about: u64) -> i64 {
    pair(city, holder, AFFECTION, about).map_or(0, |f| f.value.as_int().unwrap())
}

#[test]
fn finn_and_gwen_fall_in_love_and_the_courier_does_not() {
    let mut city = still(|_| {});
    city.run(3 * DAY);
    // Side by side, well matched: lovers on the second day, each way.
    let bond = pair(&city, FINN, BOND, GWEN).expect("lovers");
    assert_eq!(bond.value, Value::Int(LOVERS));
    assert_eq!(bond.provenance.cause, Cause::new("fell_in_love"));
    assert!(
        (DAY..2 * DAY).contains(&bond.provenance.tick),
        "on day {}",
        bond.provenance.tick / DAY
    );
    assert_eq!(
        pair(&city, GWEN, BOND, FINN).map(|f| f.value),
        Some(Value::Int(LOVERS))
    );
    // Never fonder than their 97% compatibility allows.
    assert!(fond(&city, FINN, GWEN) <= 9_667 && fond(&city, FINN, GWEN) > 9_000);
    // The courier sees Finn every day, but they are ill-matched (48%): no fonder than that, and
    // no bond.
    let courier = fond(&city, COURIER, FINN);
    assert!((3_000..=4_834).contains(&courier), "{courier}");
    assert!(pair(&city, COURIER, BOND, FINN).is_none());
    // The bond has brought a longing — quiet while they are together.
    assert_eq!(
        pair(&city, GWEN, NEED, FINN).map(|f| f.value),
        Some(Value::Int(0))
    );
}

#[test]
fn apart_gwen_longs_for_finn_and_goes_back_to_him() {
    // Finn, without a mind of his own, keeps to the kitchen; Gwen spends her nights there too, out
    // of the cold, and by day they fall for each other there.
    let mut city = still(|p| p.minds.retain(|(m, _)| *m != FINN));
    city.go(FINN, KITCHEN, 140);
    city.run(3 * DAY + 12 * HOUR); // noon on the fourth day, lovers in the kitchen
    assert_eq!((city.room_of(FINN), city.room_of(GWEN)), (KITCHEN, KITCHEN));
    assert!(pair(&city, GWEN, BOND, FINN).is_some(), "lovers");
    // Gwen is sent down to the cellar. By day she can still see him up the stairwell, and that is
    // enough; at dusk she cannot, and the longing grows.
    city.go(GWEN, CELLAR, 140);
    city.run_until(2 * HOUR, |c| c.room_of(GWEN) == CELLAR);
    city.release(GWEN); // and left to herself
    let mut longest = 0;
    let mut went_for = None;
    let mut finn_kept_her_in_the_kitchen = false;
    for _ in 0..10 * HOUR {
        city.run(1);
        longest =
            longest.max(pair(&city, GWEN, NEED, FINN).map_or(0, |f| f.value.as_int().unwrap()));
        let goal = city.store().read(FactKey::new(e(GWEN), GOAL));
        if let Some(g) = goal.filter(|_| went_for.is_none()) {
            let reason = city
                .store()
                .read(FactKey::new(e(GWEN), REASON))
                .unwrap()
                .value;
            went_for = Some((g.value, reason, city.tick));
        }
        // Finn, in the lit kitchen, does not go on believing she is there while she is below:
        // he sees she is gone, or sees where she went.
        let below = city.room_of(GWEN) == CELLAR && city.room_of(FINN) == KITCHEN;
        let thinks = pair(&city, FINN, PLACE_OF, GWEN).map(|f| (f.value, f.provenance.tick));
        if let Some((Value::Entity(place), since)) = thinks {
            finn_kept_her_in_the_kitchen |= below
                && place == e(KITCHEN)
                && city.tick > since + 3
                && city.read(GWEN, GOAL).is_none();
        }
    }
    // Her longing grew past the line, and moved her: to the kitchen, where she left him.
    assert!(longest > 3_000, "longing reached {longest}");
    let (goal, reason, _) = went_for.expect("she went back");
    assert_eq!(
        (goal, reason),
        (Value::Entity(e(KITCHEN)), Value::Int(REASON_NEED))
    );
    assert_eq!(city.room_of(GWEN), KITCHEN);
    assert!(
        !finn_kept_her_in_the_kitchen,
        "he could see she was not in the kitchen"
    );
    // Together again, the longing eases away.
    assert_eq!(
        pair(&city, GWEN, NEED, FINN).map(|f| f.value),
        Some(Value::Int(0))
    );
}

#[test]
fn lovers_kept_apart_cool_and_part_and_the_longing_ends() {
    // Finn and Gwen without minds of their own: lovers on the doorstep, then taken apart — he up
    // to the bedroom, she down to the cellar, where neither can see the other and neither will
    // go looking. Fondness fades fast in this world: 40% a day.
    let mut city = still(|p| {
        p.minds.retain(|(m, _)| *m != GWEN && *m != FINN);
        p.information_rules.as_mut().unwrap().affection_fade_per_day = 4_000;
    });
    city.run(2 * DAY + 18 * HOUR);
    assert!(pair(&city, FINN, BOND, GWEN).is_some(), "lovers first");
    city.go(GWEN, CELLAR, 140).go(FINN, BEDROOM, 140);
    city.run(4 * DAY);
    assert_eq!((city.room_of(FINN), city.room_of(GWEN)), (BEDROOM, CELLAR));
    let parted = city.chronicle.iter().find(|c| {
        c.subject() == e(FINN) && c.fact_type() == BOND && c.cause() == Cause::new("parted")
    });
    assert!(parted.is_some(), "they parted");
    assert!(pair(&city, FINN, BOND, GWEN).is_none());
    // With the bond gone, so is the longing.
    assert!(pair(&city, FINN, NEED, GWEN).is_none());
}
