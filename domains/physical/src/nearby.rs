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
use crate::space::{heading, lift, local_position, position_in, root_of, unrotate};
use kernel::fixed::isqrt;
use kernel::hierarchy::ancestry;
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
/// `center` itself and the containers it is inside: a person is *in* the bedroom, not near it.
/// Distances are between base points at the view's tick, so a body walking past is found where
/// it is now. "Who is within earshot?"
pub fn within(view: &dyn CommittedView, center: EntityId, radius: i64) -> Vec<(EntityId, i64)> {
    let inside: Vec<EntityId> = ancestry(view, center, CONTAINED_IN);
    let mut found = match view.spatial() {
        Some(index) => {
            let root = root_of(view, center);
            let Some(at) = position_in(view, center, root) else {
                return Vec::new();
            };
            let mut out = Vec::new();
            let mut frames = Vec::new();
            search(
                view,
                index,
                center,
                root,
                &mut frames,
                at,
                at,
                radius,
                &mut out,
            );
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
    found.retain(|(e, _)| !inside.contains(e));
    found.sort_by_key(|&(e, d)| (d, e));
    found
}

/// Rounding slack, in centimetres per level of frame nesting, added to a query box when it is
/// carried down into a turned frame. Turning a point rounds each coordinate by at most half a
/// centimetre, once on the way down (the query centre) and once on the way up (the candidate),
/// so a few centimetres per level guarantees the box still holds every true answer. The exact
/// test afterwards is identical to the scanning road's, so the slack only ever admits
/// candidates — never answers.
const SLACK_PER_LEVEL: i64 = 4;

/// The indexed road of [`within`]: measure the candidates in `frame`, then descend into every
/// frame placed in it.
///
/// `frames` lists, innermost first, the `(position, heading)` of `frame` and each container
/// above it up to (not including) the root — exactly what [`lift`] needs to carry a point in
/// `frame` up to the root, by the same arithmetic [`position_in`] uses on the scanning road.
/// `local_at` is the query centre expressed in `frame`'s coordinates (for choosing candidates);
/// `at` is the same centre in the root's (for the exact test).
///
/// Every placed entity lives in exactly one frame, and every frame that holds anything is a
/// subframe of its own container, so descending from the root visits each candidate exactly
/// once. A frame is entered whenever its origin is within the radius plus its reach (the index's
/// bound on how far its contents can be from it) — a room's contents can be near even when the
/// room's origin is not, but a room across town cannot.
#[allow(clippy::too_many_arguments)]
fn search(
    view: &dyn CommittedView,
    index: &dyn SpatialQuery,
    center: EntityId,
    frame: EntityId,
    frames: &mut Vec<([i64; 3], i64)>,
    local_at: [i64; 3],
    at: [i64; 3],
    radius: i64,
    out: &mut Vec<(EntityId, i64)>,
) {
    let slack = SLACK_PER_LEVEL * (frames.len() as i64 + 1);
    let query = Aabb::around(local_at, radius.saturating_add(slack));
    for (e, placement) in index.candidates(frame, &query) {
        if e == center {
            continue;
        }
        // A settled body is at its anchor; a moving one is wherever its segment puts it now.
        let local = if placement.settled {
            placement.anchor
        } else {
            local_position(view, e)
        };
        let d = distance(at, lift(local, frames));
        if d <= radius {
            out.push((e, d));
        }
    }
    for sub in index.subframes(frame) {
        let Some(placement) = index.placement(sub) else {
            continue;
        };
        let origin = if placement.settled {
            placement.anchor
        } else {
            local_position(view, sub)
        };
        let shifted = [
            local_at[0] - origin[0],
            local_at[1] - origin[1],
            local_at[2] - origin[2],
        ];
        // Skip a whole room, house, or town when nothing in it can be in range: everything it
        // holds lies within its reach of its origin (a bound the index keeps, valid through any
        // turn — Amendment A-2), so if the question's centre is farther from that origin than
        // the radius plus the reach, the subtree has no answers. This is what keeps a question
        // asked in one home from visiting every home in the city (sweep D9).
        let gap = isqrt(shifted.iter().map(|c| (*c as i128).pow(2)).sum::<i128>() as u128);
        let limit = radius as u128 + index.reach(sub) as u128 + slack as u128 * 2;
        if gap > limit {
            continue;
        }
        let turn = heading(view, sub);
        // `frames` lists innermost first, so the subframe goes in front for the descent.
        frames.insert(0, (origin, turn));
        search(
            view,
            index,
            center,
            sub,
            frames,
            unrotate(turn, shifted),
            at,
            radius,
            out,
        );
        frames.remove(0);
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
