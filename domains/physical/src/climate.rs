//! Climate by nesting, and shelter (Vol. III Ch. 1 §1.10–1.11; Amendment A-5).
//!
//! **Climate is inherited down the containment hierarchy.** Weather is simulated only where a
//! world declares a climate — a region carrying its own temperature — and every place nested
//! inside it without one (a town in a province, a yard in a town, a cart in a yard) reads its
//! conditions from the nearest enclosing climate ([`ambient`]). A world of a thousand homes in
//! one valley therefore runs one valley's weather, not a thousand; a mountain town that should
//! be colder than its valley declares its own climate and gets its own weather. This is the
//! hierarchy doing the work: nothing about the inheritance needs coordinates at all.
//!
//! **Enclosed places are sheltered.** A region with walls ([`ENCLOSED`]) is out of the sky: the
//! sun does not shine into it and the sky's weather does not reach it, unless the world declares
//! how exposed it is ([`EXPOSURE`]). Instead, daylight comes in through its openings that let
//! light through, in proportion to their area against its floor ([`daylight_fraction`] — a room
//! with one small window is dim at noon), and its temperature follows the air outside it with a
//! lag ([`crate::systems::Shelter`]), slower for a room built of heavy material. A sheltered room
//! is a climate of its own — an indoor one — and what stands in it inherits *that*.

use crate::index::size_of;
use crate::schema::{
    CONTAINED_IN, ENCLOSED, EXPOSURE, LEADS_TO, OPAQUE, PERCENT_FULL, PORTAL_OPEN, TEMPERATURE,
};
use crate::space::portals_in;
use crate::terrain::is_true;
use kernel::fact::{FactKey, FactType};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;

/// The nearest of `entity` and its enclosing places that carries `fact`, with its value — the
/// climate an entity actually lives in. `None` if nothing up the hierarchy has one. Walks the
/// kernel's cycle-safe ancestry, so it terminates on any data.
pub fn ambient(
    view: &dyn CommittedView,
    entity: EntityId,
    fact: FactType,
) -> Option<(EntityId, i64)> {
    ancestry(view, entity, CONTAINED_IN)
        .into_iter()
        .find_map(|place| {
            view.read(FactKey::new(place, fact))
                .and_then(|f| f.value.as_int())
                .map(|v| (place, v))
        })
}

/// The temperature `entity` is in, in centidegrees: its own place's, or the nearest enclosing
/// climate's.
pub fn ambient_temperature(view: &dyn CommittedView, entity: EntityId) -> Option<i64> {
    ambient(view, entity, TEMPERATURE).map(|(_, t)| t)
}

/// The temperature of the air *outside* `region` — the nearest climate enclosing it, not counting
/// its own — which a sheltered room's air follows.
pub fn outside_temperature(view: &dyn CommittedView, region: EntityId) -> Option<i64> {
    ancestry(view, region, CONTAINED_IN)
        .into_iter()
        .skip(1)
        .find_map(|place| {
            view.read(FactKey::new(place, TEMPERATURE))
                .and_then(|f| f.value.as_int())
        })
}

/// Whether `region` is sheltered from the sky: walled, and not declared exposed.
pub fn is_sheltered(view: &dyn CommittedView, region: EntityId) -> bool {
    is_true(view, region, ENCLOSED) && view.read(FactKey::new(region, EXPOSURE)).is_none()
}

/// How open `region` is to the sky, in hundredths of a percent: its declared exposure if it has
/// one; none at all if it is walled; full otherwise (open ground under open sky).
pub fn exposure_of(view: &dyn CommittedView, region: EntityId) -> i64 {
    match view
        .read(FactKey::new(region, EXPOSURE))
        .and_then(|f| f.value.as_int())
    {
        Some(e) => e.clamp(0, PERCENT_FULL),
        None if is_true(view, region, ENCLOSED) => 0,
        None => PERCENT_FULL,
    }
}

/// Whether an opening lets light through: open, or shut but not opaque (a window).
fn passes_light(view: &dyn CommittedView, portal: EntityId) -> bool {
    let open = !matches!(
        view.read(FactKey::new(portal, PORTAL_OPEN))
            .map(|f| f.value),
        Some(Value::Bool(false))
    );
    open || !is_true(view, portal, OPAQUE)
}

/// The share of open-sky daylight that reaches a sheltered room through its openings, in
/// hundredths of a percent: the summed area of its light-passing openings, each weighted by how
/// exposed the place it opens onto is, against the room's floor area — the architect's daylight
/// factor, from geometry alone. A 10 m × 10 m room with a 90 cm × 2 m open door onto the yard
/// gets about 1.8% of daylight. A room or an opening without a declared size admits nothing it
/// cannot measure.
pub fn daylight_fraction(view: &dyn CommittedView, room: EntityId) -> i64 {
    let Some(room_size) = size_of(view, room) else {
        return 0;
    };
    let floor = (2 * room_size[0] as i128) * (2 * room_size[1] as i128);
    if floor <= 0 {
        return 0;
    }
    let mut lit: i128 = 0;
    for p in portals_in(view, room) {
        if !passes_light(view, p) {
            continue;
        }
        let Some(dest) = (match view.read(FactKey::new(p, LEADS_TO)).map(|f| f.value) {
            Some(Value::Entity(d)) => Some(d),
            _ => None,
        }) else {
            continue;
        };
        let Some(s) = size_of(view, p) else {
            continue;
        };
        let area = (2 * s[0] as i128) * s[2] as i128;
        lit += area * exposure_of(view, dest) as i128 / PERCENT_FULL as i128;
    }
    ((lit * PERCENT_FULL as i128) / floor).clamp(0, PERCENT_FULL as i128) as i64
}
