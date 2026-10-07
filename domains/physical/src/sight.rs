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

use crate::climate::ambient;
use crate::nearby::{contents, within};
use crate::schema::{
    BODY_SIZE, CONTAINED_IN, ENCLOSED, HAS_PORTAL, HEADING, ILLUMINATION, IN_VIEW, LEADS_TO,
    MOTION_END, MOTION_START, MOTION_TARGET, OPAQUE, PORTAL_OPEN, POSITION, SIGHT_RANGE,
    TERRAIN_SAMPLE, TERRAIN_SPACING,
};
use crate::shape::body_box;
use crate::space::{lower, portals_in, position_in};
use crate::terrain::{is_true, spacing, terrain_height};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::{div_round, isqrt};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::spatial::Aabb;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;
use std::collections::BTreeSet;

/// Rounding slack, in centimetres, allowed when a line carried into a nested, turned frame is
/// tested against an opening there — the same few centimetres per level the proximity search
/// allows, so a line through the middle of a doorway is never refused for a rounding.
const OPENING_SLACK: i64 = 4;

/// For one evaluation of sight: each frame's places that have no body of their own and might hold
/// something in the way, which frames hold nothing that could block a line at all, and how lit
/// each place is — worked out once and shared by every line tested, not kept from one tick to the
/// next, so sight stays a function of committed reality alone.
pub struct Bare {
    bare: std::cell::RefCell<std::collections::BTreeMap<EntityId, BareFrames>>,
    clear: std::cell::RefCell<std::collections::BTreeMap<EntityId, bool>>,
    light: std::cell::RefCell<std::collections::BTreeMap<EntityId, Option<i64>>>,
    chains: std::cell::RefCell<std::collections::BTreeMap<EntityId, std::rc::Rc<[EntityId]>>>,
    positions:
        std::cell::RefCell<std::collections::BTreeMap<(EntityId, EntityId), Option<[i64; 3]>>>,
    walk: std::cell::Cell<usize>,
}

/// A frame's places with no body of their own, and the farthest any of their contents reaches
/// from where each stands — so those a line could meet are found by one search of the index.
#[derive(Clone, Default)]
struct BareFrames {
    places: std::collections::BTreeSet<EntityId>,
    reach: i64,
}

/// The most things sight looks over to judge one frame clear, counting everything within it. A
/// frame holding more — a city's open ground — is looked into near each line instead, which gives
/// the same answer; frames within it finished on the way stay known.
const CLEAR_WALK: usize = 16_384;

impl Default for Bare {
    fn default() -> Self {
        Self {
            bare: Default::default(),
            clear: Default::default(),
            light: Default::default(),
            chains: Default::default(),
            positions: Default::default(),
            walk: std::cell::Cell::new(0),
        }
    }
}

impl Bare {
    /// `f`'s places with no body of their own whose contents could reach `span`, and which hold
    /// something that could block a line there.
    fn near(&self, view: &dyn CommittedView, f: EntityId, span: &Aabb) -> Vec<EntityId> {
        let Some(index) = view.spatial() else {
            return Vec::new();
        };
        let known = self.bare.borrow().get(&f).cloned();
        let frames = known.unwrap_or_else(|| {
            let places: std::collections::BTreeSet<EntityId> = index
                .subframes(f)
                .into_iter()
                .filter(|s| crate::index::size_of(view, *s).is_none())
                .collect();
            let reach = places
                .iter()
                .map(|s| index.reach(*s).min(i64::MAX as u64 / 4) as i64)
                .max()
                .unwrap_or(0);
            let frames = BareFrames { places, reach };
            self.bare.borrow_mut().insert(f, frames.clone());
            frames
        });
        if frames.places.is_empty() {
            return Vec::new();
        }
        // Every bare place whose contents could reach the span stands within the farthest reach
        // of it; then each is held to its own reach, and to whether it holds anything at all
        // that could block.
        index
            .candidates(f, &span.expand(frames.reach))
            .into_iter()
            .map(|(s, _)| s)
            .filter(|s| frames.places.contains(s))
            .filter(|s| {
                let reach = index.reach(*s).min(i64::MAX as u64 / 4) as i64;
                index
                    .placement(*s)
                    .map_or(true, |p| span.expand(reach).intersects(&p.bounds))
            })
            .filter(|s| !self.clear(view, *s))
            .collect()
    }

    /// `entity` and everything holding it, innermost first — [`ancestry`], each worked out once.
    fn chain(&self, view: &dyn CommittedView, entity: EntityId) -> std::rc::Rc<[EntityId]> {
        if let Some(known) = self.chains.borrow().get(&entity) {
            return std::rc::Rc::clone(known);
        }
        let chain: std::rc::Rc<[EntityId]> = ancestry(view, entity, CONTAINED_IN).into();
        self.chains
            .borrow_mut()
            .insert(entity, std::rc::Rc::clone(&chain));
        chain
    }

    /// The deepest place holding both `a` and `b` (each holds itself) — the kernel's
    /// `lowest_common_ancestor`, from the chains above.
    fn common(&self, view: &dyn CommittedView, a: EntityId, b: EntityId) -> Option<EntityId> {
        let of_b = self.chain(view, b);
        self.chain(view, a)
            .iter()
            .copied()
            .find(|e| of_b.contains(e))
    }

    /// Where `entity` stands in `frame`'s coordinates — [`position_in`], each worked out once.
    fn position_in(
        &self,
        view: &dyn CommittedView,
        entity: EntityId,
        frame: EntityId,
    ) -> Option<[i64; 3]> {
        if let Some(known) = self.positions.borrow().get(&(entity, frame)) {
            return *known;
        }
        let at = position_in(view, entity, frame);
        self.positions.borrow_mut().insert((entity, frame), at);
        at
    }

    /// Whether `entity` is lit to at least `min` — [`lit`], with each place's light looked up
    /// once: a thing that carries no light of its own is in its place's.
    fn lit(&self, view: &dyn CommittedView, entity: EntityId, min: i64) -> bool {
        let own = view
            .read(FactKey::new(entity, ILLUMINATION))
            .and_then(|f| f.value.as_int());
        let light = match (own, crate::index::container_of(view, entity)) {
            (Some(l), _) => Some(l),
            (None, None) => None,
            (None, Some(place)) => {
                let known = self.light.borrow().get(&place).copied();
                known.unwrap_or_else(|| {
                    let l = crate::climate::ambient(view, place, ILLUMINATION).map(|(_, l)| l);
                    self.light.borrow_mut().insert(place, l);
                    l
                })
            }
        };
        light.map_or(true, |l| l >= min)
    }

    /// Whether nothing in `f`, however deep, could block a line: no ground of its own or below
    /// it, and nothing walled or opaque. A line through such a frame is never blocked in it —
    /// the answer [`blocked_in`] would reach the long way. Openings never block. `false` when
    /// there is more in `f` than [`CLEAR_WALK`] things, or no index to look by.
    fn clear(&self, view: &dyn CommittedView, f: EntityId) -> bool {
        if let Some(known) = self.clear.borrow().get(&f) {
            return *known;
        }
        self.walk.set(CLEAR_WALK);
        let clear = self.clear_within(view, f).unwrap_or(false);
        self.clear.borrow_mut().insert(f, clear);
        clear
    }

    /// [`Bare::clear`] for `f`, within what is left of the walk: `None` if it ran out first.
    /// Every frame within `f` finished on the way is remembered.
    fn clear_within(&self, view: &dyn CommittedView, f: EntityId) -> Option<bool> {
        let Some(index) = view.spatial() else {
            return Some(false);
        };
        if spacing(view, f).is_some() {
            return Some(false);
        }
        for c in index.children(f) {
            let left = self.walk.get();
            if left == 0 {
                return None;
            }
            self.walk.set(left - 1);
            if view.read(FactKey::new(c, LEADS_TO)).is_some() {
                continue;
            }
            if is_true(view, c, ENCLOSED) || is_true(view, c, OPAQUE) {
                return Some(false);
            }
            if index.reach(c) == 0 {
                continue; // holds nothing
            }
            let known = self.clear.borrow().get(&c).copied();
            let inner = match known {
                Some(k) => k,
                None => {
                    let k = self.clear_within(view, c)?;
                    self.clear.borrow_mut().insert(c, k);
                    k
                }
            };
            if !inner {
                return Some(false);
            }
        }
        Some(true)
    }
}

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
    line_of_sight_with(view, observer, target, &Bare::default())
}

/// [`line_of_sight`], sharing `bare` with the other lines of the same evaluation.
fn line_of_sight_with(
    view: &dyn CommittedView,
    observer: EntityId,
    target: EntityId,
    bare: &Bare,
) -> bool {
    if observer == target {
        return true;
    }
    // Side by side in one place — the usual case — the place is the common frame, each end is
    // where it stands in it, and there is no chain of containers to climb.
    let here = crate::index::container_of(view, observer);
    if here.is_some() && here == crate::index::container_of(view, target) {
        let common = here.expect("checked");
        let o = crate::space::local_position(view, observer);
        let t = crate::space::local_position(view, target);
        return clear_between(view, common, (observer, o, &[]), (target, t, &[]), bare);
    }
    let Some(common) = bare.common(view, observer, target) else {
        return false;
    };
    let (Some(o), Some(t)) = (
        bare.position_in(view, observer, common),
        bare.position_in(view, target, common),
    ) else {
        return false;
    };
    // The containers each end is inside, below the common one. When one end holds the other — a
    // thing in the hand, the cart one rides in — the common one is that end itself, and there is
    // nothing in between.
    let inside = |e: EntityId| -> Vec<EntityId> {
        let chain = bare.chain(view, e);
        let stop = chain
            .iter()
            .position(|c| *c == common)
            .unwrap_or(chain.len());
        chain
            .get(1..stop)
            .map_or_else(Vec::new, <[EntityId]>::to_vec)
    };
    let (side_o, side_t) = (inside(observer), inside(target));
    clear_between(
        view,
        common,
        (observer, o, &side_o),
        (target, t, &side_t),
        bare,
    )
}

/// One end of a line of sight: the body, where it stands in the common frame, and the
/// containers it is inside below the common one.
type End<'a> = (EntityId, [i64; 3], &'a [EntityId]);

/// Whether `observer` can see `target`, both placed in `common`'s frame: a line from the
/// observer's eye to the target's top or middle is clear.
fn clear_between(
    view: &dyn CommittedView,
    common: EntityId,
    (observer, o, side_o): End,
    (target, t, side_t): End,
    bare: &Bare,
) -> bool {
    let eye = [o[0], o[1], o[2] + height(view, observer)];
    // A body is seen if its top or its middle is in view — a head over a sill, a chest behind
    // a low wall. (For a point, both are the point.)
    let tall = height(view, target);
    let marks = [[t[0], t[1], t[2] + tall], [t[0], t[1], t[2] + tall / 2]];
    let mut ignore = vec![observer, target];
    ignore.extend(side_o);
    ignore.extend(side_t);
    marks
        .iter()
        .any(|&mark| line_is_clear(view, common, side_o, side_t, &ignore, eye, mark, bare))
}

/// Whether the single line `eye → mark` (in `common`'s frame) is clear: through an opening of
/// every enclosed container on either side, and past every opaque body and rising ground.
#[allow(clippy::too_many_arguments)]
fn line_is_clear(
    view: &dyn CommittedView,
    common: EntityId,
    side_o: &[EntityId],
    side_t: &[EntityId],
    ignore: &[EntityId],
    eye: [i64; 3],
    mark: [i64; 3],
    bare: &Bare,
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
        if bare.clear(view, f) {
            continue;
        }
        let (Some(a), Some(b)) = (lower(view, eye, f, common), lower(view, mark, f, common)) else {
            continue;
        };
        if blocked_in(view, f, a, b, ignore, bare) {
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
    bare: &Bare,
) -> bool {
    if bare.clear(view, f) {
        return false;
    }
    if ground_rises(view, f, a, b) {
        return true;
    }
    let span = Aabb::new(a, b).expand(OPENING_SLACK);
    // Candidates: what the line's span meets, and every place within `f` with no body of its
    // own whose contents could reach the span. A place with a body that the span misses is passed
    // over: a line that misses its box can neither be blocked by it nor by what it holds. What a
    // place holds lies within the index's reach of wherever the place is, so a place farther than
    // that from the span holds nothing the line can meet.
    let candidates: Vec<EntityId> = match view.spatial() {
        Some(index) => index
            .candidates(f, &span)
            .into_iter()
            .map(|(e, _)| e)
            .chain(bare.near(view, f, &span))
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
        // What can block at all: an opaque body, a walled place, or an open region holding
        // something that might. Anything else is passed over without working out its shape.
        let enclosed = is_true(view, c, ENCLOSED);
        let opaque = !enclosed && is_true(view, c, OPAQUE);
        let holds_things = !enclosed && !contents(view, c).is_empty();
        if !enclosed && !opaque && !holds_things {
            continue;
        }
        let body = body_box(view, c);
        let hits = body.is_some_and(|bx| bx.segment_hits(a, b, 0));
        if hits && (enclosed || opaque) {
            return true;
        }
        // An open region the line passes through: what stands inside it can block too.
        if holds_things && (body.is_none() || hits) {
            let local = |p| crate::space::lower(view, p, c, f);
            if let (Some(la), Some(lb)) = (local(a), local(b)) {
                if blocked_in(view, c, la, lb, ignore, bare) {
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

/// Whether `entity` stands in light enough to be seen: the illumination of its own place, or of
/// the nearest enclosing place that has one, is at least `min`. A world that simulates no light
/// at all is always lit.
pub fn lit(view: &dyn CommittedView, entity: EntityId, min: i64) -> bool {
    ambient(view, entity, ILLUMINATION).map_or(true, |(_, light)| light >= min)
}

/// What `observer` could see within `range`: in sight, and lit — in ascending id order — and the
/// place it stands in, when that is lit (Amendment A-11): seeing the room is how a mind knows its
/// view is good enough to tell that someone is not there. And whatever it holds, which it knows
/// by touch (Amendment A-13).
pub fn in_view(
    view: &dyn CommittedView,
    observer: EntityId,
    range: i64,
    min_illumination: i64,
) -> BTreeSet<EntityId> {
    in_view_with(view, observer, range, min_illumination, &Bare::default())
}

/// [`in_view`], sharing `bare` with every other observer of the same evaluation.
fn in_view_with(
    view: &dyn CommittedView,
    observer: EntityId,
    range: i64,
    min_illumination: i64,
    bare: &Bare,
) -> BTreeSet<EntityId> {
    if range <= 0 {
        return BTreeSet::new();
    }
    let mut seen: BTreeSet<EntityId> = within(view, observer, range)
        .into_iter()
        .map(|(e, _)| e)
        .filter(|e| {
            bare.lit(view, *e, min_illumination) && line_of_sight_with(view, observer, *e, bare)
        })
        .collect();
    if let Some(here) = crate::index::container_of(view, observer) {
        if bare.lit(view, here, min_illumination) {
            seen.insert(here);
        }
    }
    // What one holds, one knows by touch, light or dark (Amendment A-13).
    seen.extend(contents(view, observer));
    seen
}

// Sight reads where things are (the placement facts the index mirrors), what blocks a line
// (walls, openings, opaque bodies, the ground), the light, the observer's range, and what it
// saw last step — to write only the difference.
const SIGHT_READS: &[FactType] = &[
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    ENCLOSED,
    LEADS_TO,
    HAS_PORTAL,
    OPAQUE,
    PORTAL_OPEN,
    TERRAIN_SAMPLE,
    TERRAIN_SPACING,
    ILLUMINATION,
    SIGHT_RANGE,
    IN_VIEW,
];
const SIGHT_WRITES: &[FactType] = &[IN_VIEW];

/// Publishes what each body with sight could see (Amendment A-8; Vol. III Ch. 1, *What is in
/// view*): everything within its sight range — Living Systems' fact — along a clear line and
/// lit above the world's threshold. One proposal per thing that came into or went out of view,
/// so a still scene writes nothing. A body whose sight range is gone sees nothing.
pub struct Sight {
    step: Step,
    min_illumination: i64,
}

impl Sight {
    /// Refresh what is in view as `step` says; things in light below `min_illumination` (hundredths
    /// of a percent) cannot be seen.
    pub const fn new(step: Step, min_illumination: i64) -> Self {
        Self {
            step,
            min_illumination,
        }
    }
}

impl System for Sight {
    fn id(&self) -> SystemId {
        SystemId::new("physical.sight")
    }
    fn reads(&self) -> &'static [FactType] {
        SIGHT_READS
    }
    fn writes(&self) -> &'static [FactType] {
        SIGHT_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut observers: BTreeSet<EntityId> =
            view.entities_with(SIGHT_RANGE).into_iter().collect();
        observers.extend(view.entities_with(IN_VIEW));
        let mut out = Vec::new();
        let bare = Bare::default();
        for observer in observers {
            let range = view
                .read(FactKey::new(observer, SIGHT_RANGE))
                .and_then(|f| f.value.as_int())
                .unwrap_or(0);
            let now = in_view_with(view, observer, range, self.min_illumination, &bare);
            let before: BTreeSet<EntityId> = view
                .read_all(FactKey::new(observer, IN_VIEW))
                .into_iter()
                .filter_map(|f| match f.value {
                    Value::Entity(e) => Some(e),
                    _ => None,
                })
                .collect();
            let key = FactKey::new(observer, IN_VIEW);
            for seen in now.difference(&before) {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Add(Value::Entity(*seen)),
                    Cause::new("came_into_view"),
                ));
            }
            for gone in before.difference(&now) {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    Change::Remove(Value::Entity(*gone)),
                    Cause::new("went_out_of_view"),
                ));
            }
        }
        out
    }
}
