//! The city-tier bench (Amendment A-23): Ashford with `N` generated residents at the city tier —
//! districts of homes and workshops, three to a home, most of them working — run for a day.
//! Reports what loading costs, what a tick costs on average and at worst, and how many facts the
//! city holds: whether a city of a million fits a desktop.
//!
//! ```text
//! TICK_SECONDS=60 cargo run --release -p reference --example city -- 100000 1000000
//! ```

use kernel::store::RealityStore;
use packages::model::{PositionSpec, ResidentSpec};
use packages::WorldPackage;
use reference::{with_tick_seconds, City};
use std::time::Instant;

/// The first generated place's id, and the first resident's.
const FIRST_PLACE: u64 = 10_000_000;
const FIRST_RESIDENT: u64 = 20_000_000;
/// Homes to a district, and residents to a home.
const HOMES_PER_DISTRICT: u64 = 1_000;
const PER_HOME: u64 = 3;
/// Residents to a workshop.
const PER_WORKSHOP: u64 = 20;
/// Where the generated districts lie: Ashford, in the Vale.
const ASHFORD: u64 = 10;

/// Ashford with a generated city of `n` residents.
fn city(n: u64, tick_seconds: u64) -> WorldPackage {
    let mut pkg = with_tick_seconds(tick_seconds);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    let homes = n.div_ceil(PER_HOME);
    let districts = homes.div_ceil(HOMES_PER_DISTRICT);
    let workshops_per_district = (HOMES_PER_DISTRICT * PER_HOME).div_ceil(PER_WORKSHOP);
    let mut next = FIRST_PLACE;
    let mut place = |pkg: &mut WorldPackage, parent: u64, x: i64, y: i64| {
        let id = next;
        next += 1;
        pkg.places.push((id, Some(parent)));
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x,
            y,
            z: Some(0),
        });
        id
    };
    let mut resident = FIRST_RESIDENT;
    for d in 0..districts {
        // Districts on a grid, 400 m apart, well clear of Ashford's own places.
        let (dx, dy) = (
            200_000 + (d as i64 % 50) * 40_000,
            200_000 + (d as i64 / 50) * 40_000,
        );
        let district = place(&mut pkg, ASHFORD, dx, dy);
        let workshops: Vec<u64> = (0..workshops_per_district)
            .map(|w| place(&mut pkg, district, (w as i64 % 15) * 2_000, 30_000))
            .collect();
        for h in 0..HOMES_PER_DISTRICT {
            if resident >= FIRST_RESIDENT + n {
                break;
            }
            let home = place(
                &mut pkg,
                district,
                (h as i64 % 40) * 700,
                (h as i64 / 40) * 1_000,
            );
            pkg.larders.push((home, 6));
            for _ in 0..PER_HOME {
                if resident >= FIRST_RESIDENT + n {
                    break;
                }
                let i = resident - FIRST_RESIDENT;
                // Four in five work, at a workshop in their district.
                let work =
                    (i % 5 != 0).then(|| workshops[(i / PER_WORKSHOP) as usize % workshops.len()]);
                pkg.residents.push(ResidentSpec {
                    id: resident,
                    home,
                    work,
                    hunger: (i as i64 * 397) % 4_000,
                });
                resident += 1;
            }
        }
    }
    pkg
}

fn main() {
    let sizes: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![10_000, 100_000]
    } else {
        sizes
    };
    let tick_seconds: u64 = std::env::var("TICK_SECONDS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(60);
    let ticks = 86_400 / tick_seconds;
    println!(
        "{:>9} {:>9} {:>9} {:>10} {:>10} {:>12} {:>11}",
        "residents", "build s", "load s", "ms/tick", "worst ms", "segments/s", "facts"
    );
    for n in sizes {
        let t0 = Instant::now();
        let pkg = city(n, tick_seconds);
        let built = t0.elapsed();
        let t1 = Instant::now();
        let mut city = City::from(pkg);
        let loaded = t1.elapsed();
        let (mut total, mut worst, mut segments) = (0.0f64, 0.0f64, 0u64);
        for _ in 0..ticks {
            city.chronicle.clear();
            let t = Instant::now();
            city.run(1);
            let ms = t.elapsed().as_secs_f64() * 1_000.0;
            total += ms;
            worst = worst.max(ms);
            segments += city
                .chronicle
                .iter()
                .filter(|c| c.fact_type() == society::schema::RESIDENT_UNTIL)
                .count() as u64;
        }
        println!(
            "{:>9} {:>9.1} {:>9.1} {:>10.2} {:>10.2} {:>12.1} {:>11}",
            n,
            built.as_secs_f64(),
            loaded.as_secs_f64(),
            total / ticks as f64,
            worst,
            segments as f64 / 86_400.0,
            city.store().len()
        );
        let _ = city.store().state_hash();
    }
}
