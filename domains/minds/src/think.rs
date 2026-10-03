//! Thinking (Vol. I Ch. 4 §11; Vol. V Ch. 9 §9.3; Amendment A-9): pressures, candidates,
//! evaluation against beliefs, commitment, action through intents, and a trace.
//!
//! Every input is a belief or the mind's own commitment; nothing here reads reality. A mind that
//! has never seen or been told of a place cannot choose it, because it is not among the
//! candidates: candidates are drawn from beliefs alone.

use crate::schema::{
    ACT_OPEN, AT, BELIEF_LEADS_TO, BELIEF_PLACE_OF, BELIEF_WARMTH_OF, BLOCKED, FELT_BODY_HEAT,
    GOAL, GOING_TO, REASON, REASON_ROUTINE, REASON_WARMTH, REFUSED, ROUTINE, SCORE, STEP,
    STEP_APPROACH, STEP_GO, STEP_OPEN, TRAVEL_SPEED, TRAVEL_TO, VIA, WALK_SPEED,
};
use kernel::fact::{Cause, Fact, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::SimClock;
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// How many ticks a mind lets pass after acting before it judges what came of it: its intent
/// reaches reality the next tick, and reality reaches its beliefs through perception the tick
/// after that.
pub const SETTLE_TICKS: u64 = 3;

/// The rules every mind decides by, all from the world package (Vol. IV Ch. 2 §2.2).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MindRules {
    /// How often a mind thinks, in seconds of simulated time.
    pub think_step_seconds: u64,
    /// Body heat, in centidegrees, below which a mind feels cold; the further below, the more
    /// it wants warmth.
    pub cold_below_centi_c: i64,
    /// The age, in seconds, at which a belief is trusted half as much as a fresh one.
    pub trust_half_age_seconds: u64,
    /// What each opening on the way costs a choice: the effort of going somewhere.
    pub hop_cost: i64,
    /// What keeping a routine is worth.
    pub routine_value: i64,
    /// How much better a new choice must be before a mind abandons the one it has made.
    pub switch_margin: i64,
}

const THINK_READS: &[FactType] = &[
    BELIEF_PLACE_OF,
    BELIEF_LEADS_TO,
    BELIEF_WARMTH_OF,
    FELT_BODY_HEAT,
    GOING_TO,
    BLOCKED,
    REFUSED,
    WALK_SPEED,
    ROUTINE,
    GOAL,
    REASON,
    SCORE,
    STEP,
    VIA,
    AT,
];
const THINK_WRITES: &[FactType] = &[
    GOAL,
    REASON,
    SCORE,
    STEP,
    VIA,
    AT,
    TRAVEL_TO,
    TRAVEL_SPEED,
    ACT_OPEN,
];

/// Every mind, thinking on the world's cadence (staggered by id so a crowd does not think in
/// lockstep):
///
/// 1. **Pressures** — how cold it feels, against the world's comfort line;
/// 2. **Candidates** — the places it remembers warmer than here, and the routine the hour gives
///    it, each reachable by a way it believes in;
/// 3. **Evaluation** — the cold it feels times the warmth it remembers, trusted less the older
///    the memory; or the worth of the routine; less the cost of the openings on the way;
/// 4. **Commitment** — a goal is kept until reached, unwanted, impossible, or clearly beaten;
/// 5. **Acting** — by travel and open intents: when the way proves shut, it walks to the first
///    opening it believes is on the way and opens it;
/// 6. **Trace** — the goal, its reason, and its winning score, as facts.
pub struct Think {
    clock: SimClock,
    think_ticks: u64,
    day_ms: u64,
    rules: MindRules,
}

impl Think {
    /// Minds deciding by `rules`, on `clock`, over a day of `day_seconds`.
    pub fn new(clock: SimClock, day_seconds: u64, rules: MindRules) -> Self {
        Self {
            clock,
            think_ticks: clock
                .step(rules.think_step_seconds.saturating_mul(1000))
                .period_ticks,
            day_ms: day_seconds.saturating_mul(1000).max(1),
            rules,
        }
    }
}

/// One option a mind weighs.
#[derive(Clone, Copy, Debug)]
struct Choice {
    target: EntityId,
    reason: i64,
    score: i64,
}

/// One mind's view of itself and the world: its beliefs and commitments, and nothing else.
struct Mind<'a> {
    view: &'a dyn CommittedView,
    me: EntityId,
}

impl Mind<'_> {
    fn own(&self, fact: FactType) -> Option<Fact> {
        self.view.read(FactKey::new(self.me, fact))
    }
    fn own_entity(&self, fact: FactType) -> Option<EntityId> {
        match self.own(fact)?.value {
            Value::Entity(e) => Some(e),
            _ => None,
        }
    }
    fn own_int(&self, fact: FactType) -> Option<i64> {
        self.own(fact)?.value.as_int()
    }
    fn belief_entity(&self, fact: FactType, about: EntityId) -> Option<EntityId> {
        match self.view.read(FactKey::pair(self.me, fact, about))?.value {
            Value::Entity(e) => Some(e),
            _ => None,
        }
    }
    fn belief_int(&self, fact: FactType, about: EntityId) -> Option<i64> {
        self.view
            .read(FactKey::pair(self.me, fact, about))?
            .value
            .as_int()
    }

    /// Where the mind believes it stands.
    fn here(&self) -> Option<EntityId> {
        self.belief_entity(BELIEF_PLACE_OF, self.me)
    }

    /// The openings the mind believes in, by the place each is in: `place → [(opening, leads)]`.
    fn ways(&self) -> BTreeMap<EntityId, Vec<(EntityId, EntityId)>> {
        let mut ways: BTreeMap<EntityId, Vec<(EntityId, EntityId)>> = BTreeMap::new();
        for (opening, leads) in self.view.read_about(self.me, BELIEF_LEADS_TO) {
            let (Value::Entity(to), Some(from)) =
                (leads.value, self.belief_entity(BELIEF_PLACE_OF, opening))
            else {
                continue;
            };
            ways.entry(from).or_default().push((opening, to));
        }
        ways
    }

    /// The openings on the shortest way the mind believes leads from `from` to `to` (empty when
    /// they are the same place), or `None` if it knows no way. Ties go to the lower ids.
    fn route(&self, from: EntityId, to: EntityId) -> Option<Vec<EntityId>> {
        if from == to {
            return Some(Vec::new());
        }
        let ways = self.ways();
        let mut came: BTreeMap<EntityId, (EntityId, EntityId)> = BTreeMap::new();
        let mut seen: BTreeSet<EntityId> = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(place) = queue.pop_front() {
            for &(opening, next) in ways.get(&place).into_iter().flatten() {
                if !seen.insert(next) {
                    continue;
                }
                came.insert(next, (place, opening));
                if next == to {
                    let mut path = Vec::new();
                    let mut at = to;
                    while let Some(&(prev, opening)) = came.get(&at) {
                        path.push(opening);
                        at = prev;
                    }
                    path.reverse();
                    return Some(path);
                }
                queue.push_back(next);
            }
        }
        None
    }

    /// Whether the mind knows `target` as a place one can be in: one it stands in, has felt the
    /// air of, or believes an opening leads to.
    fn knows_as_place(&self, target: EntityId) -> bool {
        self.here() == Some(target)
            || self.belief_int(BELIEF_WARMTH_OF, target).is_some()
            || self
                .view
                .read_about(self.me, BELIEF_LEADS_TO)
                .iter()
                .any(|(_, f)| f.value == Value::Entity(target))
    }

    /// The place the mind must reach to reach `target`: the target itself if it is a place,
    /// else where the mind believes the target is.
    fn place_of(&self, target: EntityId) -> Option<EntityId> {
        if self.knows_as_place(target) {
            Some(target)
        } else {
            self.belief_entity(BELIEF_PLACE_OF, target)
        }
    }

    /// Whether the mind is where `target` would take it: in it, or standing by it.
    fn at(&self, target: EntityId) -> bool {
        self.here() == Some(target) || self.own_entity(AT) == Some(target)
    }
}

/// Whether `t` (seconds into the day) lies in the window `[from, to)`, which wraps past midnight
/// when `from > to`.
fn in_window(t: i64, from: i64, to: i64) -> bool {
    if from <= to {
        from <= t && t < to
    } else {
        t >= from || t < to
    }
}

/// A list of proposals from one mind, all from this system.
struct Acts<'a> {
    ctx: &'a TickContext,
    me: EntityId,
    out: &'a mut Vec<Proposal>,
}

impl Acts<'_> {
    fn push(&mut self, fact: FactType, change: Change, why: &'static str) {
        self.out.push(Proposal::new(
            SystemId::new("minds.think"),
            FactKey::new(self.me, fact),
            self.ctx.basis_tick(),
            change,
            Cause::new(why),
        ));
    }
    fn set(&mut self, fact: FactType, value: Value, why: &'static str) {
        self.push(fact, Change::Set(value), why);
    }
    fn clear(&mut self, fact: FactType, why: &'static str) {
        self.push(fact, Change::Tombstone, why);
    }
}

impl Think {
    /// How much a belief formed at `formed` is trusted now, in hundredths of a percent:
    /// `H / (H + age)`, half at the world's half-trust age.
    fn trust(&self, now_tick: u64, formed: u64) -> i64 {
        let age = self
            .clock
            .ms_at(now_tick)
            .saturating_sub(self.clock.ms_at(formed)) as i128;
        let half = (self.rules.trust_half_age_seconds as i128 * 1000).max(1);
        (10_000 * half / (half + age)) as i64
    }

    /// What the mind could do, best first, and what staying put is worth.
    fn options(&self, mind: &Mind, here: EntityId, tick: u64) -> (Vec<Choice>, i64) {
        let r = &self.rules;
        let mut options = Vec::new();
        let mut stay = 0;

        // Warmth: the colder it feels, the more a place remembered warmer than here is worth.
        let cold = mind
            .own_int(FELT_BODY_HEAT)
            .map_or(0, |body| (r.cold_below_centi_c - body).max(0));
        let air_here = mind.belief_int(BELIEF_WARMTH_OF, here);
        if cold > 0 {
            for (place, felt) in mind.view.read_about(mind.me, BELIEF_WARMTH_OF) {
                let (Some(warm), true) = (felt.value.as_int(), place != here) else {
                    continue;
                };
                let gain = warm - air_here.unwrap_or(warm);
                let Some(route) = mind.route(here, place) else {
                    continue;
                };
                if gain <= 0 {
                    continue;
                }
                let worth = cold as i128 * gain as i128 / 100
                    * self.trust(tick, felt.provenance.tick) as i128
                    / 10_000;
                let score = worth as i64 - r.hop_cost * route.len() as i64;
                if score > 0 {
                    options.push(Choice {
                        target: place,
                        reason: REASON_WARMTH,
                        score,
                    });
                }
            }
        }

        // Routines: where the world says this mind belongs at this hour.
        let time_of_day = (self.clock.ms_at(tick) % self.day_ms / 1000) as i64;
        for (target, routine) in mind.view.read_about(mind.me, ROUTINE) {
            let Some([from, to, _]) = routine.value.as_vec3() else {
                continue;
            };
            if !in_window(time_of_day, from, to) {
                continue;
            }
            if mind.at(target) {
                stay = stay.max(r.routine_value);
                continue;
            }
            let Some(route) = mind.place_of(target).and_then(|p| mind.route(here, p)) else {
                continue;
            };
            options.push(Choice {
                target,
                reason: REASON_ROUTINE,
                score: r.routine_value - r.hop_cost * route.len() as i64,
            });
        }

        // Best first; among equals, the lower id.
        options.sort_by(|a, b| b.score.cmp(&a.score).then(a.target.cmp(&b.target)));
        (options, stay)
    }

    fn think(
        &self,
        view: &dyn CommittedView,
        ctx: &TickContext,
        me: EntityId,
        out: &mut Vec<Proposal>,
    ) {
        let mind = Mind { view, me };
        let Some(here) = mind.here() else {
            return; // it does not know where it is: nothing to decide from
        };
        let tick = ctx.tick();
        let (options, stay) = self.options(&mind, here, tick);
        let mut acts = Acts { ctx, me, out };
        let speed = mind.own_int(WALK_SPEED).unwrap_or(0);

        let Some(goal) = mind.own_entity(GOAL) else {
            // Uncommitted: take the best option if it beats staying put.
            if let Some(best) = options.first().filter(|o| o.score > stay) {
                self.adopt(&mut acts, *best, speed);
            }
            return;
        };

        if mind.at(goal) {
            Self::settle(&mut acts, "arrived");
            return;
        }
        let Some(kept) = options.iter().find(|o| o.target == goal) else {
            // No longer wanted, or no way to it believed: let it go, and stop walking for it.
            Self::settle(&mut acts, "no_longer_wanted");
            if mind.own_entity(GOING_TO).is_some() {
                acts.clear(TRAVEL_TO, "no_longer_wanted");
            }
            return;
        };
        if let Some(best) = options
            .first()
            .filter(|b| b.target != goal && b.score > kept.score + self.rules.switch_margin)
        {
            self.adopt(&mut acts, *best, speed);
            return;
        }
        self.pursue(&mind, &mut acts, here, goal, tick, speed);
    }

    /// Commit to `choice`: record the trace, and set off.
    fn adopt(&self, acts: &mut Acts, choice: Choice, speed: i64) {
        let why = if choice.reason == REASON_WARMTH {
            "cold"
        } else {
            "routine"
        };
        acts.set(GOAL, Value::Entity(choice.target), why);
        acts.set(REASON, Value::Int(choice.reason), why);
        acts.set(SCORE, Value::Int(choice.score), why);
        acts.clear(AT, why);
        Self::go(acts, choice.target, speed, why);
    }

    /// Travel toward `target`.
    fn go(acts: &mut Acts, target: EntityId, speed: i64, why: &'static str) {
        acts.set(STEP, Value::Int(STEP_GO), why);
        acts.clear(VIA, why);
        acts.set(TRAVEL_TO, Value::Entity(target), why);
        acts.set(TRAVEL_SPEED, Value::Int(speed), why);
    }

    /// Let go of the goal and its plan.
    fn settle(acts: &mut Acts, why: &'static str) {
        for fact in [GOAL, REASON, SCORE, STEP, VIA] {
            acts.clear(fact, why);
        }
    }

    /// Carry the plan for `goal` one step further, once the last step has had time to tell.
    fn pursue(
        &self,
        mind: &Mind,
        acts: &mut Acts,
        here: EntityId,
        goal: EntityId,
        tick: u64,
        speed: i64,
    ) {
        let Some(step) = mind.own(STEP) else {
            Self::go(acts, goal, speed, "resume");
            return;
        };
        if tick < step.provenance.tick + SETTLE_TICKS {
            return;
        }
        let blocked = mind.own(BLOCKED).map(|f| f.value) == Some(Value::Bool(true));
        let going = mind.own_entity(GOING_TO);
        let via = mind.own_entity(VIA);
        match step.value.as_int() {
            Some(STEP_GO) if blocked => {
                // The way is shut somewhere. Go to the first opening believed to be on it.
                let first = mind
                    .place_of(goal)
                    .and_then(|p| mind.route(here, p))
                    .and_then(|r| r.first().copied());
                match first {
                    Some(opening) => {
                        acts.set(STEP, Value::Int(STEP_APPROACH), "way_shut");
                        acts.set(VIA, Value::Entity(opening), "way_shut");
                        acts.set(TRAVEL_TO, Value::Entity(opening), "way_shut");
                        acts.set(TRAVEL_SPEED, Value::Int(speed), "way_shut");
                    }
                    None => Self::settle(acts, "no_way"),
                }
            }
            Some(STEP_GO) if going.is_none() => {
                // The walk ended short of the goal — or, for a thing in this place, beside it.
                if mind.place_of(goal) == Some(here) && !mind.knows_as_place(goal) {
                    acts.set(AT, Value::Entity(goal), "arrived");
                    Self::settle(acts, "arrived");
                } else {
                    Self::go(acts, goal, speed, "resume");
                }
            }
            Some(STEP_APPROACH) if blocked => Self::settle(acts, "no_way"),
            Some(STEP_APPROACH) if going.is_none() => {
                if let Some(opening) = via {
                    acts.set(STEP, Value::Int(STEP_OPEN), "opening_the_way");
                    acts.set(ACT_OPEN, Value::Entity(opening), "opening_the_way");
                }
            }
            Some(STEP_OPEN) => {
                if via.is_some() && mind.own_entity(REFUSED) == via {
                    Self::settle(acts, "could_not_open");
                } else {
                    Self::go(acts, goal, speed, "the_way_is_open");
                }
            }
            _ => {}
        }
    }
}

impl System for Think {
    fn id(&self) -> SystemId {
        SystemId::new("minds.think")
    }
    fn reads(&self) -> &'static [FactType] {
        THINK_READS
    }
    fn writes(&self) -> &'static [FactType] {
        THINK_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for me in view.entities_with(WALK_SPEED) {
            // Staggered: each mind thinks on its own beat of the world's cadence.
            if (ctx.tick() + me.raw()) % self.think_ticks.max(1) == 0 {
                self.think(view, ctx, me, &mut out);
            }
        }
        out
    }
}
