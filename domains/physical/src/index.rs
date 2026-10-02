//! Physical Reality's placement rule for the store's spatial index (Vol. V Ch. 2 §2.1,
//! Amendment A-2; Vol. III Ch. 1 §1.12).
//!
//! Space is Physical Reality's (Appendix A), so this domain alone tells the store where things
//! are. The rule is deliberately small: an entity is placed in the frame of its immediate
//! container ([`CONTAINED_IN`]), at its local position, occupying the box [`occupied`]
//! describes. The index then keeps one grid per container, so a moving container never forces
//! its contents to be re-filed.
//!
//! [`occupied`] is also the definition the scanning fallback uses, which is what makes the
//! conformance rule easy to keep: indexed and scanned answers are built from one function.

use crate::schema::{CONTAINED_IN, POSITION_X, POSITION_Y, POSITION_Z};
use crate::space::local_position;
use kernel::fact::{FactKey, FactType};
use kernel::identity::EntityId;
use kernel::spatial::{Aabb, Placement, SpatialProjector};
use kernel::system::CommittedView;
use kernel::value::Value;

/// Every fact the placement rule reads. A system that asks a spatial question through the index
/// must declare all of these in its read set (Amendment A-2, no opinions).
pub const PLACEMENT_READS: &[FactType] = &[CONTAINED_IN, POSITION_X, POSITION_Y, POSITION_Z];

/// The container `entity` is placed in, if any.
pub fn container_of(view: &dyn CommittedView, entity: EntityId) -> Option<EntityId> {
    match view.read(FactKey::new(entity, CONTAINED_IN))?.value {
        Value::Entity(container) => Some(container),
        _ => None,
    }
}

/// Where `entity` is and what it occupies, in its container's local coordinates: the
/// container, the entity's reference point (its local position), and the box it fills there.
/// `None` if it is in no container. An entity with no size occupies just its point.
pub fn occupied(view: &dyn CommittedView, entity: EntityId) -> Option<(EntityId, [i64; 3], Aabb)> {
    let frame = container_of(view, entity)?;
    let at = local_position(view, entity);
    Some((frame, at, Aabb::point(at)))
}

/// The placement rule Physical Reality registers with the store.
#[derive(Clone, Copy, Debug, Default)]
pub struct PhysicalProjector;

impl SpatialProjector for PhysicalProjector {
    fn watches(&self) -> &'static [FactType] {
        PLACEMENT_READS
    }

    fn place(&self, view: &dyn CommittedView, entity: EntityId) -> Option<Placement> {
        let (frame, anchor, bounds) = occupied(view, entity)?;
        Some(Placement {
            frame,
            anchor,
            bounds,
            settled: true,
        })
    }
}
