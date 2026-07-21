//! Spatial queries over the containment hierarchy (Vol. III Ch. 1 §1.12, Querying Reality).
//!
//! Space is representation-independent (§1.4): a consumer asks a question -- how far apart,
//! where relative to -- and gets an answer without depending on how space is stored. Here
//! positions are local coordinates within each entity's immediate container
//! ([`crate::schema::POSITION_X`] etc.), composed up the containment hierarchy (via
//! `kernel::hierarchy`) so any two loaded entities have a relative position in the frame of
//! their lowest common region -- a bedroom, a house, or a whole city, whichever encloses both.
//!
//! Frames are assumed axis-aligned; rotation/orientation between frames is a later refinement.

use crate::schema::{CONTAINED_IN, HAS_PORTAL, LEADS_TO, POSITION_X, POSITION_Y, POSITION_Z};
use kernel::fact::{FactKey, FactType};
use kernel::hierarchy::lowest_common_ancestor;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use std::collections::{BTreeSet, VecDeque};

const AXES: [FactType; 3] = [POSITION_X, POSITION_Y, POSITION_Z];

/// An entity's local position within its immediate container. A missing axis reads as 0 --
/// the container's origin.
fn local_position(view: &dyn CommittedView, entity: EntityId) -> [i64; 3] {
    let mut p = [0i64; 3];
    for (i, axis) in AXES.iter().enumerate() {
        p[i] = view
            .read(FactKey::new(entity, *axis))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0);
    }
    p
}

/// `entity`'s position expressed in `ancestor`'s coordinate frame: the sum of local positions
/// from `entity` up to (but not including) `ancestor`. `None` if `ancestor` does not contain
/// `entity`.
pub fn position_in(
    view: &dyn CommittedView,
    entity: EntityId,
    ancestor: EntityId,
) -> Option<[i64; 3]> {
    let mut sum = [0i64; 3];
    let mut here = entity;
    loop {
        if here == ancestor {
            return Some(sum);
        }
        let local = local_position(view, here);
        for (s, l) in sum.iter_mut().zip(local) {
            *s = s.saturating_add(l);
        }
        match view.read(FactKey::new(here, CONTAINED_IN)).map(|f| f.value) {
            Some(Value::Entity(parent)) => here = parent,
            _ => return None,
        }
    }
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
    Some(isqrt(sq) as i64)
}

/// Floor of the integer square root of a non-negative `i128`, by binary search.
fn isqrt(n: i128) -> i128 {
    if n < 2 {
        return n.max(0);
    }
    let mut hi: i128 = 1;
    while hi.saturating_mul(hi) <= n {
        hi = hi.saturating_mul(2);
    }
    let mut lo: i128 = hi / 2;
    while lo < hi {
        let mid = lo + (hi - lo + 1) / 2;
        if mid.saturating_mul(mid) <= n {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    lo
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
