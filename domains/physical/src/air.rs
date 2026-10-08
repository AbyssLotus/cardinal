//! Air between places (Vol. III Ch. 1 §1.10, "smoke diffuses"; Amendment A-37).
//!
//! Air passes through every opening that stands open, both ways at once: the warmer, smokier air
//! on one side trades with the other's. How much passes is the opening's area times the speed the
//! air moves through it — the world's natural draught, and, where the opening gives onto open
//! air, a share of the wind outside. What passes changes each side in proportion to how much air
//! it holds:
//!
//! - a **walled place** holds its floor area times its height;
//! - **open ground with a size** — a yard, a ruin — holds its area times the world's mixing
//!   height: the air over it that smoke fills before it is carried off;
//! - **open air** — a climate, or a place with no size — is too much to change: what reaches it
//!   is lost in it, and it gives back only its own air.
//!
//! Warmth ([`crate::systems::Shelter`]) and smoke ([`crate::fire::Fire`]) both pass this way.

use crate::index::size_of;
use crate::schema::{BODY_SIZE, ENCLOSED, PLACE, PORTAL_OPEN, WIND_SPEED};
use crate::space::{far_side, portal_destination, portals_in};
use crate::terrain::is_true;
use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use std::collections::BTreeSet;

/// How air moves through openings, all world rules (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct AirRules {
    /// How fast air moves through an open opening of itself, each way, in cm/s: the draught
    /// warm and cold air make trading places.
    pub opening_speed_cm_s: i64,
    /// What share of the wind outside, in hundredths of a percent, blows through an opening that
    /// gives onto open air.
    pub draught: i64,
    /// How high over open ground smoke mixes before it is carried off, in centimetres.
    pub open_air_height_cm: i64,
}

/// One opening air passes through: between `a` and `b`, at `flow` cubic centimetres a second.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Way {
    /// The place on one side.
    pub a: EntityId,
    /// The place on the other.
    pub b: EntityId,
    /// How much air passes each way, in cubic centimetres a second.
    pub flow: i128,
}

/// How much air `place` holds, in cubic centimetres: a walled place, all of it; open ground with
/// a size, its area to the world's mixing height; anything else is open air — `None`.
pub fn volume_cm3(view: &dyn CommittedView, place: EntityId, rules: &AirRules) -> Option<i128> {
    let [hw, hd, h] = size_of(view, place)?;
    let area = 4 * hw as i128 * hd as i128;
    if area <= 0 {
        return None;
    }
    if is_true(view, place, ENCLOSED) {
        Some((area * h as i128).max(1))
    } else if is_true(view, place, PLACE) {
        Some((area * rules.open_air_height_cm.max(1) as i128).max(1))
    } else {
        None
    }
}

/// Whether `place` is open air: it holds no measured air of its own.
pub fn open_air(view: &dyn CommittedView, place: EntityId, rules: &AirRules) -> bool {
    volume_cm3(view, place, rules).is_none()
}

/// Whether air passes through `portal`: it stands open (a broken door stands open).
fn open(view: &dyn CommittedView, portal: EntityId) -> bool {
    view.read(FactKey::new(portal, PORTAL_OPEN))
        .map(|f| f.value)
        != Some(Value::Bool(false))
}

/// The air passing through `portal` from its host `from`, in cubic centimetres a second: its
/// area, times the world's draught, and a share of the wind if it gives onto open air.
fn flow(view: &dyn CommittedView, portal: EntityId, to: EntityId, rules: &AirRules) -> i128 {
    let Some([hw, _, h]) = view
        .read(FactKey::new(portal, BODY_SIZE))
        .and_then(|f| f.value.as_vec3())
    else {
        return 0;
    };
    let area = 2 * hw.max(0) as i128 * h.max(0) as i128;
    let wind = if is_true(view, to, ENCLOSED) {
        0
    } else {
        crate::climate::ambient(view, to, WIND_SPEED).map_or(0, |(_, w)| w.max(0))
    };
    let speed = rules.opening_speed_cm_s.max(0) as i128
        + wind as i128 * rules.draught.max(0) as i128 / crate::schema::PERCENT_FULL as i128;
    area * speed
}

/// The ways air passes from each of `places`: every open opening any of them hosts, once each —
/// an opening's two faces are one way — with the place on its far side. Sorted, so the answer is
/// deterministic.
pub fn ways(view: &dyn CommittedView, places: &BTreeSet<EntityId>, rules: &AirRules) -> Vec<Way> {
    let mut seen: BTreeSet<EntityId> = BTreeSet::new();
    let mut out = Vec::new();
    for &place in places {
        for portal in portals_in(view, place) {
            let Some(to) = portal_destination(view, portal) else {
                continue;
            };
            let face = far_side(view, portal, to);
            let id = face.map_or(portal, |f| f.min(portal));
            if !seen.insert(id) || !open(view, portal) {
                continue;
            }
            let flow = flow(view, portal, to, rules);
            if flow > 0 {
                out.push(Way {
                    a: place,
                    b: to,
                    flow,
                });
            }
        }
    }
    out
}

/// The open openings `place` hosts, each with the place it gives onto and the air passing
/// through it, in cubic centimetres a second — from this side, so a door between two rooms is
/// counted by each from its own face.
pub fn openings(
    view: &dyn CommittedView,
    place: EntityId,
    rules: &AirRules,
) -> Vec<(EntityId, i128)> {
    portals_in(view, place)
        .into_iter()
        .filter(|p| open(view, *p))
        .filter_map(|p| {
            let to = portal_destination(view, p)?;
            let flow = flow(view, p, to, rules);
            (flow > 0).then_some((to, flow))
        })
        .collect()
}
