//! Perception (Vol. II Ch. 4 *Observation*; Vol. I Ch. 4 §10.1; Amendment A-8): reality, as
//! Physical Reality and Living Systems publish it, turned into what one mind observes, believes,
//! and remembers.

use crate::schema::{
    AFFECTION, BLOCKED, ECON_OWNER, FELT_BODY_HEAT, FELT_FATIGUE, FELT_HEALTH, FELT_HUNGER,
    FELT_NEED, FELT_NEED_MET_BY, GOING_TO, IN_SIGHT, LEADS_TO, LIVING_BODY_HEAT, LIVING_FATIGUE,
    LIVING_HEALTH, LIVING_HUNGER, LIVING_KIND_MET_BY, LIVING_NEED, LIVING_NEED_KIND,
    LIVING_SIGHT_RANGE, MADE_OF, MIND_TEMPERAMENT, NUTRITION, OPEN, OWNER, PHYS_ACT_REFUSED,
    PHYS_CONTAINED_IN, PHYS_IN_VIEW, PHYS_LEADS_TO, PHYS_MADE_OF, PHYS_MOBILE, PHYS_NUTRITION,
    PHYS_PICKED, PHYS_PORTAL_OPEN, PHYS_SOLID, PHYS_TEMPERATURE, PHYS_TRAVEL_BLOCKED,
    PHYS_TRAVEL_TO, PLACE_OF, REFUSED, RES_STOCK, RES_YIELD, SOC_JOB_CARRIES, SOC_JOB_FROM,
    SOC_JOB_HOURS, SOC_JOB_KEEP, SOC_JOB_MAKES, SOC_JOB_TO, SOC_ROLE, STOCK, WARMTH_OF,
    WORK_CARRIES, WORK_FROM, WORK_HOURS, WORK_KEEP, WORK_MAKES, WORK_ROLE, WORK_TO, YIELDS,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;
use std::collections::BTreeSet;

// Perception reads what is in view and the facts it reports about what is seen, the air and
// the body it feels, the body's own intent and reports — and its own beliefs, to write only
// what changed.
const PERCEPTION_READS: &[FactType] = &[
    ECON_OWNER,
    OWNER,
    PHYS_SOLID,
    PHYS_PICKED,
    SOC_ROLE,
    SOC_JOB_CARRIES,
    SOC_JOB_MAKES,
    SOC_JOB_FROM,
    SOC_JOB_TO,
    SOC_JOB_KEEP,
    SOC_JOB_HOURS,
    WORK_ROLE,
    WORK_CARRIES,
    WORK_MAKES,
    WORK_FROM,
    WORK_TO,
    WORK_KEEP,
    WORK_HOURS,
    FELT_HUNGER,
    LIVING_HUNGER,
    NUTRITION,
    PHYS_NUTRITION,
    RES_YIELD,
    YIELDS,
    RES_STOCK,
    STOCK,
    PHYS_MADE_OF,
    MADE_OF,
    LIVING_NEED,
    LIVING_NEED_KIND,
    LIVING_KIND_MET_BY,
    PHYS_MOBILE,
    FELT_NEED,
    FELT_NEED_MET_BY,
    LIVING_SIGHT_RANGE,
    PHYS_IN_VIEW,
    PHYS_CONTAINED_IN,
    PHYS_LEADS_TO,
    PHYS_PORTAL_OPEN,
    PHYS_TEMPERATURE,
    PHYS_TRAVEL_TO,
    PHYS_TRAVEL_BLOCKED,
    PHYS_ACT_REFUSED,
    LIVING_BODY_HEAT,
    LIVING_FATIGUE,
    LIVING_HEALTH,
    FELT_FATIGUE,
    FELT_HEALTH,
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
    GOING_TO,
    BLOCKED,
    REFUSED,
];
const PERCEPTION_WRITES: &[FactType] = &[
    OWNER,
    WORK_ROLE,
    WORK_CARRIES,
    WORK_MAKES,
    WORK_FROM,
    WORK_TO,
    WORK_KEEP,
    WORK_HOURS,
    FELT_HUNGER,
    NUTRITION,
    YIELDS,
    STOCK,
    MADE_OF,
    FELT_NEED,
    FELT_NEED_MET_BY,
    FELT_FATIGUE,
    FELT_HEALTH,
    IN_SIGHT,
    PLACE_OF,
    LEADS_TO,
    OPEN,
    WARMTH_OF,
    FELT_BODY_HEAT,
    GOING_TO,
    BLOCKED,
    REFUSED,
];

/// Every mind with senses perceives, each step:
///
/// - **itself** — where it stands, how warm the air is there, how warm its body feels, how tired
///   and how well it feels, where it is trying to go, whether its way is blocked, and what it
///   last could not do (the dead feel only that they are dead, and perceive nothing more);
/// - **what is in view** — where each thing it sees is (stamped *seen* as it comes into sight),
///   and for an opening, where it leads and whether it is open;
/// - **what it lost sight of** — the belief about it is stamped with this moment, the last time
///   it was seen, and then left alone: memory.
///
/// A belief is written only when it changes (or, losing sight, once to stamp it), so a still
/// world writes nothing. Warmth, felt or remembered, is only noticed in steps of the world's
/// declared resolution.
pub struct Perception {
    step: Step,
    warmth_resolution: i64,
    need_resolution: i64,
}

impl Perception {
    /// Perceive as `step` says, noticing changes of warmth of at least `warmth_resolution`
    /// centidegrees, and of fatigue or health of at least `need_resolution` hundredths of a
    /// percent.
    pub const fn new(step: Step, warmth_resolution: i64, need_resolution: i64) -> Self {
        Self {
            step,
            warmth_resolution,
            need_resolution,
        }
    }
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

fn value(view: &dyn CommittedView, key: FactKey) -> Option<Value> {
    view.read(key).map(|f| f.value)
}

fn entities(view: &dyn CommittedView, key: FactKey) -> BTreeSet<EntityId> {
    view.read_all(key)
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(e) => Some(e),
            _ => None,
        })
        .collect()
}

/// The air temperature where `entity` stands: its place's, or the nearest enclosing place's.
fn air_around(view: &dyn CommittedView, entity: EntityId) -> Option<i64> {
    ancestry(view, entity, PHYS_CONTAINED_IN)
        .into_iter()
        .skip(1)
        .find_map(|place| int(view, FactKey::new(place, PHYS_TEMPERATURE)))
}

impl Perception {
    fn propose(
        &self,
        ctx: &TickContext,
        key: FactKey,
        change: Change,
        why: &'static str,
    ) -> Proposal {
        Proposal::new(
            SystemId::new("information.perception"),
            key,
            ctx.basis_tick(),
            change,
            Cause::new(why),
        )
    }

    /// Write `now` at `key` if it differs from what is held (removing it when `now` is `None`).
    fn mirror(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        out: &mut Vec<Proposal>,
        key: FactKey,
        now: Option<Value>,
        why: &'static str,
    ) {
        match (value(view, key), now) {
            (held, Some(v)) if held != Some(v) => {
                out.push(self.propose(ctx, key, Change::Set(v), why));
            }
            (Some(_), None) => out.push(self.propose(ctx, key, Change::Tombstone, why)),
            _ => {}
        }
    }

    /// Write a warmth at `key` if none is held or it has moved by the world's resolution.
    #[allow(clippy::too_many_arguments)]
    fn feel(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        out: &mut Vec<Proposal>,
        key: FactKey,
        now: i64,
        resolution: i64,
    ) {
        // A change smaller than the resolution goes unnoticed — except reaching zero, which
        // always is.
        let changed = int(view, key).map_or(true, |held| {
            (now - held).abs() >= resolution || (now == 0 && held != 0)
        });
        if changed {
            out.push(self.propose(ctx, key, Change::Set(Value::Int(now)), "felt"));
        }
    }

    fn perceive(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        mind: EntityId,
        out: &mut Vec<Proposal>,
    ) {
        // One's health first: the dead feel that, and then nothing more (Amendment A-10).
        if let Some(health) = int(view, FactKey::new(mind, LIVING_HEALTH)) {
            let key = FactKey::new(mind, FELT_HEALTH);
            self.feel(view, ctx, out, key, health, self.need_resolution);
            if health == 0 {
                return;
            }
        }
        if let Some(hungry) = int(view, FactKey::new(mind, LIVING_HUNGER)) {
            let key = FactKey::new(mind, FELT_HUNGER);
            self.feel(view, ctx, out, key, hungry, self.need_resolution);
        }
        if let Some(tired) = int(view, FactKey::new(mind, LIVING_FATIGUE)) {
            let key = FactKey::new(mind, FELT_FATIGUE);
            self.feel(view, ctx, out, key, tired, self.need_resolution);
        }
        // Needs that arose (Amendment A-11): how strongly, and how they are met — and letting go
        // of those that are gone.
        let needs = view.read_about(mind, LIVING_NEED);
        for (object, level) in &needs {
            let Some(level) = level.value.as_int() else {
                continue;
            };
            let key = FactKey::pair(mind, FELT_NEED, *object);
            self.feel(view, ctx, out, key, level, self.need_resolution);
            let met_by = match value(view, FactKey::pair(mind, LIVING_NEED_KIND, *object)) {
                Some(Value::Entity(kind)) => value(view, FactKey::new(kind, LIVING_KIND_MET_BY)),
                _ => None,
            };
            let key = FactKey::pair(mind, FELT_NEED_MET_BY, *object);
            self.mirror(view, ctx, out, key, met_by, "felt");
        }
        for (object, _) in view.read_about(mind, FELT_NEED) {
            if !needs.iter().any(|(o, _)| *o == object) {
                let key = FactKey::pair(mind, FELT_NEED, object);
                out.push(self.propose(ctx, key, Change::Tombstone, "eased"));
                let key = FactKey::pair(mind, FELT_NEED_MET_BY, object);
                if value(view, key).is_some() {
                    out.push(self.propose(ctx, key, Change::Tombstone, "eased"));
                }
            }
        }
        // One's work (Amendment A-18): the job one holds, as Society records it, and what it
        // asks — known as one knows oneself.
        let job = match value(view, FactKey::new(mind, SOC_ROLE)) {
            Some(Value::Entity(job)) => Some(job),
            _ => None,
        };
        let role = job.map(Value::Entity);
        self.mirror(
            view,
            ctx,
            out,
            FactKey::new(mind, WORK_ROLE),
            role,
            "my_work",
        );
        for (mine, society) in [
            (WORK_CARRIES, SOC_JOB_CARRIES),
            (WORK_MAKES, SOC_JOB_MAKES),
            (WORK_FROM, SOC_JOB_FROM),
            (WORK_TO, SOC_JOB_TO),
            (WORK_KEEP, SOC_JOB_KEEP),
            (WORK_HOURS, SOC_JOB_HOURS),
        ] {
            let now = job.and_then(|j| value(view, FactKey::new(j, society)));
            self.mirror(view, ctx, out, FactKey::new(mind, mine), now, "my_work");
        }
        // Oneself: where one stands, the air there, one's body, one's own intent and reports.
        let here = value(view, FactKey::new(mind, PHYS_CONTAINED_IN));
        self.mirror(
            view,
            ctx,
            out,
            FactKey::pair(mind, PLACE_OF, mind),
            here,
            "felt",
        );
        if let (Some(Value::Entity(place)), Some(air)) = (here, air_around(view, mind)) {
            self.feel(
                view,
                ctx,
                out,
                FactKey::pair(mind, WARMTH_OF, place),
                air,
                self.warmth_resolution,
            );
        }
        if let Some(body) = int(view, FactKey::new(mind, LIVING_BODY_HEAT)) {
            self.feel(
                view,
                ctx,
                out,
                FactKey::new(mind, FELT_BODY_HEAT),
                body,
                self.warmth_resolution,
            );
        }
        for (theirs, mine) in [
            (PHYS_TRAVEL_TO, GOING_TO),
            (PHYS_TRAVEL_BLOCKED, BLOCKED),
            (PHYS_ACT_REFUSED, REFUSED),
        ] {
            let now = value(view, FactKey::new(mind, theirs));
            self.mirror(view, ctx, out, FactKey::new(mind, mine), now, "felt");
        }

        // What is in view now, against what the mind was watching.
        let now = entities(view, FactKey::new(mind, PHYS_IN_VIEW));
        let before = entities(view, FactKey::new(mind, IN_SIGHT));
        // Every material seen this step, in anything — each learned once (Amendment A-16).
        let mut materials: BTreeSet<EntityId> = BTreeSet::new();
        for &seen in now.iter().filter(|e| **e != mind) {
            let place = value(view, FactKey::new(seen, PHYS_CONTAINED_IN));
            let key = FactKey::pair(mind, PLACE_OF, seen);
            match place {
                // Come into sight: seen now, whether or not it is where it was last seen.
                Some(place) if !before.contains(&seen) => {
                    out.push(self.propose(ctx, key, Change::Set(place), "seen"));
                }
                Some(_) => self.mirror(view, ctx, out, key, place, "seen"),
                None => {}
            }
            // How much a deposit holds, in whole units (Amendment A-15).
            if let Some(held) = int(view, FactKey::new(seen, RES_STOCK)) {
                let units = Some(Value::Int(held / 100));
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, STOCK, seen),
                    units,
                    "seen",
                );
            }
            // Whose it is (Amendment A-19).
            let owner = value(view, FactKey::new(seen, ECON_OWNER));
            let owner_key = FactKey::pair(mind, OWNER, seen);
            self.mirror(view, ctx, out, owner_key, owner, "seen");
            // What it is made of, as far as can be seen (Amendment A-12).
            let made_of = entities(view, FactKey::new(seen, PHYS_MADE_OF));
            let believed = entities(view, FactKey::pair(mind, MADE_OF, seen));
            let key = FactKey::pair(mind, MADE_OF, seen);
            for m in made_of.difference(&believed) {
                out.push(self.propose(ctx, key, Change::Add(Value::Entity(*m)), "seen"));
            }
            // The materials it shows, for what they feed (Amendment A-16).
            materials.extend(made_of.iter().copied());
            // What a deposit yields (Amendment A-16).
            if let Some(yields) = value(view, FactKey::new(seen, RES_YIELD)) {
                let yields_key = FactKey::pair(mind, YIELDS, seen);
                self.mirror(view, ctx, out, yields_key, Some(yields), "seen");
                if let Value::Entity(m) = yields {
                    materials.insert(m);
                }
            }
            for m in believed.difference(&made_of) {
                out.push(self.propose(ctx, key, Change::Remove(Value::Entity(*m)), "seen"));
            }
            if let Some(leads) = value(view, FactKey::new(seen, PHYS_LEADS_TO)) {
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, LEADS_TO, seen),
                    Some(leads),
                    "seen",
                );
                let open = !matches!(
                    value(view, FactKey::new(seen, PHYS_PORTAL_OPEN)),
                    Some(Value::Bool(false))
                );
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, OPEN, seen),
                    Some(Value::Bool(open)),
                    "seen",
                );
            }
        }
        for m in materials {
            let fed = value(view, FactKey::new(m, PHYS_NUTRITION));
            if fed.is_some() {
                self.mirror(
                    view,
                    ctx,
                    out,
                    FactKey::pair(mind, NUTRITION, m),
                    fed,
                    "seen",
                );
            }
        }
        // What one picks from, one feels how much is left on, though too dark to see it: the
        // step after the pick, and the step after its yield (Amendment A-18).
        if let Some(picked) = view.read(FactKey::new(mind, PHYS_PICKED)) {
            let fresh = view.tick() <= picked.provenance.tick + 1;
            if let (Value::Entity(deposit), true) = (picked.value, fresh) {
                // Seen, it is known already.
                let held =
                    int(view, FactKey::new(deposit, RES_STOCK)).filter(|_| !now.contains(&deposit));
                if let Some(held) = held {
                    let key = FactKey::pair(mind, STOCK, deposit);
                    self.mirror(view, ctx, out, key, Some(Value::Int(held / 100)), "felt");
                }
            }
        }
        // Lost sight of: stamp the belief with the last moment it was seen — then leave it be.
        // Except what was in one's own hand: one feels that it is gone (Amendment A-13) — and,
        // if it was put down where one stands, where it lies (Amendment A-18).
        let standing = value(view, FactKey::new(mind, PHYS_CONTAINED_IN));
        for &gone in before.difference(&now) {
            let key = FactKey::pair(mind, PLACE_OF, gone);
            match value(view, key) {
                Some(Value::Entity(holder)) if holder == mind => {
                    let lies = value(view, FactKey::new(gone, PHYS_CONTAINED_IN));
                    if lies.is_some() && lies == standing {
                        out.push(self.propose(ctx, key, Change::Set(lies.unwrap()), "put_down"));
                    } else {
                        out.push(self.propose(ctx, key, Change::Tombstone, "gone_from_hand"));
                    }
                }
                Some(last) => out.push(self.propose(ctx, key, Change::Set(last), "lost_sight")),
                None => {}
            }
        }
        // Absence (Amendment A-11): standing somewhere lit enough to see — the place itself is
        // in view — a person, a movable thing, or a thing one could carry off (made of something,
        // and no fixture: Amendment A-18) believed here and not seen is not here.
        if let Some(Value::Entity(here)) = value(view, FactKey::new(mind, PHYS_CONTAINED_IN)) {
            if now.contains(&here) {
                for (thing, place) in view.read_about(mind, PLACE_OF) {
                    let portable = view.read(FactKey::new(thing, PHYS_MADE_OF)).is_some()
                        && value(view, FactKey::new(thing, PHYS_SOLID)) != Some(Value::Bool(true));
                    let movable = view.read(FactKey::new(thing, LIVING_BODY_HEAT)).is_some()
                        || value(view, FactKey::new(thing, PHYS_MOBILE)) == Some(Value::Bool(true))
                        || portable;
                    if place.value == Value::Entity(here)
                        && thing != mind
                        && movable
                        && !now.contains(&thing)
                        && !before.contains(&thing)
                    {
                        let key = FactKey::pair(mind, PLACE_OF, thing);
                        out.push(self.propose(ctx, key, Change::Tombstone, "not_there"));
                    }
                }
            }
        }
        let sight = FactKey::new(mind, IN_SIGHT);
        for &came in now.difference(&before) {
            out.push(self.propose(ctx, sight, Change::Add(Value::Entity(came)), "seen"));
        }
        for &went in before.difference(&now) {
            out.push(self.propose(
                ctx,
                sight,
                Change::Remove(Value::Entity(went)),
                "lost_sight",
            ));
        }
    }
}

impl System for Perception {
    fn id(&self) -> SystemId {
        SystemId::new("information.perception")
    }
    fn reads(&self) -> &'static [FactType] {
        PERCEPTION_READS
    }
    fn writes(&self) -> &'static [FactType] {
        PERCEPTION_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        // The minds: everything with senses (Living Systems' sight range) — and anything still
        // watching something, so a mind that loses its senses also loses its view.
        let mut minds: BTreeSet<EntityId> =
            view.entities_with(LIVING_SIGHT_RANGE).into_iter().collect();
        minds.extend(view.entities_with(IN_SIGHT));
        // And anyone who has felt their own health — so a mind that dies, losing its senses,
        // still feels its death.
        minds.extend(view.entities_with(FELT_HEALTH));
        let mut out = Vec::new();
        for mind in minds {
            self.perceive(view, ctx, mind, &mut out);
        }
        out
    }
}

// Fondness reads who each mind is watching, both minds' temperaments, and what it already feels.
const FONDNESS_READS: &[FactType] = &[IN_SIGHT, MIND_TEMPERAMENT, AFFECTION];
const FONDNESS_WRITES: &[FactType] = &[AFFECTION];

/// How compatible two temperaments are, in hundredths of a percent: 100% less the mean
/// difference of their three traits (Amendment A-11).
pub fn compatibility(a: [i64; 3], b: [i64; 3]) -> i64 {
    let differ: i64 = (0..3).map(|i| (a[i] - b[i]).abs()).sum();
    (10_000 - differ / 3).clamp(0, 10_000)
}

/// Fondness (Vol. II Ch. 4; Appendix A, Ruling 2; Amendment A-11): every mind with a temperament
/// grows fonder of each other one it watches — at the world's rate times how compatible they are,
/// never past their compatibility — and cooler toward those it has not seen, at the world's rate
/// of fading. One-way: what Finn feels for Gwen is his alone.
pub struct Fondness {
    step: Step,
    per_hour: i64,
    fade_per_day: i64,
}

impl Fondness {
    /// Grow fond at `per_hour` and fade at `fade_per_day` (hundredths of a percent), stepping as
    /// `step` says.
    pub const fn new(step: Step, per_hour: i64, fade_per_day: i64) -> Self {
        Self {
            step,
            per_hour,
            fade_per_day,
        }
    }
}

impl System for Fondness {
    fn id(&self) -> SystemId {
        SystemId::new("information.fondness")
    }
    fn reads(&self) -> &'static [FactType] {
        FONDNESS_READS
    }
    fn writes(&self) -> &'static [FactType] {
        FONDNESS_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let temperament = |e: EntityId| {
            view.read(FactKey::new(e, MIND_TEMPERAMENT))
                .and_then(|f| f.value.as_vec3())
        };
        let dt = self.step.dt_ms as i128;
        let mut out = Vec::new();
        for mind in view.entities_with(MIND_TEMPERAMENT) {
            let Some(mine) = temperament(mind) else {
                continue;
            };
            let watching = entities(view, FactKey::new(mind, IN_SIGHT));
            let mut others: BTreeSet<EntityId> = watching.clone();
            others.extend(view.read_about(mind, AFFECTION).into_iter().map(|(o, _)| o));
            for other in others.into_iter().filter(|o| *o != mind) {
                let Some(theirs) = temperament(other) else {
                    continue;
                };
                let key = FactKey::pair(mind, AFFECTION, other);
                let held = int(view, key).unwrap_or(0);
                let mut rng = ctx.rng(mind.raw() ^ other.raw().rotate_left(32));
                let next = if watching.contains(&other) {
                    // Fonder for the time together — never past how well they fit.
                    let fit = compatibility(mine, theirs);
                    let gain = kernel::fixed::div_dither(
                        self.per_hour as i128 * fit as i128 * dt / 10_000,
                        3_600_000,
                        &mut rng,
                    ) as i64;
                    if held >= fit {
                        held
                    } else {
                        (held + gain).min(fit)
                    }
                } else {
                    let loss = kernel::fixed::div_dither(
                        self.fade_per_day as i128 * dt,
                        86_400_000,
                        &mut rng,
                    ) as i64;
                    (held - loss).max(0)
                };
                if next == held {
                    continue;
                }
                let (change, why) = match next {
                    0 => (Change::Tombstone, "forgotten"),
                    n if n > held => (Change::Set(Value::Int(n)), "time_together"),
                    n => (Change::Set(Value::Int(n)), "time_apart"),
                };
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new(why),
                ));
            }
        }
        out
    }
}
