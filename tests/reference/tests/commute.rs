//! A day in a generated city with people constantly crossing the seam (Amendments A-24, A-25):
//! four districts, each a street of homes and workshops with a gate onto the city's ground, and
//! most residents working in the next district along. Attention lies on two of them. All day
//! people come into the scene and go out of it — waking at home in it, leaving it for work,
//! arriving at work in it, going home out of it, coming home into it — and at every tick each
//! is exactly one thing, a record or an organism, as hungry the tick after a crossing as the tick
//! before — and those living in the scene eat from their households' pantries (Amendment A-26).

use kernel::level::Level;
use kernel::store::RealityStore;
use kernel::time::SimClock;
use kernel::value::Value;
use reference::generate::{self, CitySpec, Generated};
use reference::{with_tick_seconds, City};
use society::schema::{
    RESIDENT_AT, RESIDENT_DOING, RESIDENT_HOME, RESIDENT_HUNGER, SCENE, WALKING, WORKING,
};

const HOUR: u64 = 60; // minute ticks
const CLOCK: SimClock = SimClock::new(60_000);

/// The generated town, with attention on its first and third districts.
fn town() -> (City, Generated) {
    let mut pkg = with_tick_seconds(60);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    let town = generate::city(
        &mut pkg,
        CitySpec {
            residents: 96,
            homes_per_district: 8,
            per_home: 3,
            per_workshop: 10,
            walkable_districts: 4,
        },
    );
    let mut city = City::from(pkg);
    city.attend(town.districts[0], SCENE)
        .attend(town.districts[2], SCENE);
    (city, town)
}

/// Which tier `who` is at — `Some(true)` an organism, `Some(false)` a record — and the hunger
/// level they hold there.
fn tier(city: &City, who: u64) -> (Option<bool>, Option<Level>) {
    let record = city.read(who, RESIDENT_HUNGER);
    let body = city.read(who, living::schema::HUNGER);
    let level = |v: Option<Value>| v.and_then(Level::from_value);
    match (record, body) {
        (Some(_), None) => (Some(false), level(record)),
        (None, Some(_)) => (Some(true), level(body)),
        _ => (None, None),
    }
}

fn in_scene(town: &Generated, place: u64) -> bool {
    matches!(town.district_of(place), Some(0 | 2))
}

#[test]
fn a_day_of_people_passing_between_districts_and_across_the_seam() {
    let (mut city, town) = town();
    let people: Vec<u64> = town.residents.iter().map(|r| r.0).collect();
    let mut was: Vec<(Option<bool>, Option<Level>)> =
        people.iter().map(|&w| tier(&city, w)).collect();
    let (mut into, mut out_of) = ([0u32; 24], [0u32; 24]);
    let (mut meals, mut hungriest, mut seen) = (0, 0, 0);

    for _ in 0..24 * HOUR {
        city.run(1);
        let hour = ((city.tick - 1) / HOUR) as usize;
        for c in &city.chronicle[seen..] {
            if c.fact_type() == physical::schema::PICKED {
                meals += 1;
            }
        }
        seen = city.chronicle.len();
        for (i, &who) in people.iter().enumerate() {
            let now = tier(&city, who);
            // One person, one tier at a time (Ruling 19): never both, never neither.
            let (Some(organism), Some(level)) = now else {
                panic!("{who} is both or neither at tick {}", city.tick);
            };
            hungriest = hungriest.max(level.at(CLOCK, city.tick, 0, 10_000));
            let (before, held) = was[i];
            if before != Some(organism) {
                // A crossing, either way: as hungry after as before, to the unit.
                let held = held.expect("held a level before");
                assert_eq!(
                    level.at(CLOCK, city.tick, 0, 10_000),
                    held.at(CLOCK, city.tick, 0, 10_000),
                    "{who}'s hunger jumped crossing at tick {}",
                    city.tick
                );
                if organism {
                    into[hour] += 1;
                } else {
                    out_of[hour] += 1;
                }
            }
            was[i] = now;
        }
        // Eleven in the morning: everyone with work is at it, at whichever tier their workshop
        // is — organisms in the scene's workshops, records working at the city's.
        if city.tick == 11 * HOUR {
            for &(who, _, work) in &town.residents {
                let Some(work) = work else { continue };
                if in_scene(&town, work) {
                    assert_eq!(
                        city.read(who, physical::schema::CONTAINED_IN),
                        Some(Value::Entity(reference::e(work))),
                        "{who} at work in the scene"
                    );
                } else {
                    assert_eq!(city.int(who, RESIDENT_DOING), Some(WORKING), "{who}");
                    assert_eq!(
                        city.read(who, RESIDENT_AT),
                        Some(Value::Entity(reference::e(work)))
                    );
                }
            }
        }
    }

    // All day long, people came and went: the scene's own waking at midnight, the morning's
    // leaving and arriving, the evening's.
    let (ins, outs): (u32, u32) = (into.iter().sum(), out_of.iter().sum());
    let morning = |counts: &[u32; 24]| counts[6..10].iter().sum::<u32>();
    let evening = |counts: &[u32; 24]| counts[15..20].iter().sum::<u32>();
    assert!(
        into[0] >= 40,
        "the scene's districts at midnight: {}",
        into[0]
    );
    assert!(
        morning(&into) >= 25 && morning(&out_of) >= 25,
        "{into:?} {out_of:?}"
    );
    assert!(
        evening(&into) >= 25 && evening(&out_of) >= 25,
        "{into:?} {out_of:?}"
    );
    assert!(ins >= 120 && outs >= 60, "{ins} in, {outs} out");
    // Those whose homes are in the scene ate there, from their households' pantries; nobody
    // came near starving at either tier.
    assert!(meals > 0, "no meal taken from a pantry");
    assert!(hungriest < 9_000, "hungriest {hungriest}");

    // And at the end of the day everyone is home, at whichever tier home is.
    for &(who, home, _) in &town.residents {
        let home_value = Some(Value::Entity(reference::e(home)));
        if in_scene(&town, home) {
            assert_eq!(
                city.read(who, physical::schema::CONTAINED_IN),
                home_value,
                "{who} home in the scene"
            );
        } else {
            assert_eq!(city.read(who, RESIDENT_AT), home_value, "{who} home");
            assert_ne!(city.int(who, RESIDENT_DOING), Some(WALKING), "{who}");
        }
        assert_eq!(
            city.read(who, RESIDENT_HOME),
            home_value,
            "{who} keeps their home"
        );
    }
    let _ = city.store().state_hash();
}
