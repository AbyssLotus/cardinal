//! The scaling bench: Ashford with `N` villagers added to the yard, each with a mind, senses, and
//! hunger, and a fruit tree for every ten of them, each villager knowing the nearest. Runs a
//! simulated day at ten-minute ticks and reports, for each `N`, the cost of a tick, the chronicle
//! it writes, and how the store grows.
//!
//! ```text
//! cargo run --release -p reference --example scale -- 10 100 1000
//! ```

use kernel::store::RealityStore;
use packages::model::{
    BodySpec, ContainmentSpec, DepositSpec, Flag, FlagSpec, OrganismSpec, PositionSpec,
};
use packages::WorldPackage;
use reference::{with_tick_seconds, City};
use std::time::Instant;

/// Where the villagers live: Ashford's yard, an open place under the Vale's sky.
const YARD: u64 = 100;
/// The first villager's id, and the first tree's.
const FIRST_VILLAGER: u64 = 100_000;
const FIRST_TREE: u64 = 900_000;
/// How far apart the villagers stand, cm: a village, not a crowd.
const SPACING: i64 = 1_000;
const APPLE: u64 = 704;

/// Ashford, with `n` villagers on a square grid in the yard and a tree for every ten.
fn village(n: u64) -> WorldPackage {
    let mut pkg = with_tick_seconds(600);
    pkg.physical_rules.temperature_variability_centi_c = 0;
    let side = (n as f64).sqrt().ceil() as i64;
    let at = |i: i64| (5_000 + (i % side) * SPACING, 5_000 + (i / side) * SPACING);
    let trees = n.div_ceil(10).max(1);
    for t in 0..trees {
        let id = FIRST_TREE + t;
        // Each tree stands among the ten villagers it feeds, just off the grid.
        let (x, y) = at((t * 10) as i64);
        pkg.containment.push(ContainmentSpec {
            child_id: id,
            parent_id: YARD,
        });
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x: x + SPACING / 2,
            y: y + SPACING / 2,
            z: Some(0),
        });
        pkg.bodies.push(BodySpec {
            entity_id: id,
            half_width: 50,
            half_depth: 50,
            height: 400,
        });
        pkg.flags.push(FlagSpec {
            entity_id: id,
            flags: vec![Flag::Solid],
        });
        pkg.deposits.push(DepositSpec {
            id,
            made_of: APPLE,
            size: [4, 4, 8],
            per_day: 20,
            cap: 30,
            stock: 10,
        });
    }
    for i in 0..n {
        let id = FIRST_VILLAGER + i;
        let (x, y) = at(i as i64);
        pkg.organisms.push(OrganismSpec {
            id,
            region_id: YARD,
            body_heat_centi_c: 3_700,
        });
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x,
            y,
            z: Some(0),
        });
        pkg.bodies.push(BodySpec {
            entity_id: id,
            half_width: 25,
            half_depth: 15,
            height: 175,
        });
        pkg.flags.push(FlagSpec {
            entity_id: id,
            flags: vec![Flag::Mobile],
        });
        pkg.senses.push((id, 3_000));
        pkg.minds.push((id, 140));
        pkg.knows.push((id, vec![YARD, FIRST_TREE + i / 10]));
        // Hungry at different hours, so they do not all go to eat at once.
        pkg.hunger.push((id, (i as i64 * 397) % 4_000));
    }
    pkg
}

/// The ten fact types holding the most values, and how many each holds.
fn census(city: &City) {
    use kernel::fact::FactKey;
    use kernel::system::CommittedView;
    let mut types: std::collections::BTreeSet<kernel::fact::FactType> = Default::default();
    for e in city.store().entities_with(physical::schema::CONTAINED_IN) {
        for (k, _) in city.store().facts_of(e) {
            types.insert(k.fact_type);
        }
    }
    for e in city.store().entities_with(minds::schema::WALK_SPEED) {
        for (k, _) in city.store().facts_of(e) {
            types.insert(k.fact_type);
        }
    }
    let mut counts: Vec<(usize, &str)> = types
        .iter()
        .map(|t| {
            let n: usize = city
                .store()
                .entities_with(*t)
                .into_iter()
                .map(|e| {
                    let pairs = city.store().read_about(e, *t).len();
                    if pairs > 0 {
                        pairs
                    } else {
                        city.store().read_all(FactKey::new(e, *t)).len()
                    }
                })
                .sum();
            (n, t.name())
        })
        .collect();
    counts.sort_by(|a, b| b.0.cmp(&a.0));
    for (n, t) in counts.iter().take(10) {
        println!("    {n:>8} {t}");
    }
}

fn main() {
    let sizes: Vec<u64> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse().ok())
        .collect();
    let sizes = if sizes.is_empty() {
        vec![10, 100, 1_000]
    } else {
        sizes
    };
    let ticks: u64 = std::env::var("TICKS")
        .ok()
        .and_then(|t| t.parse().ok())
        .unwrap_or(144);
    println!(
        "{:>7} {:>9} {:>10} {:>10} {:>12} {:>10} {:>10}",
        "agents", "load ms", "ms/tick", "worst ms", "entries/tick", "facts 0", "facts end"
    );
    for n in sizes {
        let pkg = village(n);
        let t0 = Instant::now();
        let mut city = City::from(pkg);
        let load = t0.elapsed();
        let facts0 = city.store().len();
        let (mut total, mut worst, mut entries) = (0.0f64, 0.0f64, 0usize);
        let mut by_system: std::collections::BTreeMap<&str, (u128, usize)> = Default::default();
        for _ in 0..ticks {
            city.chronicle.clear();
            let t = Instant::now();
            city.run(1);
            let ms = t.elapsed().as_secs_f64() * 1_000.0;
            total += ms;
            worst = worst.max(ms);
            entries += city.chronicle.len();
            for m in city.meters() {
                let e = by_system.entry(m.system.name()).or_default();
                e.0 += m.nanos;
                e.1 += m.proposals;
            }
        }
        println!(
            "{:>7} {:>9.1} {:>10.2} {:>10.2} {:>12} {:>10} {:>10}",
            n + 27,
            load.as_secs_f64() * 1_000.0,
            total / ticks as f64,
            worst,
            entries / ticks as usize,
            facts0,
            city.store().len()
        );
        let _ = city.store().state_hash();
        if std::env::var("METERS").is_ok() {
            let mut rows: Vec<_> = by_system.into_iter().collect();
            rows.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
            for (name, (nanos, proposals)) in rows.iter().take(8) {
                println!(
                    "    {:>8.2} ms/tick {:>7} proposals/tick  {name}",
                    *nanos as f64 / 1e6 / ticks as f64,
                    proposals / ticks as usize
                );
            }
        }
        if std::env::var("CENSUS").is_ok() {
            census(&city);
        }
    }
}
