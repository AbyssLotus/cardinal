//! Spatial queries over the containment hierarchy (Vol. III Ch. 1 §1.12, Querying Reality).
//!
//! Space is representation-independent (§1.4): a consumer asks a question — how far apart,
//! how high, where relative to, which way, by which route — and gets an answer without
//! depending on how space is stored. Here an entity's position is local to its immediate
//! container ([`crate::schema::POSITION`]), and composes up the containment hierarchy (via
//! `kernel::hierarchy`) so any two entities in one hierarchy have a relative position in the
//! frame of their lowest common container — a bedroom, a house, or a whole city.
//!
//! **Frames turn** (Amendment A-3). Each container's [`crate::schema::HEADING`] orients its
//! own frame within its parent: a child's local `+y` points along the container's heading. So a
//! position is lifted into the parent's frame by rotating it by the container's heading and
//! adding the container's own position — [`lift`], the one function every query composes
//! positions with, so every road computes the same centimetre. Rotations use the kernel's
//! integer sine and cosine and round to the centimetre at each level.
//!
//! **Positions are live** (Amendment A-3). A body in motion is wherever its segment puts it at
//! the view's tick ([`crate::motion`]); every query here sees that, with nothing written.

use crate::motion::position_at;
use crate::schema::{CONTAINED_IN, HAS_PORTAL, HEADING, LEADS_TO};
use kernel::fact::FactKey;
use kernel::fixed::{div_round, isqrt, sin_cos, TRIG_ONE};
use kernel::hierarchy::{ancestry, lowest_common_ancestor};
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// An entity's position within its immediate container at the view's tick — its base point,
/// wherever its motion puts it now. A body with no position fact is at the container's origin.
pub fn local_position(view: &dyn CommittedView, entity: EntityId) -> [i64; 3] {
    position_at(view, entity, view.tick())
}

/// An entity's facing within its container's frame, in hundredths of a degree clockwise from
/// north (`0..36000`); 0 if it declares none.
pub fn heading(view: &dyn CommittedView, entity: EntityId) -> i64 {
    view.read(FactKey::new(entity, HEADING))
        .and_then(|f| f.value.as_int())
        .unwrap_or(0)
        .rem_euclid(36_000)
}

/// Express `p`, given in a frame turned to compass `heading`, in the unturned parent frame. A
/// frame facing east (9000) maps its `+y` (ahead) to the parent's `+x` (east), and its `+x`
/// (right) to the parent's `−y` (south). Exact for heading 0; otherwise rounded to the
/// centimetre.
pub fn rotate(heading: i64, p: [i64; 3]) -> [i64; 3] {
    if heading.rem_euclid(36_000) == 0 {
        return p;
    }
    let (s, c) = sin_cos(heading);
    let (s, c, one) = (s as i128, c as i128, TRIG_ONE as i128);
    let (x, y) = (p[0] as i128, p[1] as i128);
    [
        div_round(x * c + y * s, one) as i64,
        div_round(y * c - x * s, one) as i64,
        p[2],
    ]
}

/// The inverse of [`rotate`]: express `p`, given in the parent frame, in a frame turned to
/// compass `heading`.
pub fn unrotate(heading: i64, p: [i64; 3]) -> [i64; 3] {
    rotate(-heading, p)
}

/// Lift `p` — a position in the frame of `frames[0]` — up through the frames listed, from the
/// innermost outward, into the frame that contains the last of them. At each step the point is
/// turned by that frame's heading and shifted by that frame's own position.
///
/// The single definition of composing positions: [`position_in`] and the indexed proximity
/// search in `crate::nearby` both use it, in the same order, so both reach exactly the same
/// centimetre — which is what lets the indexed answers equal the scanned ones.
pub fn lift(p: [i64; 3], frames: &[([i64; 3], i64)]) -> [i64; 3] {
    let mut p = p;
    for &(origin, heading) in frames {
        let turned = rotate(heading, p);
        p = [
            origin[0].saturating_add(turned[0]),
            origin[1].saturating_add(turned[1]),
            origin[2].saturating_add(turned[2]),
        ];
    }
    p
}

/// `entity`'s position expressed in `ancestor`'s coordinate frame, lifted through every
/// container between them (each turned by its heading). `None` if `ancestor` does not contain
/// `entity`.
///
/// Walks the kernel's cycle-safe [`ancestry`], so a malformed containment loop ends the walk
/// instead of hanging it (audit §7: the previous hand-rolled loop never returned when asked for
/// an ancestor that was not on a cyclic chain).
pub fn position_in(
    view: &dyn CommittedView,
    entity: EntityId,
    ancestor: EntityId,
) -> Option<[i64; 3]> {
    let chain = ancestry(view, entity, CONTAINED_IN);
    let stop = chain.iter().position(|e| *e == ancestor)?;
    if stop == 0 {
        return Some([0; 3]);
    }
    let frames: Vec<([i64; 3], i64)> = chain[1..stop]
        .iter()
        .map(|&f| (local_position(view, f), heading(view, f)))
        .collect();
    Some(lift(local_position(view, entity), &frames))
}

/// The outermost container of `entity` — the root of its containment hierarchy, whose frame
/// every position in the hierarchy can be expressed in. An entity with no container is its
/// own root.
pub fn root_of(view: &dyn CommittedView, entity: EntityId) -> EntityId {
    *ancestry(view, entity, CONTAINED_IN)
        .last()
        .expect("ancestry always includes the entity itself")
}

/// `entity`'s facing in the frame of `ancestor`: its own heading plus that of every container
/// between, since each container turns the frame of what it holds. `None` if `ancestor` does
/// not contain `entity`.
pub fn heading_in(view: &dyn CommittedView, entity: EntityId, ancestor: EntityId) -> Option<i64> {
    let chain = ancestry(view, entity, CONTAINED_IN);
    let stop = chain.iter().position(|e| *e == ancestor)?;
    Some(
        chain[..stop]
            .iter()
            .map(|&e| heading(view, e))
            .sum::<i64>()
            .rem_euclid(36_000),
    )
}

/// The displacement from `from` to `to`, expressed in the frame of their lowest common
/// containing region (Vol. III Ch. 1 §1.8). `None` if they share no common container -- e.g.
/// one is not loaded into a shared hierarchy.
pub fn relative_position(
    view: &dyn CommittedView,
    from: EntityId,
    to: EntityId,
) -> Option<[i64; 3]> {
    let lca = lowest_common_ancestor(view, from, to, CONTAINED_IN)?;
    let from_pos = position_in(view, from, lca)?;
    let to_pos = position_in(view, to, lca)?;
    Some([
        to_pos[0] - from_pos[0],
        to_pos[1] - from_pos[1],
        to_pos[2] - from_pos[2],
    ])
}

/// The straight-line distance between `from` and `to` in centimetres, or `None` if they share
/// no common container. Exact integer arithmetic (i128 intermediate), so it is deterministic.
pub fn distance(view: &dyn CommittedView, from: EntityId, to: EntityId) -> Option<i64> {
    let d = relative_position(view, from, to)?;
    let sq = (d[0] as i128).pow(2) + (d[1] as i128).pow(2) + (d[2] as i128).pow(2);
    Some(isqrt(sq as u128) as i64)
}

/// The compass bearing from `from` to `to` (hundredths of a degree clockwise from north), in
/// the frame of their lowest common container — "the well is to the north-east". `None` if they
/// share no container or stand at the same spot.
pub fn bearing_to(view: &dyn CommittedView, from: EntityId, to: EntityId) -> Option<i64> {
    let d = relative_position(view, from, to)?;
    crate::motion::compass(d[0], d[1])
}

/// Where `to` lies relative to the way `observer` is facing, in hundredths of a degree in
/// `-17999..=18000`: 0 dead ahead, positive to the right (9000 is directly right), negative to
/// the left, 18000 directly behind (Vol. III Ch. 1 §1.6, Relative Position). `None` if they
/// share no container or stand at the same spot. This is what turns coordinates into "the door
/// is on your left".
pub fn relative_bearing(view: &dyn CommittedView, observer: EntityId, to: EntityId) -> Option<i64> {
    let lca = lowest_common_ancestor(view, observer, to, CONTAINED_IN)?;
    let bearing = bearing_to(view, observer, to)?;
    let facing = heading_in(view, observer, lca)?;
    let mut rel = (bearing - facing).rem_euclid(36_000);
    if rel > 18_000 {
        rel -= 36_000;
    }
    Some(rel)
}

/// How far `entity` is above the ground, in centimetres (Vol. III Ch. 1 §1.6, "Above" /
/// "Below"): its height in the frame of its hierarchy's root, whose origin is the ground datum.
/// Someone standing on a second-storey floor stacked 3 m up a house reads 300; someone in a
/// cellar sunk 3 m reads −300 (below ground). An entity with no container is the ground frame
/// itself and reads 0. Headings turn frames about the vertical, so they never change a height.
///
/// This is height within the containment hierarchy, not terrain: [`crate::schema::ELEVATION`]
/// says how high the ground itself stands above the world datum.
pub fn height_above_ground(view: &dyn CommittedView, entity: EntityId) -> i64 {
    let root = root_of(view, entity);
    position_in(view, entity, root).map_or(0, |p| p[2])
}

// ---- Connectivity: portals ---------------------------------------------------------------
//
// A portal is a located connection from a spot in one region to another region (Vol. III
// Ch. 1 §1.5). Connectivity is NOT adjacency: two regions may border yet be joined by no
// portal ("adjacent yet effectively disconnected", §1.5). These queries answer "can I get
// from here to there, and by what steps" -- the navigational question, distinct from
// straight-line distance.

/// The portals a region hosts -- its exits.
pub fn portals_in(view: &dyn CommittedView, region: EntityId) -> Vec<EntityId> {
    view.read_all(FactKey::new(region, HAS_PORTAL))
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(portal) => Some(portal),
            _ => None,
        })
        .collect()
}

/// The region a portal leads to (its far side), or `None` if it currently leads nowhere.
pub fn portal_destination(view: &dyn CommittedView, portal: EntityId) -> Option<EntityId> {
    match view.read(FactKey::new(portal, LEADS_TO)).map(|f| f.value) {
        Some(Value::Entity(dest)) => Some(dest),
        _ => None,
    }
}

/// The regions directly reachable from `region` by stepping through one of its portals
/// (deduplicated, sorted). One hop only.
pub fn destinations(view: &dyn CommittedView, region: EntityId) -> Vec<EntityId> {
    let mut set = BTreeSet::new();
    for portal in portals_in(view, region) {
        if let Some(dest) = portal_destination(view, portal) {
            set.insert(dest);
        }
    }
    set.into_iter().collect()
}

/// Every region reachable from `origin` by traversing portals, including `origin` itself
/// (Vol. III Ch. 1 §1.6, "Reachable"). A breadth-first walk of the portal graph; cycles are
/// handled by the visited set. This is what a sealed room fails and a room with a staircase
/// passes -- and it routes a basement to the yard only *through* the ground floor, because
/// that is the only portal path.
pub fn reachable_regions(view: &dyn CommittedView, origin: EntityId) -> BTreeSet<EntityId> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    seen.insert(origin);
    queue.push_back(origin);
    while let Some(region) = queue.pop_front() {
        for dest in destinations(view, region) {
            if seen.insert(dest) {
                queue.push_back(dest);
            }
        }
    }
    seen
}

/// Whether `to` can be reached from `from` by traversing portals (Vol. III Ch. 1 §1.6,
/// "Reachable"). `true` for a region and itself.
pub fn can_reach(view: &dyn CommittedView, from: EntityId, to: EntityId) -> bool {
    reachable_regions(view, from).contains(&to)
}

/// The portals to step through, in order, to get from region `from` to region `to`
/// (Vol. III Ch. 1 §1.6, "Reachable" -- and by which way). `Some(vec![])` when `from` is
/// `to`; `None` when no chain of portals leads there (a sealed vault, or an upstairs window
/// seen from the yard it only opens onto).
///
/// A breadth-first walk of the portal graph that remembers how each region was first reached,
/// then reads the route back from `to`. It returns a route with the fewest steps; among
/// equally short routes, the one through the lowest-numbered portals, because portals are
/// visited in id order -- so the answer is deterministic (Vol. V Ch. 4 §4.1).
///
/// Fewest steps, not safest: from an upstairs room the shortest way out may be a window with
/// a 4 m drop. Physical Reality reports the ways and their [`crate::schema::PORTAL_DANGER`];
/// weighing one against the other is a decision, and decisions belong to the domains that
/// make them (§1.3).
pub fn route(view: &dyn CommittedView, from: EntityId, to: EntityId) -> Option<Vec<EntityId>> {
    if from == to {
        return Some(Vec::new());
    }
    // region -> (the region it was first reached from, the portal used)
    let mut came_by: BTreeMap<EntityId, (EntityId, EntityId)> = BTreeMap::new();
    let mut queue = VecDeque::from([from]);
    while let Some(region) = queue.pop_front() {
        for portal in portals_in(view, region) {
            let Some(dest) = portal_destination(view, portal) else {
                continue;
            };
            if dest == from || came_by.contains_key(&dest) {
                continue;
            }
            came_by.insert(dest, (region, portal));
            if dest == to {
                let mut path = Vec::new();
                let mut here = to;
                while here != from {
                    let (previous, via) = came_by[&here];
                    path.push(via);
                    here = previous;
                }
                path.reverse();
                return Some(path);
            }
            queue.push_back(dest);
        }
    }
    None
}
