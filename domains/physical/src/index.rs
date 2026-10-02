//! Physical Reality's placement rule for the store's spatial index (Vol. V Ch. 2 §2.1,
//! Amendment A-2; Vol. III Ch. 1 §1.12).
//!
//! Space is Physical Reality's (Appendix A), so this domain alone tells the store where things
//! are. An entity is placed in the frame of its immediate container ([`CONTAINED_IN`]), at its
//! base point, occupying the box its size and heading describe ([`occupied_at`]). The index then
//! keeps one grid per container, so a moving or turning container never forces its contents to
//! be re-filed — their local positions do not change.
//!
//! A body in motion (Amendment A-3) is filed under the box its whole segment sweeps, and marked
//! unsettled: the index then holds a box guaranteed to contain it at every tick of its journey,
//! without being rewritten while it travels, and a query that needs its exact point asks
//! [`crate::motion`] for it.
//!
//! [`occupied`] is also the definition the scanning fallback uses, which is what makes the
//! conformance rule easy to keep: indexed and scanned answers are built from one function.

use crate::motion::segment;
use crate::schema::{
    BODY_SIZE, CONTAINED_IN, HEADING, MOTION_END, MOTION_START, MOTION_TARGET, POSITION,
};
use crate::space::{heading, local_position, rotate};
use kernel::fact::{FactKey, FactType};
use kernel::identity::EntityId;
use kernel::spatial::{Aabb, Placement, SpatialProjector};
use kernel::system::CommittedView;
use kernel::value::Value;

/// Every fact the placement rule reads. A system that asks a spatial question through the index
/// must declare all of these in its read set (Amendment A-2, no opinions).
pub const PLACEMENT_READS: &[FactType] = &[
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
];

/// The container `entity` is placed in, if any.
pub fn container_of(view: &dyn CommittedView, entity: EntityId) -> Option<EntityId> {
    match view.read(FactKey::new(entity, CONTAINED_IN))?.value {
        Value::Entity(container) => Some(container),
        _ => None,
    }
}

/// A body's declared size `[half_width, half_depth, height]`, or `None` for a point.
pub fn size_of(view: &dyn CommittedView, entity: EntityId) -> Option<[i64; 3]> {
    view.read(FactKey::new(entity, BODY_SIZE))?
        .value
        .as_vec3()
        .map(|s| s.map(|c| c.max(0)))
}

/// The box a body of `size` (or a point, if `None`) fills when its base stands at `base`,
/// turned to `heading`, in its container's frame: its footprint's four corners, turned and
/// enclosed, and from its base up through its height. A turned rectangle's box is larger than
/// the rectangle — boxes are for finding candidates; exact containment tests use the body's own
/// frame.
pub fn box_at(base: [i64; 3], size: Option<[i64; 3]>, heading: i64) -> Aabb {
    let Some([hw, hd, height]) = size else {
        return Aabb::point(base);
    };
    let mut bounds = Aabb::point([base[0], base[1], base[2]]);
    for (cx, cy) in [(-hw, -hd), (-hw, hd), (hw, -hd), (hw, hd)] {
        let c = rotate(heading, [cx, cy, 0]);
        bounds = bounds.union(&Aabb::point([base[0] + c[0], base[1] + c[1], base[2]]));
    }
    bounds.max[2] = base[2].saturating_add(height);
    bounds
}

/// Where `entity` is and what it fills at the view's tick, in its container's local
/// coordinates: the container, the entity's base point, and its box there. `None` if it is in no
/// container.
pub fn occupied(view: &dyn CommittedView, entity: EntityId) -> Option<(EntityId, [i64; 3], Aabb)> {
    let frame = container_of(view, entity)?;
    let at = local_position(view, entity);
    Some((frame, at, occupied_at(view, entity, at)))
}

/// The box `entity` fills when its base is at `base` (its own size and heading).
pub fn occupied_at(view: &dyn CommittedView, entity: EntityId, base: [i64; 3]) -> Aabb {
    box_at(base, size_of(view, entity), heading(view, entity))
}

/// The placement rule Physical Reality registers with the store.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicalProjector;

impl SpatialProjector for PhysicalProjector {
    fn watches(&self) -> &'static [FactType] {
        PLACEMENT_READS
    }

    fn place(&self, view: &dyn CommittedView, entity: EntityId) -> Option<Placement> {
        let frame = container_of(view, entity)?;
        // Placement must not depend on the tick (the store re-places only when facts change),
        // so a body with a segment is filed by the whole segment: the box at its start and at
        // its end, enclosed — every point of a straight leg lies between them.
        match segment(view, entity) {
            Some(leg) => Some(Placement {
                frame,
                anchor: leg.from,
                bounds: occupied_at(view, entity, leg.from)
                    .union(&occupied_at(view, entity, leg.to)),
                settled: false,
            }),
            None => {
                let at = crate::motion::anchor(view, entity);
                Some(Placement {
                    frame,
                    anchor: at,
                    bounds: occupied_at(view, entity, at),
                    settled: true,
                })
            }
        }
    }
}
