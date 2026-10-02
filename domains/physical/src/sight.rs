//! Line of sight (Vol. III Ch. 1 §1.6 "What can be seen?", §1.11 "Walls block vision";
//! Amendment A-4).
//!
//! Sight is a geometric question Physical Reality answers and others build on: the information
//! pipeline decides what an observer *notices* (Vol. II Ch. 4 — lighting, attention, distance),
//! but whether anything stands between two bodies is a fact about space. A line runs from the
//! observer's **eye** (the top of its body; its base, for a point) to the target's **top**, and
//! another to its **middle**, expressed in the frame of their lowest common container; the target
//! is in sight if either line is clear. A line is blocked by any of:
//!
//! - **a wall** — an enclosed container on either side of the line that the line leaves or
//!   enters other than through one of its openings that passes sight (an open portal, or a
//!   closed one that is not opaque). An opening without a size is a point, and a line cannot be
//!   threaded through a point: to be seen through, an opening must declare its size;
//! - **an opaque body** with a size, anywhere along the line in any frame it passes through —
//!   and an enclosed region the line crosses that neither end is inside (a building in between);
//! - **rising ground** — terrain higher than the line beneath any point of it.
//!
//! Like proximity, sight looks for candidates through the spatial index when there is one and by
//! scanning otherwise; the tests that decide are exact and shared, so both roads agree.

use crate::nearby::{contents, within};
use crate::schema::{CONTAINED_IN, ENCLOSED, LEADS_TO, OPAQUE, PORTAL_OPEN};
use crate::shape::body_box;
use crate::space::{lower, portals_in, position_in};
use crate::terrain::{is_true, spacing, terrain_height};
use kernel::fact::FactKey;
use kernel::fixed::{div_round, isqrt};
use kernel::hierarchy::{ancestry, lowest_common_ancestor};
use kernel::identity::EntityId;
use kernel::spatial::Aabb;
use kernel::system::CommittedView;
use kernel::value::Value;

/// Rounding slack, in centimetres, allowed when a line carried into a nested, turned frame is
/// tested against an opening there — the same few centimetres per level the proximity search
/// allows, so a line through the middle of a doorway is never refused for a rounding.
const OPENING_SLACK: i64 = 4;

/// How tall `entity` is (0 for a point).
fn height(view: &dyn CommittedView, entity: EntityId) -> i64 {
    crate::index::size_of(view, entity).map_or(0, |s| s[2])
}

/// Whether an opening passes sight: open, or closed but not opaque (a shut window).
fn passes_sight(view: &dyn CommittedView, portal: EntityId) -> bool {
    let open = !matches!(
        view.read(FactKey::new(portal, PORTAL_OPEN))
            .map(|f| f.value),
        Some(Value::Bool(false))
    );
    open || !is_true(view, portal, OPAQUE)
}

/// Whether `observer` can see `target`: nothing opaque, no wall, and no rising ground lies on
/// the straight line from the observer's eye to the target's middle. `false` if they share no
/// containment hierarchy. A body can always see itself.
pub fn line_of_sight(view: &dyn CommittedView, observer: EntityId, target: EntityId) -> bool {
    if observer == target {
        return true;
    }
    let Some(common) = lowest_common_ancestor(view, observer, target, CONTAINED_IN) else {
        return false;
    };
    let (Some(o), Some(t)) = (
        position_in(view, observer, common),
        position_in(view, target, common),
    ) else {
        return false;
    };
    let eye = [o[0], o[1], o[2] + height(view, observer)];
    // A body is seen if its top or its middle is in view — a head over a sill, a chest behind
    // a low wall. (For a point, both are the point.)
    let tall = height(view, target);
    let marks = [[t[0], t[1], t[2] + tall], [t[0], t[1], t[2] + tall / 2]];

    // The containers each end is inside, below the common one.
    let inside = |e: EntityId| -> Vec<EntityId> {
        let chain = ancestry(view, e, CONTAINED_IN);
        let stop = chain
            .iter()
            .position(|c| *c == common)
            .unwrap_or(chain.len());
        chain[1..stop].to_vec()
    };
    let (side_o, side_t) = (inside(observer), inside(target));
    let mut ignore = vec![observer, target];
    ignore.extend(&side_o);
    ignore.extend(&side_t);
    marks
        .iter()
        .any(|&mark| line_is_clear(view, common, &side_o, &side_t, &ignore, eye, mark))
}

/// Whether the single line `eye → mark` (in `common`'s frame) is clear: through an opening of
/// every enclosed container on either side, and past every opaque body and rising ground.
fn line_is_clear(
    view: &dyn CommittedView,
    common: EntityId,
    side_o: &[EntityId],
    side_t: &[EntityId],
    ignore: &[EntityId],
    eye: [i64; 3],
    mark: [i64; 3],
) -> bool {
    // 1. Walls: every enclosed container on either side must be crossed through an opening.
    for &c in side_o.iter().chain(side_t.iter()) {
        if is_true(view, c, ENCLOSED) && !through_an_opening(view, c, common, eye, mark) {
            return false;
        }
    }
    // 2. Opaque bodies and rising ground, in the common frame and in every container on either
    //    side. The two ends and the containers they are inside are not obstacles to themselves.
    let frames = std::iter::once(common)
        .chain(side_o.iter().copied())
        .chain(side_t.iter().copied());
    for f in frames {
        let (Some(a), Some(b)) = (lower(view, eye, f, common), lower(view, mark, f, common)) else {
            continue;
        };
        if blocked_in(view, f, a, b, ignore) {
            return false;
        }
    }
    true
}

/// Whether the line `eye → mark` (in `common`'s frame) leaves or enters enclosed container `c`
/// through one of its openings that passes sight.
fn through_an_opening(
    view: &dyn CommittedView,
    c: EntityId,
    common: EntityId,
    eye: [i64; 3],
    mark: [i64; 3],
) -> bool {
    let (Some(a), Some(b)) = (lower(view, eye, c, common), lower(view, mark, c, common)) else {
        return false;
    };
    portals_in(view, c).into_iter().any(|p| {
        passes_sight(view, p)
            && body_box(view, p).is_some_and(|opening| {
                let mut tall = opening;
                // Openings are thin; let the line graze them by the rounding slack.
                tall.size = [tall.size[0], tall.size[1] + OPENING_SLACK, tall.size[2]];
                tall.segment_hits(a, b, OPENING_SLACK)
            })
    })
}

/// Whether anything placed in frame `f` blocks the segment `a → b` (in `f`'s frame): an opaque
/// body, an enclosed region the line enters, the contents of an open region it passes through,
/// or `f`'s own rising ground.
fn blocked_in(
    view: &dyn CommittedView,
    f: EntityId,
    a: [i64; 3],
    b: [i64; 3],
    ignore: &[EntityId],
) -> bool {
    if ground_rises(view, f, a, b) {
        return true;
    }
    let span = Aabb::new(a, b).expand(OPENING_SLACK);
    let candidates: Vec<EntityId> = match view.spatial() {
        Some(index) => index
            .candidates(f, &span)
            .into_iter()
            .map(|(e, _)| e)
            .chain(index.subframes(f))
            .collect(),
        None => contents(view, f),
    };
    let mut seen = std::collections::BTreeSet::new();
    for c in candidates {
        if !seen.insert(c) || ignore.contains(&c) {
            continue;
        }
        // Openings are judged by the walls they pierce, never as obstacles themselves.
        if view.read(FactKey::new(c, LEADS_TO)).is_some() {
            continue;
        }
        let body = body_box(view, c);
        let hits = body.is_some_and(|bx| bx.segment_hits(a, b, 0));
        if hits && (is_true(view, c, OPAQUE) || is_true(view, c, ENCLOSED)) {
            return true;
        }
        // An open region the line may pass through: what stands inside it can block too.
        let holds_things = !contents(view, c).is_empty();
        if holds_things && !is_true(view, c, ENCLOSED) && (body.is_none() || hits) {
            let local = |p| crate::space::lower(view, p, c, f);
            if let (Some(la), Some(lb)) = (local(a), local(b)) {
                if blocked_in(view, c, la, lb, ignore) {
                    return true;
                }
            }
        }
    }
    false
}

/// Whether `f`'s terrain rises above the segment `a → b` anywhere between its ends, sampled
/// every half grid spacing along the line.
fn ground_rises(view: &dyn CommittedView, f: EntityId, a: [i64; 3], b: [i64; 3]) -> bool {
    let Some(sp) = spacing(view, f) else {
        return false;
    };
    let run = isqrt(((b[0] - a[0]) as i128).pow(2) as u128 + ((b[1] - a[1]) as i128).pow(2) as u128)
        as i64;
    let steps = (run / (sp / 2).max(1)).clamp(1, 4096) as i128;
    (1..steps).any(|k| {
        let at = |i: usize| (a[i] as i128 + div_round((b[i] - a[i]) as i128 * k, steps)) as i64;
        terrain_height(view, f, at(0), at(1)).is_some_and(|g| at(2) < g)
    })
}

/// Everything within `radius` of `observer` that it can see, nearest first — "what do you see?"
pub fn visible(view: &dyn CommittedView, observer: EntityId, radius: i64) -> Vec<(EntityId, i64)> {
    within(view, observer, radius)
        .into_iter()
        .filter(|(e, _)| line_of_sight(view, observer, *e))
        .collect()
}
