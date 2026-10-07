//! A generated city on top of Ashford (Amendments A-23 to A-25): districts of homes and workshops,
//! each district a street with a gate onto the city's open ground, each home and workshop a door
//! onto its street — the Lanes, many times over. Most residents work in the next district along,
//! so every morning and evening people cross from district to district, and across the seam
//! wherever attention lies.
//!
//! Ids: places from [`FIRST_PLACE`], residents from [`FIRST_RESIDENT`], openings from
//! [`FIRST_OPENING`].

use packages::model::{PortalSpec, PositionSpec, ResidentSpec};
use packages::WorldPackage;

/// The first generated place: the city's open ground, which holds its districts.
pub const FIRST_PLACE: u64 = 10_000_000;
/// The first resident.
pub const FIRST_RESIDENT: u64 = 20_000_000;
/// The first opening.
pub const FIRST_OPENING: u64 = 30_000_000;
/// The first pantry (Amendment A-26): each home in a walkable district has one.
pub const FIRST_PANTRY: u64 = 40_000_000;
/// Where the city lies: in Ashford, well clear of its own places.
const ASHFORD: u64 = 10;

/// The shape of a generated city.
#[derive(Clone, Copy, Debug)]
pub struct CitySpec {
    /// How many residents.
    pub residents: u64,
    /// Homes to a district.
    pub homes_per_district: u64,
    /// Residents to a home.
    pub per_home: u64,
    /// Residents to a workshop.
    pub per_workshop: u64,
    /// How many districts, from the first, have doors on their homes and workshops — walkable in
    /// the scene. Every district has its gate; inside the rest, people are walked only at the city
    /// tier.
    pub walkable_districts: u64,
}

/// What was generated: the city's ground, and each district with its homes and workshops.
#[derive(Clone, Debug, Default)]
pub struct Generated {
    /// The city's open ground.
    pub ground: u64,
    /// Each district (a street), in order.
    pub districts: Vec<u64>,
    /// Each district's homes.
    pub homes: Vec<Vec<u64>>,
    /// Each district's workshops.
    pub workshops: Vec<Vec<u64>>,
    /// Each resident: `(id, home, work)`.
    pub residents: Vec<(u64, u64, Option<u64>)>,
}

impl Generated {
    /// The district holding `place`, a home or a workshop.
    pub fn district_of(&self, place: u64) -> Option<usize> {
        (0..self.districts.len())
            .find(|&d| self.homes[d].contains(&place) || self.workshops[d].contains(&place))
    }
}

/// Add a city shaped by `spec` to `pkg`, every place in it at the city tier.
pub fn city(pkg: &mut WorldPackage, spec: CitySpec) -> Generated {
    let mut next_place = FIRST_PLACE;
    let mut next_opening = FIRST_OPENING;
    let mut place = |pkg: &mut WorldPackage, parent: u64, at: (i64, i64)| {
        let id = next_place;
        next_place += 1;
        pkg.places.push((id, Some(parent)));
        pkg.positions.push(PositionSpec {
            entity_id: id,
            x: at.0,
            y: at.1,
            z: Some(0),
        });
        id
    };
    // A way between `outer` and `inner`, which lies at `at` in `outer`: an opening each side,
    // `inner`'s at `inside` in its own frame, paired.
    let mut door =
        |pkg: &mut WorldPackage, outer: u64, inner: u64, at: (i64, i64), inside: (i64, i64)| {
            let (out, back) = (next_opening, next_opening + 1);
            next_opening += 2;
            for (id, host, dest, (x, y)) in [(out, outer, inner, at), (back, inner, outer, inside)]
            {
                pkg.portals.push(PortalSpec {
                    portal_id: id,
                    host_region: host,
                    dest_region: dest,
                    x,
                    y,
                    z: None,
                });
            }
            pkg.portal_pairs.push((out, back));
        };

    let homes_total = spec.residents.div_ceil(spec.per_home.max(1));
    let districts = homes_total.div_ceil(spec.homes_per_district.max(1));
    let workshops_per_district = (spec.homes_per_district * spec.per_home)
        .div_ceil(spec.per_workshop.max(1))
        .max(1);
    let ground = place(pkg, ASHFORD, (200_000, 200_000));
    pkg.residency.push((ground, society::schema::CITY));
    let mut out = Generated {
        ground,
        ..Default::default()
    };
    for d in 0..districts {
        // Districts on a grid, 400 m apart.
        let (dx, dy) = ((d as i64 % 50) * 40_000, (d as i64 / 50) * 40_000);
        let district = place(pkg, ground, (dx, dy));
        let walkable = d < spec.walkable_districts;
        // Every district's gate: 10 m west along the street, onto the ground — so one walking
        // from the scene toward a district anywhere in the city knows the way out.
        door(pkg, ground, district, (dx - 1_000, dy), (-1_000, 0));
        // Homes north of the street, workshops south, 7 m apart, doors 3 m from their centres.
        let mut homes = Vec::new();
        for h in 0..spec.homes_per_district {
            let at = ((h as i64 % 40) * 700, 1_000 + (h as i64 / 40) * 2_000);
            let home = place(pkg, district, at);
            if walkable {
                door(pkg, district, home, (at.0, at.1 - 300), (0, -300));
            }
            pkg.larders.push((home, 6));
            if walkable {
                pkg.pantries.push((home, FIRST_PANTRY + home - FIRST_PLACE));
            }
            homes.push(home);
        }
        let mut workshops = Vec::new();
        for w in 0..workshops_per_district {
            let at = ((w as i64 % 40) * 700, -1_000 - (w as i64 / 40) * 2_000);
            let workshop = place(pkg, district, at);
            if walkable {
                door(pkg, district, workshop, (at.0, at.1 + 300), (0, 300));
            }
            workshops.push(workshop);
        }
        out.districts.push(district);
        out.homes.push(homes);
        out.workshops.push(workshops);
    }
    // Residents, three to a home; four in five work, in the next district along.
    let mut id = FIRST_RESIDENT;
    'all: for d in 0..out.districts.len() {
        let next = (d + 1) % out.districts.len();
        for &home in &out.homes[d] {
            for _ in 0..spec.per_home {
                if id >= FIRST_RESIDENT + spec.residents {
                    break 'all;
                }
                let i = id - FIRST_RESIDENT;
                let shops = &out.workshops[next];
                let work = (i % 5 != 0)
                    .then(|| shops[((i / spec.per_workshop.max(1)) as usize) % shops.len()]);
                pkg.residents.push(ResidentSpec {
                    id,
                    home,
                    work,
                    hunger: (i as i64 * 397) % 4_000,
                });
                out.residents.push((id, home, work));
                id += 1;
            }
        }
    }
    out
}
