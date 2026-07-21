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

use crate::schema::{CONTAINED_IN, POSITION_X, POSITION_Y, POSITION_Z};
use kernel::fact::{FactKey, FactType};
use kernel::hierarchy::lowest_common_ancestor;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;

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
