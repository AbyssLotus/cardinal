//! What is near, and what is inside: proximity and contents queries (Vol. III Ch. 1 §1.6
//! "What is nearby?", §1.8 "What is inside this building?", §1.12 Querying Reality;
//! Amendment A-2).
//!
//! These are the questions a text world asks constantly — what does the player see in this
//! room, who is within earshot, what is the nearest door — and they must be cheap. Each query
//! here has two roads to the same answer:
//!
//! - **Indexed**, when the store keeps a spatial index (`CommittedView::spatial`): candidates
//!   come from the per-container grids, and only they are measured.
//! - **Scanned**, when it does not: every placed entity is measured.
//!
//! Both roads use the same exact test, on the same coordinates, and return the same sorted
//! list — the conformance rule of Amendment A-2, held by `tests/spatial_conformance.rs`.
//!
//! **Distance across rooms.** An entity's position is local to its container, so two entities
//! in different rooms are compared in the frame of their hierarchy's root: positions compose up
//! the containment chain ([`crate::space::position_in`]). Proximity is purely geometric — a
//! wall does not make the person on its far side any farther away. Whether they can be *seen* or
//! *reached* are separate questions with separate answers ([`crate::space::can_reach`]).

use crate::index::{container_of, occupied};
use crate::schema::CONTAINED_IN;
use crate::space::{position_in, root_of};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::spatial::{Aabb, SpatialQuery};
use kernel::system::CommittedView;

/// Everything placed directly inside `container`, sorted by id — "you see here…". Not
/// recursive: the sword in the chest is in the chest, not in the room.
pub fn contents(view: &dyn CommittedView, container: EntityId) -> Vec<EntityId> {
    match view.spatial() {
        Some(index) => index.children(container),
        None => view
            .entities_with(CONTAINED_IN)
            .into_iter()
            .filter(|e| container_of(view, *e) == Some(container))
            .collect(),
    }
}

/// The entities placed directly in `frame` whose occupied box intersects `bounds` (local
/// coordinates of `frame`), sorted by id — "what is in this corner of the room".
pub fn in_box(view: &dyn CommittedView, frame: EntityId, bounds: &Aabb) -> Vec<EntityId> {
    let hits = |e: &EntityId| {
        occupied(view, *e).is_some_and(|(f, _, occupies)| f == frame && occupies.intersects(bounds))
    };
    match view.spatial() {
        Some(index) => index
            .candidates(frame, bounds)
            .into_iter()
            .map(|(e, _)| e)
            .filter(hits)
            .collect(),
        None => contents(view, frame).into_iter().filter(hits).collect(),
    }
}

/// Straight-line distance between two points, exact to the centimetre (integer square root of
/// an `i128` sum, so it never overflows and never differs between platforms).
fn distance(a: [i64; 3], b: [i64; 3]) -> i64 {
    let sq: i128 = (0..3).map(|i| (a[i] as i128 - b[i] as i128).pow(2)).sum();
    isqrt(sq as u128) as i64
}

/// Every entity within `radius` (centimetres, inclusive) of `center`, anywhere in `center`'s
/// containment hierarchy, with its distance — sorted nearest first, ties by id. Excludes
/// `center` itself. "Who is within earshot?"
pub fn within(view: &dyn CommittedView, center: EntityId, radius: i64) -> Vec<(EntityId, i64)> {
    let mut found = match view.spatial() {
        Some(index) => {
            let Some((root, at)) = locate(view, index, center) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            search(view, index, center, root, [0; 3], at, radius, &mut out);
            out
        }
        None => {
            let root = root_of(view, center);
            let Some(at) = position_in(view, center, root) else {
                return Vec::new();
            };
            view.entities_with(CONTAINED_IN)
                .into_iter()
                .filter(|e| *e != center && root_of(view, *e) == root)
                .filter_map(|e| {
                    let d = distance(at, position_in(view, e, root)?);
                    (d <= radius).then_some((e, d))
                })
                .collect()
        }
    };
    found.sort_by_key(|&(e, d)| (d, e));
    found
}

/// The root of `entity`'s hierarchy and `entity`'s position in the root's frame, read from the
/// index: a walk up the placements summing anchors — the same walk and the same sums as
/// [`root_of`] and [`position_in`] over committed facts, because a settled placement's frame
/// and anchor *are* the entity's container and local position. Falls back to the facts when the
/// entity is not placed or is in motion. Stops at the first repeated entity, as the kernel's
/// ancestry walk does, so a malformed containment cycle cannot hang it.
fn locate(
    view: &dyn CommittedView,
    index: &dyn SpatialQuery,
    entity: EntityId,
) -> Option<(EntityId, [i64; 3])> {
    let mut here = entity;
    let mut at = [0i64; 3];
    let mut seen: Vec<EntityId> = vec![entity];
    while let Some(p) = index.placement(here) {
        if !p.settled {
            let root = root_of(view, entity);
            return Some((root, position_in(view, entity, root)?));
        }
        if seen.contains(&p.frame) {
            break;
        }
        for i in 0..3 {
            at[i] = at[i].saturating_add(p.anchor[i]);
        }
        seen.push(p.frame);
        here = p.frame;
    }
    Some((here, at))
}

/// The indexed road of [`within`]: measure the candidates in `frame` (whose origin sits at
/// `origin` in the root's coordinates), then descend into every frame placed in it.
///
/// Every placed entity lives in exactly one frame, and every frame that holds anything is a
/// subframe of its own container, so descending from the root visits each candidate exactly
/// once. A frame is entered whether or not its own point is in range — a room's contents can be
/// near even when the room's origin is not.
#[allow(clippy::too_many_arguments)]
fn search(
    view: &dyn CommittedView,
    index: &dyn SpatialQuery,
    center: EntityId,
    frame: EntityId,
    origin: [i64; 3],
    at: [i64; 3],
    radius: i64,
    out: &mut Vec<(EntityId, i64)>,
) {
    // The query sphere's bounding box, in this frame's local coordinates.
    let local_at = [at[0] - origin[0], at[1] - origin[1], at[2] - origin[2]];
    let query = Aabb::around(local_at, radius);
    for (e, placement) in index.candidates(frame, &query) {
        if e == center {
            continue;
        }
        let a = placement.anchor;
        let p = [origin[0] + a[0], origin[1] + a[1], origin[2] + a[2]];
        let d = distance(at, p);
        if d <= radius {
            out.push((e, d));
        }
    }
    for sub in index.subframes(frame) {
        if let Some(placement) = index.placement(sub) {
            let a = placement.anchor;
            let sub_origin = [origin[0] + a[0], origin[1] + a[1], origin[2] + a[2]];
            search(view, index, center, sub, sub_origin, at, radius, out);
        }
    }
}

/// The `k` entities nearest `center` (no farther than `max_radius`), with their distances,
/// nearest first, ties by id. "What is the nearest door?" — after filtering — or simply "who is
/// closest".
///
/// Searches outward in doubling radii, so a crowded room answers from a small search and an
/// empty plain widens until it finds someone or reaches `max_radius`.
pub fn nearest(
    view: &dyn CommittedView,
    center: EntityId,
    k: usize,
    max_radius: i64,
) -> Vec<(EntityId, i64)> {
    if k == 0 {
        return Vec::new();
    }
    let mut radius = 100i64.min(max_radius.max(0));
    loop {
        let found = within(view, center, radius);
        if found.len() >= k || radius >= max_radius {
            return found.into_iter().take(k).collect();
        }
        radius = radius.saturating_mul(2).min(max_radius);
    }
}
