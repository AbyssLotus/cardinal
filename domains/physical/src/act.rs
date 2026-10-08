//! Operating and turning: the intents besides travel (Appendix A, Ruling 13 as amended by A-5).
//!
//! A decider proposes that a body open a door ([`ACT_OPEN`]), shut one ([`ACT_CLOSE`]), or face a
//! new way ([`ACT_FACE`]). Door state and facing are restricted facts — only Physical Reality's
//! own systems write them — so these intents are the only way anything else can change them, and
//! [`Act`] carries them out only when the world allows:
//!
//! - **A door within reach.** The body must stand in the same region as one face of the opening,
//!   no farther from it than its own half-width, the opening's half-width, and the world's
//!   `reach_cm` together. Both faces of the opening then open or shut together.
//! - **A body that can turn.** Only a mobile body turns on request; a wall does not.
//!
//! Every intent is cleared the tick it is considered. One that could not be carried out is
//! recorded in [`ACT_REFUSED`] (naming the door, or the body itself for a turn), so the decider
//! can notice and reconsider; the body's next successful act clears it. When two bodies would
//! set one door both ways in the same tick, the lower id acts and the other is refused — door
//! state never receives contradictory writes.

use crate::index::{container_of, size_of};
use crate::schema::{
    ACT_ARRIVE, ACT_CLOSE, ACT_CONSUME, ACT_DROP, ACT_FACE, ACT_IGNITE, ACT_LEAVE, ACT_MEND,
    ACT_OPEN, ACT_PICK, ACT_PUSH, ACT_REFUSED, ACT_STRIKE, ACT_TAKE, BODY_SIZE, BROKEN, BURNING,
    CONSUMED, CONTAINED_IN, ENCLOSED, FALL_HEIGHT, HAS_PORTAL, HEADING, IMPACT, INTEGRITY,
    LEADS_TO, MADE_OF, MATERIAL_DENSITY, MATERIAL_EDIBLE, MATERIAL_FLAMMABILITY, MATERIAL_HARDNESS,
    MOBILE, MOTION_END, MOTION_START, MOTION_TARGET, OPAQUE, PERCENT_FULL, PICKED, PICK_EFFORT,
    PORTAL_FAR_SIDE, PORTAL_OPEN, POSITION, SOLID, STRENGTH, TERRAIN_SAMPLE, TERRAIN_SPACING,
    WHOLE_SHAPE, YIELD_MADE_OF,
};
use crate::space::{far_side, local_position, portal_destination};
use crate::terrain::is_true;
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::isqrt;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;
use std::collections::BTreeMap;

/// How far in front of a fixture a thing arriving at it is set down, beyond both their edges, cm.
const ARRIVAL_GAP_CM: i64 = 5;

const ACT_READS: &[FactType] = &[
    crate::schema::MEND_MATERIAL,
    crate::schema::ACT_DIG,
    crate::schema::PIT,
    crate::schema::GROUND,
    crate::schema::PAVED,
    crate::schema::PLACE,
    crate::schema::WATER_KIND,
    crate::schema::FLOOD_DEPTH,
    crate::schema::ICE,
    crate::schema::RESOURCE_STOCK,
    YIELD_MADE_OF,
    crate::schema::WETNESS,
    ACT_PUSH,
    SOLID,
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    ACT_REFUSED,
    MOBILE,
    CONTAINED_IN,
    POSITION,
    BODY_SIZE,
    HEADING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    LEADS_TO,
    PORTAL_FAR_SIDE,
    PORTAL_OPEN,
    ACT_TAKE,
    ACT_DROP,
    ACT_CONSUME,
    ACT_ARRIVE,
    ACT_LEAVE,
    ACT_PICK,
    ENCLOSED,
    MADE_OF,
    MATERIAL_DENSITY,
    MATERIAL_EDIBLE,
    MATERIAL_HARDNESS,
    ACT_STRIKE,
    INTEGRITY,
    BROKEN,
    OPAQUE,
    HAS_PORTAL,
    STRENGTH,
    TERRAIN_SPACING,
    TERRAIN_SAMPLE,
    ACT_IGNITE,
    BURNING,
    MATERIAL_FLAMMABILITY,
    ACT_MEND,
    WHOLE_SHAPE,
];
const ACT_WRITES: &[FactType] = &[
    crate::schema::MEND_MATERIAL,
    crate::schema::ACT_DIG,
    crate::schema::PIT,
    MADE_OF,
    PICK_EFFORT,
    ACT_PUSH,
    ACT_MEND,
    IMPACT,
    WHOLE_SHAPE,
    ACT_IGNITE,
    BURNING,
    MOTION_TARGET,
    MOTION_START,
    MOTION_END,
    FALL_HEIGHT,
    ACT_STRIKE,
    INTEGRITY,
    BROKEN,
    SOLID,
    OPAQUE,
    ENCLOSED,
    BODY_SIZE,
    ACT_OPEN,
    ACT_CLOSE,
    ACT_FACE,
    ACT_REFUSED,
    PORTAL_OPEN,
    HEADING,
    ACT_TAKE,
    ACT_DROP,
    ACT_CONSUME,
    CONTAINED_IN,
    POSITION,
    CONSUMED,
    ACT_ARRIVE,
    ACT_LEAVE,
    ACT_PICK,
    PICKED,
];

/// Carries out open, shut, and face intents (Amendment A-5), and take, drop, and consume
/// (Amendment A-12; Appendix A, Ruling 16):
///
/// - **take** — a thing in the body's place, within its reach, not moving, not an opening or a
///   walled place, whose weight is known and no more than the world's carrying limit, and not
///   taken by anyone else this tick (the lower id first). It is then held: contained in the
///   body, at its centre;
/// - **drop** — a thing the body holds, put down where the body stands;
/// - **consume** — a thing the body holds, made entirely of edible materials. It leaves the
///   world (its place is cleared; its identity and composition remain), and the body's
///   [`CONSUMED`] report names it.
///
/// And striking and breaking (Amendment A-27):
///
/// - **strike** — a thing within reach, made of something with a known weight. The blow lands as
///   hard as the body's strength allows bare-handed, or as the best tool in hand allows — its
///   weight adding to the blow, its own weakest material bounding it. A blow no harder than the
///   thing's weakest material leaves no mark and is refused; a harder one costs it integrity, in
///   proportion to how much harder, over its weight;
/// - **break** — first, before any intent: whatever reached no integrity is broken. A thing
///   becomes its wreckage — flat, not solid, not opaque — and what it held spills where it stood;
///   a walled place becomes a ruin, its walls down and its doors broken open; a door breaks open,
///   both faces. Things spilling this tick cannot also be taken, dropped, or eaten in it.
pub struct Act {
    reach_cm: i64,
    carry_limit_g: i64,
    strike: StrikeRules,
    wreckage_height_cm: i64,
    moves: crate::travel::MoveRules,
    ignite_intensity: i64,
    mend_scale: i64,
    wall_thickness_cm: i64,
    pit_half_cm: i64,
}

/// What a thing's shape and flags were, as [`WHOLE_SHAPE`] keeps them: `[height, flags, 0]`.
fn whole_shape(view: &dyn CommittedView, thing: EntityId) -> Value {
    let flags = i64::from(is_true(view, thing, SOLID))
        | i64::from(is_true(view, thing, OPAQUE)) << 1
        | i64::from(is_true(view, thing, ENCLOSED)) << 2;
    let height = size_of(view, thing).map_or(0, |s| s[2]);
    Value::Vec3([height, flags, 0])
}

/// How a blow is judged (Amendment A-27), all world rules.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct StrikeRules {
    /// How hard a bare-handed blow lands, as a percentage of the striker's strength.
    pub hand_percent: i64,
    /// How much harder each kilogram of a tool in hand makes a blow, as a percentage of strength.
    pub tool_percent_per_kg: i64,
    /// Integrity lost per unit a blow lands harder than the thing's weakest material, per
    /// kilogram the thing weighs.
    pub scale: i64,
    /// The hardest material a bare hand works (Amendment A-41): fruit, soil. Anything harder is
    /// taken only with a tool no softer than it.
    pub bare_hand_hardness: i64,
    /// What a go of work costs the tool that does it, in hundredths of a percent of its
    /// integrity, when the stuff worked is as hard as the tool (Amendment A-46); less, in
    /// proportion, for softer stuff.
    pub tool_wear: i64,
}

/// A tool a go of work is done with, and how hard it is (Amendment A-46); `None`, a bare hand.
type Tool = Option<(EntityId, i64)>;

impl Act {
    /// Acts within `reach_cm` beyond a body's own extent, carries no more than `carry_limit_kg`,
    /// strikes by `strike`, and leaves wreckage no taller than `wreckage_height_cm` (world
    /// rules).
    pub const fn new(
        reach_cm: i64,
        carry_limit_kg: i64,
        strike: StrikeRules,
        wreckage_height_cm: i64,
        moves: crate::travel::MoveRules,
        ignite_intensity: i64,
        mend_scale: i64,
        wall_thickness_cm: i64,
    ) -> Self {
        Self {
            reach_cm,
            carry_limit_g: carry_limit_kg.saturating_mul(1000),
            strike,
            wreckage_height_cm,
            moves,
            ignite_intensity,
            mend_scale,
            wall_thickness_cm,
            pit_half_cm: 100,
        }
    }

    /// The same, digging pits `pit_half_cm` to either side of where the digger stands (Amendment
    /// A-42; a world rule).
    pub const fn with_pits(mut self, pit_half_cm: i64) -> Self {
        self.pit_half_cm = pit_half_cm;
        self
    }

    /// The place `body` stands in and what its ground is made of, if there is ground there to dig
    /// (Amendment A-42): not inside a thing or a walled place, not in water, on flooded or frozen
    /// ground, or on ground paved over.
    fn ground_under(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
    ) -> Option<(EntityId, EntityId)> {
        let place = container_of(view, body)?;
        let int = |fact| {
            view.read(FactKey::new(place, fact))
                .and_then(|f| f.value.as_int())
        };
        if !is_true(view, place, crate::schema::PLACE)
            || is_true(view, place, ENCLOSED)
            || is_true(view, place, crate::schema::PAVED)
            || view
                .read(FactKey::new(place, crate::schema::WATER_KIND))
                .is_some()
            || int(crate::schema::FLOOD_DEPTH).unwrap_or(0) > 0
            || int(crate::schema::ICE).unwrap_or(0) > 0
        {
            return None;
        }
        let ground = kernel::hierarchy::ancestry(view, place, CONTAINED_IN)
            .into_iter()
            .find_map(|p| entity_at(view, p, crate::schema::GROUND))?;
        Some((place, ground))
    }

    /// A pit in `place` within `body`'s reach that is not dug bare, the lowest first.
    fn pit_at_hand(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        place: EntityId,
    ) -> Option<EntityId> {
        crate::nearby::contents(view, place)
            .into_iter()
            .filter(|p| is_true(view, *p, crate::schema::PIT))
            .filter(|p| {
                view.read(FactKey::new(*p, crate::schema::RESOURCE_STOCK))
                    .and_then(|f| f.value.as_int())
                    .map_or(true, |stock| stock >= 100)
            })
            .filter(|p| self.within_reach(view, body, *p))
            .min()
    }

    /// The broken things whose integrity has come back whole: to be what they were (Amendment
    /// A-31). Looked for among what changed, as breaking is.
    fn mended_whole(view: &dyn CommittedView, basis: u64) -> Vec<EntityId> {
        let candidates = if basis > 0 {
            view.changed_since(INTEGRITY, basis - 1)
        } else {
            None
        }
        .unwrap_or_else(|| view.entities_with(BROKEN));
        candidates
            .into_iter()
            .filter(|e| {
                is_true(view, *e, BROKEN)
                    && view.read(FactKey::new(*e, INTEGRITY)).is_none()
                    && view.read(FactKey::new(*e, WHOLE_SHAPE)).is_some()
            })
            .collect()
    }

    /// What `body` can mend `target` with (Amendment A-31), if it can: it is damaged; within
    /// reach, or the place the body stands in; and the body has a bare hand for it, if it is no
    /// harder than a hand works (Amendment A-46), or holds a tool no softer than its weakest
    /// material — the hardest, which wears least, the lower id among equals.
    fn mend_tool(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        target: EntityId,
    ) -> Option<Tool> {
        let damaged = view.read(FactKey::new(target, INTEGRITY)).is_some();
        let near = container_of(view, body) == Some(target)
            || if view.read(FactKey::new(target, LEADS_TO)).is_some() {
                self.reachable_face(view, body, target).is_some()
            } else {
                self.within_reach(view, body, target)
            };
        if !damaged || !near {
            return None;
        }
        let resists = crate::materials::structural_hardness(view, target)?;
        if resists <= self.strike.bare_hand_hardness {
            return Some(None);
        }
        let mut best: Tool = None;
        for t in crate::nearby::contents(view, body) {
            let Some(h) = crate::materials::structural_hardness(view, t) else {
                continue;
            };
            if h >= resists
                && best.map_or(true, |(b, bh)| {
                    (h, std::cmp::Reverse(t)) > (bh, std::cmp::Reverse(b))
                })
            {
                best = Some((t, h));
            }
        }
        best.map(Some)
    }

    /// What a go of work on stuff `worked` hard costs `tool` (Amendment A-46): integrity, by the
    /// world's rule, in proportion to how hard the stuff is against the tool. Worn to nothing, the
    /// tool is used up and gone — what was left of it spent, not left as wreckage.
    fn wear(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        tool: Tool,
        worked: i64,
    ) -> Vec<(EntityId, FactType, Change, &'static str)> {
        let Some((tool, hard)) = tool else {
            return Vec::new();
        };
        let mut rng = ctx.rng(tool.raw() ^ (1 << 63));
        let worn = kernel::fixed::div_dither(
            self.strike.tool_wear as i128 * worked.max(0) as i128,
            hard.max(1) as i128,
            &mut rng,
        ) as i64;
        if worn <= 0 {
            return Vec::new();
        }
        let whole = view
            .read(FactKey::new(tool, INTEGRITY))
            .and_then(|f| f.value.as_int())
            .unwrap_or(PERCENT_FULL);
        if whole > worn {
            return vec![(tool, INTEGRITY, Change::Delta(-worn), "worn")];
        }
        let mut gone = vec![
            (tool, CONTAINED_IN, Change::Tombstone, "worn_out"),
            (tool, POSITION, Change::Tombstone, "worn_out"),
        ];
        if view.read(FactKey::new(tool, INTEGRITY)).is_some() {
            gone.push((tool, INTEGRITY, Change::Tombstone, "worn_out"));
        }
        gone
    }

    /// The pieces of `target`'s weakest material — what gave way, and what mends it — that `body`
    /// holds or has within reach, held first, then the lowest ids (Amendment A-44): loose things
    /// made of nothing else, not the thing itself, not a fixture, nothing used up already.
    fn mending_stuff(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        target: EntityId,
        handled: &std::collections::BTreeSet<EntityId>,
    ) -> Vec<EntityId> {
        let Some(stuff) = crate::materials::materials_of(view, target)
            .into_iter()
            .min_by_key(|m| (crate::materials::hardness(view, *m).unwrap_or(0), *m))
        else {
            return Vec::new();
        };
        let of_it = |t: &EntityId| {
            *t != target
                && *t != body
                && !handled.contains(t)
                && !is_true(view, *t, SOLID)
                && !is_true(view, *t, MOBILE)
                && view.read(FactKey::new(*t, LEADS_TO)).is_none()
                && crate::materials::materials_of(view, *t) == [stuff]
        };
        let mut held: Vec<EntityId> = crate::nearby::contents(view, body)
            .into_iter()
            .filter(|t| of_it(t))
            .collect();
        held.sort_unstable();
        let mut near: Vec<EntityId> = container_of(view, body)
            .map(|place| crate::nearby::contents(view, place))
            .unwrap_or_default()
            .into_iter()
            .filter(|t| of_it(t) && self.within_reach(view, body, *t))
            .collect();
        near.sort_unstable();
        held.extend(near);
        held
    }

    /// Whether `body` has a flame to hand (Amendment A-28): something burning that it holds, or
    /// that is within its reach.
    fn has_flame(&self, view: &dyn CommittedView, body: EntityId) -> bool {
        view.entities_with(BURNING).into_iter().any(|fire| {
            container_of(view, fire) == Some(body) || self.within_reach(view, body, fire)
        })
    }

    /// The things standing on `broken` that do not move by themselves — a lamp on a table — each
    /// with its fall, now that `broken` holds nothing up. (Bodies that move by themselves are
    /// gravity's, next tick.) Found among what shares its place, the same with or without the
    /// spatial index.
    fn left_standing_on(
        &self,
        view: &dyn CommittedView,
        broken: EntityId,
        place: EntityId,
    ) -> Vec<(EntityId, crate::travel::Fall)> {
        let under = crate::index::occupied_at(view, broken, local_position(view, broken));
        crate::nearby::contents(view, place)
            .into_iter()
            .filter(|t| {
                *t != broken
                    && !is_true(view, *t, MOBILE)
                    && view.read(FactKey::new(*t, LEADS_TO)).is_none()
                    && !is_true(view, *t, ENCLOSED)
                    && view.read(FactKey::new(*t, MOTION_TARGET)).is_none()
            })
            .filter(|t| {
                let b = crate::index::occupied_at(view, *t, local_position(view, *t));
                let on_top = (b.min[2] - under.max[2]).abs() <= 1;
                on_top
                    && b.min[0] < under.max[0]
                    && b.max[0] > under.min[0]
                    && b.min[1] < under.max[1]
                    && b.max[1] > under.min[1]
            })
            .filter_map(|t| {
                crate::travel::fall_of(view, t, &self.moves, Some(broken)).map(|f| (t, f))
            })
            .collect()
    }

    /// How hard `body` can hit, in the units of hardness: bare-handed, or with the best thing it
    /// holds, which never strikes harder than its own weakest material.
    fn blow_strength(&self, view: &dyn CommittedView, body: EntityId) -> i64 {
        let strength = view
            .read(FactKey::new(body, STRENGTH))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0)
            .max(0) as i128;
        let r = &self.strike;
        let hand = strength * r.hand_percent as i128 / 100;
        let tools = crate::nearby::contents(view, body)
            .into_iter()
            .filter_map(|tool| {
                let hard = crate::materials::structural_hardness(view, tool)? as i128;
                let grams = crate::materials::weight_g(view, tool)? as i128;
                let swung = strength
                    * (r.hand_percent as i128 * 1000 + r.tool_percent_per_kg as i128 * grams)
                    / 100_000;
                Some(swung.min(hard))
            });
        tools.chain(std::iter::once(hand)).max().unwrap_or(0) as i64
    }

    /// How hard `body` works `deposit` with one go (Amendment A-41): the hardest blow it can land
    /// with what is fit for what the deposit yields — a tool no softer than it, or a bare hand if
    /// it is soft enough — or `None` if it has nothing fit.
    fn working_effort(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        deposit: EntityId,
    ) -> Option<(i64, Tool, i64)> {
        let yields = match view
            .read(FactKey::new(deposit, YIELD_MADE_OF))
            .map(|f| f.value)
        {
            Some(Value::Entity(m)) => crate::materials::hardness(view, m).unwrap_or(0),
            _ => crate::materials::structural_hardness(view, deposit).unwrap_or(0),
        };
        self.effort_on(view, body, yields)
            .map(|(effort, tool)| (effort, tool, yields))
    }

    /// How hard `body` works a material `yields` hard with one go, and what with: the hardest
    /// blow it lands with a tool no softer than it — the lower id among equals — or a bare hand if
    /// it is soft enough and lands as hard; or `None`.
    fn effort_on(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        yields: i64,
    ) -> Option<(i64, Tool)> {
        let strength = view
            .read(FactKey::new(body, STRENGTH))
            .and_then(|f| f.value.as_int())
            .unwrap_or(0)
            .max(0) as i128;
        let r = &self.strike;
        let mut best: Option<(i128, Tool)> = (yields <= r.bare_hand_hardness)
            .then_some((strength * r.hand_percent as i128 / 100, None));
        let mut held = crate::nearby::contents(view, body);
        held.sort_unstable();
        for tool in held {
            let (Some(hard), Some(grams)) = (
                crate::materials::structural_hardness(view, tool),
                crate::materials::weight_g(view, tool),
            ) else {
                continue;
            };
            if hard < yields {
                continue;
            }
            let swung = (strength
                * (r.hand_percent as i128 * 1000 + r.tool_percent_per_kg as i128 * grams as i128)
                / 100_000)
                .min(hard as i128);
            if best.map_or(true, |(b, _)| swung > b) {
                best = Some((swung, Some((tool, hard))));
            }
        }
        best.map(|(e, tool)| (e as i64, tool))
    }

    /// The integrity a blow by `body` takes from `target`, or `None` if it cannot land or leaves
    /// no mark: out of reach, already broken, of unknown make or weight, or no harder than the
    /// target's weakest material.
    fn blow(&self, view: &dyn CommittedView, body: EntityId, target: EntityId) -> Option<i64> {
        let whole = view
            .read(FactKey::new(target, INTEGRITY))
            .and_then(|f| f.value.as_int())
            .unwrap_or(PERCENT_FULL);
        if target == body || whole <= 0 || is_true(view, target, BROKEN) {
            return None;
        }
        let reachable = if view.read(FactKey::new(target, LEADS_TO)).is_some() {
            self.reachable_face(view, body, target).is_some()
        } else {
            self.within_reach(view, body, target)
        };
        if !reachable {
            return None;
        }
        let resists = crate::materials::structural_hardness(view, target)?;
        let grams = crate::materials::weight_g(view, target)?.max(1);
        let lands = self.blow_strength(view, body);
        if lands <= resists {
            return None;
        }
        let damage = (lands - resists) as i128 * self.strike.scale as i128 * 1000 / grams as i128;
        Some(damage.clamp(1, PERCENT_FULL as i128) as i64)
    }

    /// The entities to break this tick: everything whose integrity has reached nothing and is not
    /// yet broken — looked for among what changed, when the store can say.
    fn breaking(view: &dyn CommittedView, basis: u64) -> Vec<EntityId> {
        let candidates = if basis > 0 {
            view.changed_since(INTEGRITY, basis - 1)
        } else {
            None
        }
        .unwrap_or_else(|| view.entities_with(INTEGRITY));
        candidates
            .into_iter()
            .filter(|e| {
                view.read(FactKey::new(*e, INTEGRITY))
                    .and_then(|f| f.value.as_int())
                    == Some(0)
                    && !is_true(view, *e, BROKEN)
            })
            .collect()
    }

    /// Both faces of every door into or out of walled place `place`.
    fn doors_of(view: &dyn CommittedView, place: EntityId) -> Vec<EntityId> {
        let hosted = |p: EntityId| -> Vec<EntityId> {
            view.read_all(FactKey::new(p, HAS_PORTAL))
                .into_iter()
                .filter_map(|f| match f.value {
                    Value::Entity(o) => Some(o),
                    _ => None,
                })
                .collect()
        };
        let mut faces: Vec<EntityId> = hosted(place);
        if let Some(around) = container_of(view, place) {
            faces.extend(
                hosted(around)
                    .into_iter()
                    .filter(|o| entity_at(view, *o, LEADS_TO) == Some(place)),
            );
        }
        let far: Vec<EntityId> = faces
            .iter()
            .filter_map(|f| portal_destination(view, *f).and_then(|d| far_side(view, *f, d)))
            .collect();
        faces.extend(far);
        faces.sort_unstable();
        faces.dedup();
        faces
    }

    /// Why `body` cannot take `thing` now, or `None` if it can.
    fn cannot_take(&self, view: &dyn CommittedView, body: EntityId, thing: EntityId) -> Option<()> {
        let unfit = thing == body
            || view.read(FactKey::new(thing, LEADS_TO)).is_some()
            || is_true(view, thing, ENCLOSED)
            || view.read(FactKey::new(thing, MOTION_TARGET)).is_some()
            || !self.within_reach(view, body, thing);
        let bearable =
            crate::materials::weight_g(view, thing).is_some_and(|g| g <= self.carry_limit_g);
        (unfit || !bearable).then_some(())
    }

    /// Whether `body` can reach door face `portal`: same region, and close enough.
    fn within_reach(&self, view: &dyn CommittedView, body: EntityId, portal: EntityId) -> bool {
        if container_of(view, body).is_none()
            || container_of(view, body) != container_of(view, portal)
        {
            return false;
        }
        let (a, b) = (local_position(view, body), local_position(view, portal));
        let (dx, dy) = ((a[0] - b[0]) as i128, (a[1] - b[1]) as i128);
        let gap = isqrt((dx * dx + dy * dy) as u128) as i64;
        let half = |e| size_of(view, e).map_or(0, |s| s[0].max(s[1]));
        gap <= half(body) + half(portal) + self.reach_cm.max(0)
    }

    /// The face of `door` the body can reach, if any: the door itself or its far side.
    fn reachable_face(
        &self,
        view: &dyn CommittedView,
        body: EntityId,
        door: EntityId,
    ) -> Option<EntityId> {
        if self.within_reach(view, body, door) {
            return Some(door);
        }
        let dest = portal_destination(view, door)?;
        far_side(view, door, dest).filter(|q| self.within_reach(view, body, *q))
    }
}

fn entity_at(view: &dyn CommittedView, e: EntityId, fact: FactType) -> Option<EntityId> {
    match view.read(FactKey::new(e, fact))?.value {
        Value::Entity(x) => Some(x),
        _ => None,
    }
}

impl System for Act {
    fn id(&self) -> SystemId {
        SystemId::new("physical.act")
    }
    fn reads(&self) -> &'static [FactType] {
        ACT_READS
    }
    fn writes(&self) -> &'static [FactType] {
        ACT_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        let mut push = |e: EntityId, fact: FactType, change: Change, cause: &'static str| {
            out.push(Proposal::new(
                self.id(),
                FactKey::new(e, fact),
                ctx.basis_tick(),
                change,
                Cause::new(cause),
            ));
        };
        // Every body with any intent, in ascending id: the lower id acts first on a shared door.
        let mut actors: Vec<EntityId> = Vec::new();
        for fact in [
            ACT_OPEN,
            ACT_CLOSE,
            ACT_FACE,
            ACT_TAKE,
            ACT_DROP,
            ACT_CONSUME,
            ACT_PICK,
            ACT_STRIKE,
            ACT_IGNITE,
            ACT_MEND,
            crate::schema::ACT_DIG,
        ] {
            actors.extend(view.entities_with(fact));
        }
        actors.sort_unstable();
        actors.dedup();
        // Door faces already set this tick, and to what; things already handled this tick.
        let mut doors: BTreeMap<EntityId, bool> = BTreeMap::new();
        let mut handled: std::collections::BTreeSet<EntityId> = Default::default();

        // What breaks (Amendment A-27), before anything is done with what it held.
        for broken in Self::breaking(view, ctx.basis_tick()) {
            push(broken, BROKEN, Change::Set(Value::Bool(true)), "broke");
            // What it was, to be again if mended whole (Amendment A-31).
            push(
                broken,
                WHOLE_SHAPE,
                Change::Set(whole_shape(view, broken)),
                "broke",
            );
            let faces = if is_true(view, broken, ENCLOSED) {
                // A walled place: its walls come down, and every door and shuttable window in
                // them is broken open. Whoever is inside is struck by what falls, from as high as
                // the walls stood (Amendment A-31).
                push(
                    broken,
                    ENCLOSED,
                    Change::Set(Value::Bool(false)),
                    "walls_fell",
                );
                let height = size_of(view, broken).map_or(0, |s| s[2]);
                for caught in crate::nearby::contents(view, broken) {
                    if is_true(view, caught, MOBILE) && height > 0 {
                        push(
                            caught,
                            IMPACT,
                            Change::Set(Value::Int(height)),
                            "struck_by_falling_walls",
                        );
                    }
                }
                Self::doors_of(view, broken)
                    .into_iter()
                    .filter(|f| {
                        is_true(view, *f, OPAQUE)
                            || view.read(FactKey::new(*f, PORTAL_OPEN)).map(|v| v.value)
                                == Some(Value::Bool(false))
                    })
                    .collect()
            } else if view.read(FactKey::new(broken, LEADS_TO)).is_some() {
                let mut faces = vec![broken];
                if let Some(dest) = portal_destination(view, broken) {
                    faces.extend(far_side(view, broken, dest));
                }
                faces
            } else {
                // A thing: its wreckage lies flat where it stood, and what it held spills out.
                if is_true(view, broken, SOLID) {
                    push(broken, SOLID, Change::Set(Value::Bool(false)), "broke");
                }
                if is_true(view, broken, OPAQUE) {
                    push(broken, OPAQUE, Change::Set(Value::Bool(false)), "broke");
                }
                if let Some([w, d, h]) = size_of(view, broken) {
                    if h > self.wreckage_height_cm {
                        let flat = [w, d, self.wreckage_height_cm.max(0)];
                        push(broken, BODY_SIZE, Change::Set(Value::Vec3(flat)), "broke");
                    }
                }
                if let Some(place) = container_of(view, broken) {
                    // What stood on it falls to what is under it now.
                    let now = view.tick() as i64;
                    for (thing, fall) in self.left_standing_on(view, broken, place) {
                        if !handled.insert(thing) {
                            continue;
                        }
                        let landing = Value::Vec3(fall.landing);
                        push(thing, POSITION, Change::Set(Value::Vec3(fall.from)), "fell");
                        push(thing, MOTION_TARGET, Change::Set(landing), "fell");
                        push(thing, MOTION_START, Change::Set(Value::Int(now)), "fell");
                        let end = Value::Int(now + fall.ticks as i64);
                        push(thing, MOTION_END, Change::Set(end), "fell");
                        push(
                            thing,
                            FALL_HEIGHT,
                            Change::Set(Value::Int(fall.drop)),
                            "fell",
                        );
                    }
                    let at = local_position(view, broken);
                    for spilled in crate::nearby::contents(view, broken) {
                        if !handled.insert(spilled) {
                            continue;
                        }
                        let off = local_position(view, spilled);
                        let to = [at[0] + off[0], at[1] + off[1], at[2] + off[2]];
                        push(
                            spilled,
                            CONTAINED_IN,
                            Change::Set(Value::Entity(place)),
                            "spilled",
                        );
                        push(spilled, POSITION, Change::Set(Value::Vec3(to)), "spilled");
                    }
                }
                Vec::new()
            };
            for face in faces {
                if face != broken {
                    push(face, BROKEN, Change::Set(Value::Bool(true)), "broke");
                    push(
                        face,
                        WHOLE_SHAPE,
                        Change::Set(whole_shape(view, face)),
                        "broke",
                    );
                }
                if doors.insert(face, true).is_none() {
                    push(
                        face,
                        PORTAL_OPEN,
                        Change::Set(Value::Bool(true)),
                        "broken_open",
                    );
                }
                if is_true(view, face, OPAQUE) {
                    push(face, OPAQUE, Change::Set(Value::Bool(false)), "broke");
                }
            }
        }
        // What was broken and is mended whole is what it was again (Amendment A-31): standing as
        // tall, as solid, as opaque, and walled, as it was — and a walled place's doors with it.
        for whole in Self::mended_whole(view, ctx.basis_tick()) {
            let mut restore = |thing: EntityId| {
                let Some([height, flags, _]) = view
                    .read(FactKey::new(thing, WHOLE_SHAPE))
                    .and_then(|f| f.value.as_vec3())
                else {
                    return;
                };
                push(thing, BROKEN, Change::Tombstone, "mended");
                push(thing, WHOLE_SHAPE, Change::Tombstone, "mended");
                for (bit, fact) in [(1, SOLID), (2, OPAQUE), (4, ENCLOSED)] {
                    if flags & bit != 0 {
                        push(thing, fact, Change::Set(Value::Bool(true)), "mended");
                    }
                }
                if let Some([w, d, h]) = size_of(view, thing) {
                    if h != height && height > 0 {
                        let back = Value::Vec3([w, d, height]);
                        push(thing, BODY_SIZE, Change::Set(back), "mended");
                    }
                }
            };
            restore(whole);
            let walled = view
                .read(FactKey::new(whole, WHOLE_SHAPE))
                .and_then(|f| f.value.as_vec3())
                .is_some_and(|[_, flags, _]| flags & 4 != 0);
            if walled {
                for face in Self::doors_of(view, whole) {
                    if is_true(view, face, BROKEN)
                        && view.read(FactKey::new(face, INTEGRITY)).is_none()
                    {
                        restore(face);
                    }
                }
            }
        }

        // Things coming into the world, and leaving it (Amendment A-15): into a place, or a hand;
        // or, asked to arrive at a fixture — a hearth — set down just in front of it, in its
        // place, where one can walk up to it (Amendment A-18).
        for thing in view.entities_with(ACT_ARRIVE) {
            let Some(at) = entity_at(view, thing, ACT_ARRIVE) else {
                continue;
            };
            push(thing, ACT_ARRIVE, Change::Tombstone, "considered");
            let (into, position) = match container_of(view, at) {
                Some(place) if is_true(view, at, SOLID) => {
                    let [x, y, z] = local_position(view, at);
                    let depth = size_of(view, at).map_or(0, |s| s[1]);
                    let own = size_of(view, thing).map_or(0, |s| s[1]);
                    (place, [x, y - depth - own - ARRIVAL_GAP_CM, z])
                }
                _ => (at, [0, 0, 0]),
            };
            if handled.insert(thing) {
                push(
                    thing,
                    CONTAINED_IN,
                    Change::Set(Value::Entity(into)),
                    "arrived",
                );
                push(
                    thing,
                    POSITION,
                    Change::Set(Value::Vec3(position)),
                    "arrived",
                );
            }
        }
        for thing in view.entities_with(ACT_LEAVE) {
            push(thing, ACT_LEAVE, Change::Tombstone, "considered");
            if handled.insert(thing) && container_of(view, thing).is_some() {
                push(thing, CONTAINED_IN, Change::Tombstone, "used_up");
                push(thing, POSITION, Change::Tombstone, "used_up");
            }
        }
        for body in actors {
            let mut refused: Option<EntityId> = None;
            let mut succeeded = false;
            for (fact, open) in [(ACT_OPEN, true), (ACT_CLOSE, false)] {
                let Some(door) = entity_at(view, body, fact) else {
                    continue;
                };
                push(body, fact, Change::Tombstone, "considered");
                // A broken door stands open: there is nothing left to shut (Amendment A-27).
                if is_true(view, door, BROKEN) {
                    if open {
                        succeeded = true;
                    } else {
                        refused = Some(door);
                    }
                    continue;
                }
                let faces = match self.reachable_face(view, body, door) {
                    Some(_) => {
                        let mut faces = vec![door];
                        if let Some(dest) = portal_destination(view, door) {
                            faces.extend(far_side(view, door, dest));
                        }
                        faces
                    }
                    None => {
                        refused = Some(door);
                        continue;
                    }
                };
                if faces
                    .iter()
                    .any(|f| doors.get(f).is_some_and(|v| *v != open))
                {
                    refused = Some(door); // someone else set it the other way this tick
                    continue;
                }
                for f in faces {
                    if doors.insert(f, open).is_none() {
                        push(
                            f,
                            PORTAL_OPEN,
                            Change::Set(Value::Bool(open)),
                            if open { "opened" } else { "shut" },
                        );
                    }
                }
                succeeded = true;
            }
            if let Some(Value::Int(bearing)) =
                view.read(FactKey::new(body, ACT_FACE)).map(|f| f.value)
            {
                push(body, ACT_FACE, Change::Tombstone, "considered");
                if is_true(view, body, MOBILE) {
                    push(
                        body,
                        HEADING,
                        Change::Set(Value::Int(bearing.rem_euclid(36_000))),
                        "turned",
                    );
                    succeeded = true;
                } else {
                    refused = Some(body);
                }
            }
            // Handling things (Amendment A-12).
            if let Some(thing) = entity_at(view, body, ACT_TAKE) {
                push(body, ACT_TAKE, Change::Tombstone, "considered");
                if self.cannot_take(view, body, thing).is_some() || !handled.insert(thing) {
                    refused = Some(thing);
                } else {
                    push(
                        thing,
                        CONTAINED_IN,
                        Change::Set(Value::Entity(body)),
                        "taken",
                    );
                    push(
                        thing,
                        POSITION,
                        Change::Set(Value::Vec3([0, 0, 0])),
                        "taken",
                    );
                    succeeded = true;
                }
            }
            if let Some(thing) = entity_at(view, body, ACT_DROP) {
                push(body, ACT_DROP, Change::Tombstone, "considered");
                let held = container_of(view, thing) == Some(body);
                match container_of(view, body).filter(|_| held && handled.insert(thing)) {
                    Some(place) => {
                        let at = local_position(view, body);
                        push(
                            thing,
                            CONTAINED_IN,
                            Change::Set(Value::Entity(place)),
                            "dropped",
                        );
                        push(thing, POSITION, Change::Set(Value::Vec3(at)), "dropped");
                        succeeded = true;
                    }
                    None => refused = Some(thing),
                }
            }
            if let Some(thing) = entity_at(view, body, ACT_CONSUME) {
                push(body, ACT_CONSUME, Change::Tombstone, "considered");
                let held = container_of(view, thing) == Some(body);
                if held && crate::materials::is_edible(view, thing) && handled.insert(thing) {
                    push(thing, CONTAINED_IN, Change::Tombstone, "consumed");
                    push(thing, POSITION, Change::Tombstone, "consumed");
                    push(
                        body,
                        CONSUMED,
                        Change::Set(Value::Entity(thing)),
                        "consumed",
                    );
                    succeeded = true;
                } else {
                    refused = Some(thing);
                }
            }
            // Mending (Amendments A-31, A-44): with a tool no softer than what is mended, a little
            // at a time — less, the more there is of it — and with the stuff it is made of, used
            // up in proportion to what the go restores: from what is laid by for it, then from
            // what the mender holds or has within reach. With less than a go needs, it restores
            // what that much allows; with none, it is refused.
            if let Some(target) = entity_at(view, body, ACT_MEND) {
                push(body, ACT_MEND, Change::Tombstone, "considered");
                let mut mended = false;
                if let Some(tool) = self.mend_tool(view, body, target) {
                    let kg = crate::fire::structure_kg(view, target, self.wall_thickness_cm).max(1);
                    let mut rng = ctx.rng(target.raw());
                    let full = kernel::fixed::div_dither(
                        self.mend_scale as i128 * 1000,
                        kg as i128,
                        &mut rng,
                    ) as i64;
                    let whole = view
                        .read(FactKey::new(target, INTEGRITY))
                        .and_then(|f| f.value.as_int())
                        .unwrap_or(PERCENT_FULL);
                    let wanted = full.min(PERCENT_FULL - whole).max(0);
                    // Grams of its stuff a go of `r` takes.
                    let grams_for = |r: i64| r as i128 * kg as i128 * 1000 / PERCENT_FULL as i128;
                    let laid = view
                        .read(FactKey::new(target, crate::schema::MEND_MATERIAL))
                        .and_then(|f| f.value.as_int())
                        .unwrap_or(0)
                        .max(0) as i128;
                    let mut stuff = laid;
                    if stuff < grams_for(wanted) {
                        for piece in self.mending_stuff(view, body, target, &handled) {
                            if stuff >= grams_for(wanted) {
                                break;
                            }
                            stuff += crate::materials::weight_g(view, piece).unwrap_or(0) as i128;
                            handled.insert(piece);
                            push(piece, CONTAINED_IN, Change::Tombstone, "used_in_mending");
                            push(piece, POSITION, Change::Tombstone, "used_in_mending");
                        }
                    }
                    let restored = if stuff >= grams_for(wanted) {
                        wanted
                    } else {
                        (stuff * PERCENT_FULL as i128 / (kg as i128 * 1000)) as i64
                    };
                    if restored > 0 {
                        push(target, INTEGRITY, Change::Delta(restored), "mended");
                        // The go wears the tool it was done with (Amendment A-46).
                        let resists =
                            crate::materials::structural_hardness(view, target).unwrap_or(0);
                        for (e, fact, change, why) in self.wear(view, ctx, tool, resists) {
                            push(e, fact, change, why);
                        }
                        let left = (stuff - grams_for(restored)).max(0) as i64;
                        let change = if left > 0 {
                            Change::Set(Value::Int(left))
                        } else {
                            Change::Tombstone
                        };
                        if left as i128 != laid {
                            push(target, crate::schema::MEND_MATERIAL, change, "laid_by");
                        }
                        mended = true;
                    }
                }
                if mended {
                    succeeded = true;
                } else {
                    refused = Some(target);
                }
            }
            // Setting fire (Amendment A-28): with a flame to hand, to a thing in hand or in reach
            // that can burn.
            if let Some(target) = entity_at(view, body, ACT_IGNITE) {
                push(body, ACT_IGNITE, Change::Tombstone, "considered");
                let in_hand = container_of(view, target) == Some(body);
                let near = in_hand || self.within_reach(view, body, target);
                // A flame lights only what will burn at least as fiercely as a fire begins: not
                // soaked timber (Amendment A-38).
                let takes = crate::fire::burns_as(view, target) >= self.ignite_intensity;
                if near
                    && takes
                    && crate::fire::can_catch(view, target)
                    && self.has_flame(view, body)
                {
                    let fire = Value::Int(self.ignite_intensity);
                    push(target, BURNING, Change::Set(fire), "set_alight");
                    succeeded = true;
                } else {
                    refused = Some(target);
                }
            }
            // Striking (Amendment A-27): every blow landed on one thing this tick adds up.
            if let Some(target) = entity_at(view, body, ACT_STRIKE) {
                push(body, ACT_STRIKE, Change::Tombstone, "considered");
                match self.blow(view, body, target) {
                    Some(damage) => {
                        push(target, INTEGRITY, Change::Delta(-damage), "struck");
                        succeeded = true;
                    }
                    None => refused = Some(target),
                }
            }
            // Digging (Amendment A-42): the ground where the body stands, if there is ground to
            // dig — at a pit in reach that is not dug bare, or, if none, breaking a new one.
            if is_true(view, body, crate::schema::ACT_DIG) {
                push(
                    body,
                    crate::schema::ACT_DIG,
                    Change::Tombstone,
                    "considered",
                );
                match self.ground_under(view, body) {
                    None => refused = Some(container_of(view, body).unwrap_or(body)),
                    Some((place, ground)) => match self.pit_at_hand(view, body, place) {
                        Some(pit) => match self.working_effort(view, body, pit) {
                            Some((effort, tool, worked)) => {
                                push(body, PICKED, Change::Set(Value::Entity(pit)), "dug");
                                push(body, PICK_EFFORT, Change::Set(Value::Int(effort)), "dug");
                                for (e, fact, change, why) in self.wear(view, ctx, tool, worked) {
                                    push(e, fact, change, why);
                                }
                                succeeded = true;
                            }
                            None => refused = Some(pit),
                        },
                        None => {
                            let hard = crate::materials::hardness(view, ground).unwrap_or(0);
                            match self.effort_on(view, body, hard) {
                                Some((effort, tool)) => {
                                    for (e, fact, change, why) in self.wear(view, ctx, tool, hard) {
                                        push(e, fact, change, why);
                                    }
                                    let pit = ctx.new_entity();
                                    let [x, y, z] = local_position(view, body);
                                    let half = self.pit_half_cm;
                                    for (fact, change) in [
                                        (CONTAINED_IN, Change::Set(Value::Entity(place))),
                                        (POSITION, Change::Set(Value::Vec3([x, y, z]))),
                                        (BODY_SIZE, Change::Set(Value::Vec3([half, half, 0]))),
                                        (MADE_OF, Change::Add(Value::Entity(ground))),
                                        (crate::schema::PIT, Change::Set(Value::Bool(true))),
                                    ] {
                                        push(pit, fact, change, "broke_ground");
                                    }
                                    push(body, PICKED, Change::Set(Value::Entity(pit)), "dug");
                                    push(body, PICK_EFFORT, Change::Set(Value::Int(effort)), "dug");
                                    succeeded = true;
                                }
                                None => refused = Some(place),
                            }
                        }
                    },
                }
            }
            // Taking from a deposit (Amendments A-15, A-41): within reach, and with a hand or a
            // tool hard enough for what it yields. The go's effort is reported with it.
            if let Some(deposit) = entity_at(view, body, ACT_PICK) {
                push(body, ACT_PICK, Change::Tombstone, "considered");
                match self.working_effort(view, body, deposit) {
                    Some((effort, tool, worked)) if self.within_reach(view, body, deposit) => {
                        push(body, PICKED, Change::Set(Value::Entity(deposit)), "picked");
                        push(body, PICK_EFFORT, Change::Set(Value::Int(effort)), "picked");
                        for (e, fact, change, why) in self.wear(view, ctx, tool, worked) {
                            push(e, fact, change, why);
                        }
                        succeeded = true;
                    }
                    _ => refused = Some(deposit),
                }
            }
            match refused {
                Some(what) => push(
                    body,
                    ACT_REFUSED,
                    Change::Set(Value::Entity(what)),
                    "refused",
                ),
                None if succeeded && view.read(FactKey::new(body, ACT_REFUSED)).is_some() => {
                    push(body, ACT_REFUSED, Change::Tombstone, "done")
                }
                None => {}
            }
        }
        // Pushes (Amendment A-33), last: a thing someone has taken hold of, or that broke or fell
        // this tick, is not also pushed. One moving already finishes its move first. In a place
        // with a size, a push stops at its edge.
        for thing in view.entities_with(ACT_PUSH) {
            push(thing, ACT_PUSH, Change::Tombstone, "considered");
            let Some([dx, dy, _]) = view
                .read(FactKey::new(thing, ACT_PUSH))
                .and_then(|f| f.value.as_vec3())
            else {
                continue;
            };
            let Some(place) = container_of(view, thing) else {
                continue;
            };
            if view.read(FactKey::new(thing, MOTION_TARGET)).is_some() || !handled.insert(thing) {
                continue;
            }
            let [x, y, z] = local_position(view, thing);
            let (mut nx, mut ny) = (x.saturating_add(dx), y.saturating_add(dy));
            if let Some([hw, hd, _]) = size_of(view, place) {
                (nx, ny) = (nx.clamp(-hw, hw), ny.clamp(-hd, hd));
            }
            if [nx, ny] != [x, y] {
                push(
                    thing,
                    POSITION,
                    Change::Set(Value::Vec3([nx, ny, z])),
                    "pushed",
                );
            }
        }
        out
    }
}
