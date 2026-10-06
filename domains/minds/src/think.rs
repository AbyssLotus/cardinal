//! Thinking (Vol. I Ch. 4 §11; Vol. V Ch. 9 §9.3; Amendment A-9): pressures, candidates,
//! evaluation against beliefs, commitment, action through intents, and a trace.
//!
//! Every input is a belief or the mind's own commitment; nothing here reads reality. A mind that
//! has never seen or been told of a place cannot choose it, because it is not among the
//! candidates: candidates are drawn from beliefs alone.

use crate::schema::{
    ABOUT, ACT_CLAIM, ACT_CONSUME, ACT_DROP, ACT_MAKE, ACT_OPEN, ACT_PICK, ACT_TAKE, AT,
    BELIEF_LEADS_TO, BELIEF_MADE_OF, BELIEF_NUTRITION, BELIEF_OWNER, BELIEF_PLACE_OF,
    BELIEF_RECIPE_AT, BELIEF_RECIPE_MAKES, BELIEF_RECIPE_NEEDS, BELIEF_STOCK, BELIEF_WARMTH_OF,
    BELIEF_YIELDS, BLOCKED, CURIOSITY, DIRECTED, FELT_BODY_HEAT, FELT_FATIGUE, FELT_HEALTH,
    FELT_HUNGER, FELT_NEED, FELT_NEED_MET_BY, GAVE_UP_ON, GOAL, GOING_TO, HOME, LIKES, MET_BY_DOSE,
    MET_BY_PRESENCE, OBJECT, REASON, REASON_CURIOSITY, REASON_DOSE, REASON_HUNGER, REASON_NEED,
    REASON_ROUTINE, REASON_WANT, REASON_WARMTH, REASON_WORK, REFUSED, REST, RESTING, ROUTINE,
    SCORE, STEP, STEP_APPROACH, STEP_CLAIM, STEP_CONSUME, STEP_DROP, STEP_FETCH, STEP_GO,
    STEP_MAKE, STEP_OPEN, STEP_PICK, STEP_TAKE, TRAVEL_SPEED, TRAVEL_TO, VIA, WALK_SPEED,
    WORK_CARRIES, WORK_FROM, WORK_HOURS, WORK_KEEP, WORK_MAKES, WORK_ROLE, WORK_TO,
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

/// How many ticks a mind waits after picking or eating before doing it again: what is picked is
/// reported, made, placed in the hand, and only then felt there — twice as long as an act takes
/// to tell.
pub const FEEL_TICKS: u64 = 2 * SETTLE_TICKS;

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
    /// Fatigue, in hundredths of a percent, above which a mind is tired: it rests when it has
    /// nowhere it wants to go, and lets routines wait (Amendment A-10).
    pub tired_above: i64,
    /// Fatigue below which a resting mind is rested and gets up.
    pub rested_below: i64,
    /// How strongly a need that arose must be felt, in hundredths of a percent, before it moves
    /// a mind (Amendment A-11).
    pub need_above: i64,
    /// What each hundredth of a percent of need beyond that line is worth, in percent of a unit.
    pub need_weight: i64,
    /// Hunger past which a mind goes looking for food (Amendment A-16).
    pub hungry_above: i64,
    /// What working one's job is worth, in its hours (Amendment A-18).
    pub work_value: i64,
    /// When the world's sleeping hours begin and end, in seconds into the day (a window that
    /// wraps past midnight when `from > to`): in them, a mind with nothing better to do lies down
    /// once at all tired, and stays down till they end unless something calls it (Amendment
    /// A-18).
    pub sleep_from_seconds: i64,
    /// When they end.
    pub sleep_to_seconds: i64,
}

const THINK_READS: &[FactType] = &[
    BELIEF_OWNER,
    CURIOSITY,
    LIKES,
    HOME,
    OBJECT,
    GAVE_UP_ON,
    WORK_ROLE,
    WORK_CARRIES,
    WORK_MAKES,
    WORK_FROM,
    WORK_TO,
    WORK_KEEP,
    WORK_HOURS,
    BELIEF_RECIPE_NEEDS,
    BELIEF_RECIPE_MAKES,
    BELIEF_RECIPE_AT,
    ABOUT,
    FELT_HUNGER,
    BELIEF_NUTRITION,
    BELIEF_YIELDS,
    BELIEF_STOCK,
    DIRECTED,
    BELIEF_MADE_OF,
    FELT_NEED,
    FELT_NEED_MET_BY,
    FELT_FATIGUE,
    FELT_HEALTH,
    RESTING,
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
    ACT_CLAIM,
    OBJECT,
    GAVE_UP_ON,
    ACT_DROP,
    ACT_MAKE,
    ABOUT,
    ACT_PICK,
    ACT_TAKE,
    ACT_CONSUME,
    RESTING,
    REST,
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
/// 6. **Trace** — the goal, its reason, and its winning score, as facts;
/// 7. **Rest** — tired, with nowhere it wants to go, it lies down; it gets up when rested, when
///    the cold calls, or when a routine calls and it is no longer tired. The dead do not think
///    (Amendment A-10).
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
    // For a craving: the substance the goal is for.
    about: Option<EntityId>,
}

/// What a mind's work or wants ask of it next (Amendments A-18, A-19).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Work {
    /// Put this down in the store.
    Deliver(EntityId),
    /// Get this: take it, or pick from it if it bears.
    Get(EntityId),
    /// Make this recipe at its workplace.
    Make(EntityId),
    /// Claim this, which no one owns (Amendment A-19).
    Claim(EntityId),
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

    /// What the mind believes it carries, in ascending id.
    fn carried(&self) -> Vec<EntityId> {
        self.view
            .read_about(self.me, BELIEF_PLACE_OF)
            .into_iter()
            .filter(|(t, p)| *t != self.me && p.value == Value::Entity(self.me))
            .map(|(t, _)| t)
            .collect()
    }

    /// Whether the mind gave up trying to get `thing`, and has learned nothing newer of where it is
    /// since (Amendment A-18).
    fn gave_up_on(&self, thing: EntityId) -> bool {
        let Some(gave_up) = self.belief_int(GAVE_UP_ON, thing) else {
            return false;
        };
        let learned = self
            .view
            .read(FactKey::pair(self.me, BELIEF_PLACE_OF, thing))
            .map_or(0, |f| f.provenance.tick);
        gave_up >= learned as i64
    }

    /// Whether the mind believes someone else owns `thing` (Amendment A-19).
    fn others(&self, thing: EntityId) -> bool {
        self.belief_entity(BELIEF_OWNER, thing)
            .is_some_and(|owner| owner != self.me)
    }

    /// What `thing` is worth to the mind: how much it likes the best of what it believes the thing
    /// is made of (Amendment A-19).
    fn worth(&self, thing: EntityId) -> i64 {
        self.view
            .read_all(FactKey::pair(self.me, BELIEF_MADE_OF, thing))
            .into_iter()
            .filter_map(|m| match m.value {
                Value::Entity(m) => self.belief_int(LIKES, m),
                _ => None,
            })
            .max()
            .unwrap_or(0)
    }

    /// Whether the mind believes `thing` is made of `material`.
    fn made_of(&self, thing: EntityId, material: EntityId) -> bool {
        self.view
            .read_all(FactKey::pair(self.me, BELIEF_MADE_OF, thing))
            .iter()
            .any(|m| m.value == Value::Entity(material))
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
    /// Give up trying to get `thing`, for now (Amendment A-18).
    fn give_up_on(&mut self, thing: EntityId) {
        self.out.push(Proposal::new(
            SystemId::new("minds.think"),
            FactKey::pair(self.me, GAVE_UP_ON, thing),
            self.ctx.basis_tick(),
            Change::Set(Value::Int(self.ctx.tick() as i64)),
            Cause::new("could_not_reach_it"),
        ));
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
                        about: None,
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
                about: None,
            });
        }

        // Needs that arose, met by presence: go where their object is believed to be
        // (Amendment A-11). Believed here already, there is nothing to go for.
        for (object, felt) in mind.view.read_about(mind.me, FELT_NEED) {
            let urgency = felt.value.as_int().unwrap_or(0) - r.need_above;
            let met_by = mind.belief_int(FELT_NEED_MET_BY, object);
            if urgency <= 0 || met_by != Some(MET_BY_PRESENCE) {
                continue;
            }
            let Some(place) = mind.belief_entity(BELIEF_PLACE_OF, object) else {
                continue; // it does not know where they are
            };
            let Some(route) = mind.route(here, place).filter(|_| place != here) else {
                continue;
            };
            options.push(Choice {
                target: place,
                reason: REASON_NEED,
                score: urgency * r.need_weight / 100 - r.hop_cost * route.len() as i64,
                about: None,
            });
        }

        // Needs that arose, met by a dose: things believed made of the substance, wherever they
        // are believed to be — in hand, or in a place a way is known to (Amendment A-13).
        for (substance, felt) in mind.view.read_about(mind.me, FELT_NEED) {
            let urgency = felt.value.as_int().unwrap_or(0) - r.need_above;
            if urgency <= 0 || mind.belief_int(FELT_NEED_MET_BY, substance) != Some(MET_BY_DOSE) {
                continue;
            }
            let mut things: Vec<EntityId> = mind
                .view
                .read_about(mind.me, BELIEF_MADE_OF)
                .into_iter()
                .filter(|(_, m)| m.value == Value::Entity(substance))
                .map(|(t, _)| t)
                .collect();
            things.dedup();
            for thing in things {
                if mind.gave_up_on(thing) || mind.others(thing) {
                    continue;
                }
                let Some(place) = mind.belief_entity(BELIEF_PLACE_OF, thing) else {
                    continue;
                };
                let hops = if place == mind.me {
                    0
                } else {
                    match mind.route(here, place) {
                        Some(route) => route.len() as i64,
                        None => continue,
                    }
                };
                options.push(Choice {
                    target: thing,
                    reason: REASON_DOSE,
                    score: urgency * r.need_weight / 100 - r.hop_cost * hops,
                    about: Some(substance),
                });
            }
        }

        // Hunger (Amendment A-16): food believed in hand or somewhere a way is known to, and
        // trees believed to bear it — the more filling, the better (Amendment A-18): each
        // hundredth of a percent of hunger a food is believed to ease adds a hundredth of a unit.
        let hunger = mind.own_int(FELT_HUNGER).unwrap_or(0) - r.hungry_above;
        if hunger > 0 {
            let feeds = |m: EntityId| mind.belief_int(BELIEF_NUTRITION, m).is_some_and(|n| n > 0);
            let fills = |target: EntityId| {
                let material = mind.belief_entity(BELIEF_YIELDS, target).or_else(|| {
                    mind.view
                        .read_all(FactKey::pair(mind.me, BELIEF_MADE_OF, target))
                        .into_iter()
                        .find_map(|m| match m.value {
                            Value::Entity(m) if feeds(m) => Some(m),
                            _ => None,
                        })
                });
                material
                    .and_then(|m| mind.belief_int(BELIEF_NUTRITION, m))
                    .unwrap_or(0)
                    / 100
            };
            let mut food: Vec<EntityId> = mind
                .view
                .read_about(mind.me, BELIEF_MADE_OF)
                .into_iter()
                .filter(|(_, m)| matches!(m.value, Value::Entity(m) if feeds(m)))
                .map(|(t, _)| t)
                .collect();
            food.dedup();
            let trees = mind
                .view
                .read_about(mind.me, BELIEF_YIELDS)
                .into_iter()
                .filter(|(d, m)| {
                    matches!(m.value, Value::Entity(m) if feeds(m))
                        && mind.belief_int(BELIEF_STOCK, *d).is_some_and(|n| n > 0)
                })
                .map(|(d, _)| d);
            for target in food.into_iter().chain(trees) {
                if mind.gave_up_on(target) || mind.others(target) {
                    continue;
                }
                let Some(place) = mind.belief_entity(BELIEF_PLACE_OF, target) else {
                    continue;
                };
                let hops = if place == mind.me {
                    0
                } else {
                    match mind.route(here, place) {
                        Some(route) => route.len() as i64,
                        None => continue,
                    }
                };
                options.push(Choice {
                    target,
                    reason: REASON_HUNGER,
                    score: hunger * r.need_weight / 100 + fills(target) - r.hop_cost * hops,
                    about: None,
                });
            }
        }

        // Work (Amendment A-18): the job the mind holds, in its hours, while there is something
        // to do for it — at the worth the world gives work, less the way to where it happens.
        if let Some(job) = mind.own_entity(WORK_ROLE) {
            if let Some((_, place)) = self.work_next(mind, here, tick) {
                let hops = if place == here || place == mind.me {
                    Some(0)
                } else {
                    mind.route(here, place).map(|r| r.len() as i64)
                };
                if let Some(hops) = hops {
                    options.push(Choice {
                        target: job,
                        reason: REASON_WORK,
                        score: r.work_value - r.hop_cost * hops,
                        about: None,
                    });
                }
            }
        }

        // Curiosity (Amendment A-19): places it knows a way to and has never stood in — it has no
        // feel of their air.
        let curiosity = mind.own_int(CURIOSITY).unwrap_or(0);
        if curiosity > 0 {
            let mut unseen: Vec<EntityId> = mind
                .view
                .read_about(mind.me, BELIEF_LEADS_TO)
                .into_iter()
                .filter_map(|(_, f)| match f.value {
                    Value::Entity(place) => Some(place),
                    _ => None,
                })
                .filter(|p| *p != here && mind.belief_int(BELIEF_WARMTH_OF, *p).is_none())
                .collect();
            unseen.sort_unstable();
            unseen.dedup();
            for place in unseen {
                if let Some(route) = mind.route(here, place) {
                    options.push(Choice {
                        target: place,
                        reason: REASON_CURIOSITY,
                        score: curiosity - r.hop_cost * route.len() as i64,
                        about: None,
                    });
                }
            }
        }

        // Wants (Amendment A-19): what it owns, to keep at home; what no one owns, made of
        // something it likes, to claim.
        if let Some((thing, _, place, worth)) = self.want_next(mind, here) {
            let hops = if place == here || place == mind.me {
                Some(0)
            } else {
                mind.route(here, place).map(|r| r.len() as i64)
            };
            if let Some(hops) = hops {
                options.push(Choice {
                    target: thing,
                    reason: REASON_WANT,
                    score: worth - r.hop_cost * hops,
                    about: None,
                });
            }
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
        // The dead do not think (Amendment A-10).
        if mind.own_int(FELT_HEALTH) == Some(0) {
            return;
        }
        let Some(here) = mind.here() else {
            return; // it does not know where it is: nothing to decide from
        };
        let tick = ctx.tick();
        let (mut options, stay) = self.options(&mind, here, tick);
        let mut acts = Acts { ctx, me, out };
        let speed = mind.own_int(WALK_SPEED).unwrap_or(0);

        // Tired, a mind lets its routines and its work wait; the cold still moves it. In the
        // sleeping hours, at all tired is tired enough to lie down (Amendment A-18).
        let fatigue = mind.own_int(FELT_FATIGUE).unwrap_or(0);
        let tired = fatigue > self.rules.tired_above;
        if tired {
            options.retain(|o| o.reason != REASON_ROUTINE && o.reason != REASON_WORK);
        }
        let time = (self.clock.ms_at(tick) % self.day_ms / 1000) as i64;
        let night = in_window(
            time,
            self.rules.sleep_from_seconds,
            self.rules.sleep_to_seconds,
        );
        let sleepy = tired || (night && fatigue > self.rules.rested_below);
        let best = options.first().copied().filter(|o| o.score > stay);

        if mind.own(RESTING).map(|f| f.value) == Some(Value::Bool(true)) {
            // Resting: get up for something worth getting up for, or once rested.
            if let Some(best) = best {
                Self::get_up(&mut acts, "called_away");
                self.adopt(&mut acts, best, speed);
            } else if fatigue < self.rules.rested_below && !night {
                Self::get_up(&mut acts, "rested");
            }
            return;
        }

        let Some(goal) = mind.own_entity(GOAL) else {
            // Uncommitted: take the best option if it beats staying put; else, if tired, rest.
            if let Some(best) = best {
                self.adopt(&mut acts, best, speed);
            } else if sleepy {
                let why = if tired { "tired" } else { "bedtime" };
                acts.set(REST, Value::Bool(true), why);
                acts.set(RESTING, Value::Bool(true), why);
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

    /// Get up from resting.
    fn get_up(acts: &mut Acts, why: &'static str) {
        acts.clear(REST, why);
        acts.clear(RESTING, why);
    }

    /// Commit to `choice`: record the trace, and set off.
    fn adopt(&self, acts: &mut Acts, choice: Choice, speed: i64) {
        let why = match choice.reason {
            REASON_WARMTH => "cold",
            REASON_ROUTINE => "routine",
            REASON_DOSE => "craving",
            REASON_HUNGER => "hungry",
            REASON_WORK => "work",
            REASON_CURIOSITY => "curious",
            REASON_WANT => "wanting",
            _ => "longing",
        };
        acts.set(GOAL, Value::Entity(choice.target), why);
        acts.set(REASON, Value::Int(choice.reason), why);
        acts.set(SCORE, Value::Int(choice.score), why);
        acts.clear(AT, why);
        acts.clear(OBJECT, why);
        match choice.about {
            Some(substance) => acts.set(ABOUT, Value::Entity(substance), why),
            None => acts.clear(ABOUT, why),
        }
        if matches!(
            choice.reason,
            REASON_DOSE | REASON_HUNGER | REASON_WORK | REASON_WANT
        ) {
            // Something to fetch and consume, or work to do: the plan begins on the next thought,
            // from what is believed then (see `fetch` and `work`).
            acts.clear(STEP, why);
            return;
        }
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
        for fact in [GOAL, REASON, SCORE, STEP, VIA, OBJECT] {
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
        if matches!(mind.own_int(REASON), Some(REASON_DOSE | REASON_HUNGER)) {
            self.fetch(mind, acts, here, goal, tick, speed);
        } else if mind.own_int(REASON) == Some(REASON_WORK) {
            self.work(mind, acts, here, tick, speed);
        } else if mind.own_int(REASON) == Some(REASON_WANT) {
            self.want(mind, acts, here, goal, tick, speed);
        } else {
            self.travel_toward(mind, acts, here, goal, tick, speed);
        }
    }

    /// Fetch something and consume it (Amendments A-13, A-16): whatever is in hand that meets the
    /// need is consumed first; otherwise go to where the target is believed to be — a thing to
    /// walk up to and take, or a tree to walk up to and pick — and once something that meets the
    /// need is felt in hand, consume it. What cannot be taken or picked is given up on; what has
    /// been eaten is gone from the hand, and the next thought finds the goal met or renewed.
    fn fetch(
        &self,
        mind: &Mind,
        acts: &mut Acts,
        here: EntityId,
        target: EntityId,
        tick: u64,
        speed: i64,
    ) {
        let step = mind.own(STEP);
        let code = step.as_ref().and_then(|s| s.value.as_int());
        let felt = step
            .as_ref()
            .map_or(true, |s| tick >= s.provenance.tick + FEEL_TICKS);
        // What meets the need: the craved substance, or anything that feeds.
        let about = mind.own_entity(ABOUT);
        let made_of = mind.view.read_about(mind.me, BELIEF_MADE_OF);
        let meets = |thing: EntityId| {
            made_of
                .iter()
                .filter(|(t, _)| *t == thing)
                .any(|(_, m)| match (m.value, about) {
                    (Value::Entity(m), Some(substance)) => m == substance,
                    (Value::Entity(m), None) => {
                        mind.belief_int(BELIEF_NUTRITION, m).is_some_and(|n| n > 0)
                    }
                    _ => false,
                })
        };
        // 1. In hand already: consume it.
        let in_hand = mind
            .view
            .read_about(mind.me, BELIEF_PLACE_OF)
            .into_iter()
            .filter(|(t, p)| *t != mind.me && p.value == Value::Entity(mind.me))
            .map(|(t, _)| t)
            .find(|t| meets(*t));
        if let Some(thing) = in_hand {
            if code != Some(STEP_CONSUME) || felt {
                acts.set(STEP, Value::Int(STEP_CONSUME), "eating");
                acts.set(ACT_CONSUME, Value::Entity(thing), "eating");
            }
            return;
        }
        // 2. Go and get it — one, so wait to feel it before reaching for another.
        self.get(mind, acts, here, target, tick, speed, FEEL_TICKS);
    }

    /// Get `thing` (Amendments A-13, A-16, A-18): go to where it is believed to be, walk up to it,
    /// and take it — or pick from it, if it bears. A step about something else starts over. What
    /// cannot be taken or picked is given up on; picked or taken, and still wanted, try again
    /// `patience` ticks on.
    #[allow(clippy::too_many_arguments)]
    fn get(
        &self,
        mind: &Mind,
        acts: &mut Acts,
        here: EntityId,
        thing: EntityId,
        tick: u64,
        speed: i64,
        patience: u64,
    ) {
        let step = mind.own(STEP);
        let code = step.as_ref().and_then(|s| s.value.as_int());
        let settled = step
            .as_ref()
            .map_or(true, |s| tick >= s.provenance.tick + SETTLE_TICKS);
        let again = step
            .as_ref()
            .map_or(true, |s| tick >= s.provenance.tick + patience);
        let Some(place) = mind.belief_entity(BELIEF_PLACE_OF, thing) else {
            Self::settle(acts, "lost_it");
            return;
        };
        if place != here && place != mind.me {
            self.travel_toward(mind, acts, here, place, tick, speed);
            return;
        }
        // There: walk up to it, then take it — or pick from it, if it bears.
        let going = mind.own_entity(GOING_TO);
        let blocked = mind.own(BLOCKED).map(|f| f.value) == Some(Value::Bool(true));
        let refused = mind.own_entity(REFUSED) == Some(thing);
        let bears = mind.belief_entity(BELIEF_YIELDS, thing).is_some();
        let same = mind.own_entity(OBJECT) == Some(thing);
        let (get_step, get_act, why) = if bears {
            (STEP_PICK, ACT_PICK, "picking")
        } else {
            (STEP_TAKE, ACT_TAKE, "taking")
        };
        match code {
            Some(STEP_FETCH) if same && settled && blocked => {
                Self::settle(acts, "could_not_reach_it");
                acts.give_up_on(thing);
                if going.is_some() {
                    acts.clear(TRAVEL_TO, "could_not_reach_it");
                }
            }
            Some(STEP_FETCH) if same && settled && going.is_none() => {
                acts.set(STEP, Value::Int(get_step), why);
                acts.set(get_act, Value::Entity(thing), why);
            }
            Some(STEP_TAKE | STEP_PICK) if same && settled && refused => {
                Self::settle(acts, "could_not_reach_it");
                acts.give_up_on(thing);
            }
            // Nothing came to hand yet, or more is wanted: again, once patient enough.
            Some(STEP_TAKE | STEP_PICK) if same && again => {
                acts.set(STEP, Value::Int(get_step), why);
                acts.set(get_act, Value::Entity(thing), why);
            }
            Some(STEP_FETCH | STEP_TAKE | STEP_PICK) if same => {}
            _ if settled || code.is_none() || !same => {
                acts.set(STEP, Value::Int(STEP_FETCH), "fetching");
                acts.set(OBJECT, Value::Entity(thing), "fetching");
                acts.clear(VIA, "fetching");
                acts.set(TRAVEL_TO, Value::Entity(thing), "fetching");
                acts.set(TRAVEL_SPEED, Value::Int(speed), "fetching");
            }
            _ => {}
        }
    }

    /// What the mind's work asks of it next, and the place where that happens (Amendment A-18).
    ///
    /// The goods are what the job carries, or what its recipe makes. More are wanted in the job's
    /// hours while the store, as the mind believes it, holds fewer than the job keeps — each
    /// believed thing there counted by how far the mind trusts a memory that old, so a store not
    /// seen for a while is looked at again. What more is wanted is got from the job's source, or,
    /// for a recipe, what it needs and the mind does not carry, nearest first; with all of it in
    /// hand, the recipe is made at its workplace. What was made goes to the store at once;
    /// carried goods go once no more are wanted or can be got.
    fn work_next(&self, mind: &Mind, here: EntityId, tick: u64) -> Option<(Work, EntityId)> {
        let store = mind.own_entity(WORK_TO)?;
        let keep = mind.own_int(WORK_KEEP).unwrap_or(0).max(0);
        let time = (self.clock.ms_at(tick) % self.day_ms / 1000) as i64;
        let on_duty = mind
            .own(WORK_HOURS)
            .and_then(|f| f.value.as_vec3())
            .is_some_and(|[from, to, _]| in_window(time, from, to));
        let recipe = mind.own_entity(WORK_MAKES);
        let goods = match (mind.own_entity(WORK_CARRIES), recipe) {
            (Some(material), _) => material,
            (None, Some(r)) => mind.belief_entity(BELIEF_RECIPE_MAKES, r)?,
            (None, None) => return None,
        };
        let held = mind.carried();
        let carrying: Vec<EntityId> = held
            .iter()
            .copied()
            .filter(|t| mind.made_of(*t, goods))
            .collect();
        // How stocked the store is believed to be, in hundredths of a percent of a thing.
        let stocked: i64 = mind
            .view
            .read_about(mind.me, BELIEF_PLACE_OF)
            .into_iter()
            .filter(|(t, p)| p.value == Value::Entity(store) && mind.made_of(*t, goods))
            .map(|(_, p)| self.trust(tick, p.provenance.tick))
            .sum();
        let wanted = on_duty && stocked + carrying.len() as i64 * 10_000 < keep * 10_000;
        let next = if !wanted {
            None
        } else {
            match recipe {
                None => {
                    let from = mind.own_entity(WORK_FROM);
                    self.source_of(mind, here, goods, from, Some(store))
                }
                Some(r) => self.for_recipe(mind, here, r, &held),
            }
        };
        if let Some(&thing) = carrying.first() {
            if recipe.is_some() || next.is_none() {
                return Some((Work::Deliver(thing), store));
            }
        }
        next
    }

    /// What making `recipe` asks next: the nearest source of the first thing it needs that the
    /// mind does not carry, or, with everything in hand, the making itself, where its workplace
    /// is believed to be.
    fn for_recipe(
        &self,
        mind: &Mind,
        here: EntityId,
        recipe: EntityId,
        held: &[EntityId],
    ) -> Option<(Work, EntityId)> {
        let workplace = mind.belief_entity(BELIEF_RECIPE_AT, recipe)?;
        let at = mind.belief_entity(BELIEF_PLACE_OF, workplace)?;
        let mut used: BTreeSet<EntityId> = BTreeSet::new();
        for need in mind
            .view
            .read_all(FactKey::pair(mind.me, BELIEF_RECIPE_NEEDS, recipe))
        {
            let Some([material, count, _]) = need.value.as_vec3() else {
                continue;
            };
            let material = EntityId::from_raw(material as u64);
            let count = count.max(0) as usize;
            let have: Vec<EntityId> = held
                .iter()
                .copied()
                .filter(|t| !used.contains(t) && mind.made_of(*t, material))
                .take(count)
                .collect();
            let short = have.len() < count;
            used.extend(have);
            if short {
                return self.source_of(mind, here, material, None, None);
            }
        }
        Some((Work::Make(recipe), at))
    }

    /// The nearest source of `material` the mind believes in: a deposit believed to bear it and
    /// not bare, or a thing believed made of it, lying in a place a way is known to — only at
    /// `within` if given, and never in `not_in`. Ties go to the lower id.
    fn source_of(
        &self,
        mind: &Mind,
        here: EntityId,
        material: EntityId,
        within: Option<EntityId>,
        not_in: Option<EntityId>,
    ) -> Option<(Work, EntityId)> {
        let mut best: Option<(usize, EntityId, EntityId)> = None;
        let mut consider = |thing: EntityId, place: EntityId| {
            if place == mind.me
                || Some(place) == not_in
                || mind.gave_up_on(thing)
                || mind.others(thing)
            {
                return;
            }
            if within.is_some_and(|w| thing != w && place != w) {
                return;
            }
            let Some(route) = mind.route(here, place) else {
                return; // in someone's hand, or nowhere a way is known to
            };
            let key = (route.len(), thing, place);
            if best.map_or(true, |b| (key.0, key.1) < (b.0, b.1)) {
                best = Some(key);
            }
        };
        for (deposit, yields) in mind.view.read_about(mind.me, BELIEF_YIELDS) {
            let bears = yields.value == Value::Entity(material)
                && mind
                    .belief_int(BELIEF_STOCK, deposit)
                    .is_some_and(|n| n > 0);
            if let (true, Some(place)) = (bears, mind.belief_entity(BELIEF_PLACE_OF, deposit)) {
                consider(deposit, place);
            }
        }
        for (thing, place) in mind.view.read_about(mind.me, BELIEF_PLACE_OF) {
            if let Value::Entity(place) = place.value {
                if thing != mind.me && mind.made_of(thing, material) {
                    consider(thing, place);
                }
            }
        }
        best.map(|(_, thing, place)| (Work::Get(thing), place))
    }

    /// Carry the mind's work one step further (Amendment A-18): get what it needs, take what it
    /// carries to the store and put it down there, one thing at a time, or make the recipe at its
    /// workplace. With nothing left to do, the goal is let go.
    fn work(&self, mind: &Mind, acts: &mut Acts, here: EntityId, tick: u64, speed: i64) {
        let Some((next, place)) = self.work_next(mind, here, tick) else {
            Self::settle(acts, "work_done");
            return;
        };
        self.carry_out(mind, acts, here, next, place, tick, speed);
    }

    /// What the mind wants next (Amendment A-19): the thing it concerns, the step, where that
    /// happens, and what the thing is worth to it. In order: put down at home what it owns and
    /// carries; claim what it carries that no one owns and it likes; else the best thing to fetch
    /// — one it owns lying away from home, or one no one owns, made of something it likes — less
    /// the way there. Never what it believes someone else owns, nor an opening, a place, or what
    /// it gave up on.
    fn want_next(&self, mind: &Mind, here: EntityId) -> Option<(EntityId, Work, EntityId, i64)> {
        let home = mind.own_entity(HOME);
        let mine = |t: EntityId| mind.belief_entity(BELIEF_OWNER, t) == Some(mind.me);
        let unowned = |t: EntityId| mind.belief_entity(BELIEF_OWNER, t).is_none();
        for thing in mind.carried() {
            let worth = mind.worth(thing);
            if worth <= 0 {
                continue;
            }
            match home {
                Some(home) if mine(thing) => {
                    return Some((thing, Work::Deliver(thing), home, worth));
                }
                _ if unowned(thing) => return Some((thing, Work::Claim(thing), here, worth)),
                _ => {}
            }
        }
        let mut best: Option<(i64, EntityId, EntityId, i64)> = None;
        for (thing, place) in mind.view.read_about(mind.me, BELIEF_PLACE_OF) {
            let Value::Entity(place) = place.value else {
                continue;
            };
            let wanted = (mine(thing) && home.is_some_and(|h| h != place)) || unowned(thing);
            if thing == mind.me
                || place == mind.me
                || !wanted
                || mind.knows_as_place(thing)
                || mind.belief_entity(BELIEF_LEADS_TO, thing).is_some()
                || mind.gave_up_on(thing)
            {
                continue;
            }
            let worth = mind.worth(thing);
            if worth <= 0 {
                continue;
            }
            let Some(route) = mind.route(here, place) else {
                continue;
            };
            let score = worth - self.rules.hop_cost * route.len() as i64;
            // Best first; among equals, the lower id.
            if best.map_or(true, |b| score > b.0 || (score == b.0 && thing < b.1)) {
                best = Some((score, thing, place, worth));
            }
        }
        best.map(|(_, thing, place, worth)| (thing, Work::Get(thing), place, worth))
    }

    /// Carry the mind's want one step further (Amendment A-19). The want is for one thing, the
    /// goal; once what it wants next concerns another thing, this one is done.
    fn want(
        &self,
        mind: &Mind,
        acts: &mut Acts,
        here: EntityId,
        goal: EntityId,
        tick: u64,
        speed: i64,
    ) {
        match self.want_next(mind, here) {
            Some((thing, next, place, _)) if thing == goal => {
                self.carry_out(mind, acts, here, next, place, tick, speed);
            }
            _ => Self::settle(acts, "have_it"),
        }
    }

    /// Carry out one step of work or want (Amendments A-18, A-19): get a thing; carry one to where
    /// it goes and put it down there, one at a time; make a recipe at its workplace; or claim what
    /// is carried.
    #[allow(clippy::too_many_arguments)]
    fn carry_out(
        &self,
        mind: &Mind,
        acts: &mut Acts,
        here: EntityId,
        next: Work,
        place: EntityId,
        tick: u64,
        speed: i64,
    ) {
        let step = mind.own(STEP);
        let code = step.as_ref().and_then(|s| s.value.as_int());
        let settled = step
            .as_ref()
            .map_or(true, |s| tick >= s.provenance.tick + SETTLE_TICKS);
        let felt = step
            .as_ref()
            .map_or(true, |s| tick >= s.provenance.tick + FEEL_TICKS);
        let same = |thing: EntityId| mind.own_entity(OBJECT) == Some(thing);
        match next {
            // Gathering, an extra one does no harm: reach again once the last reach has told.
            Work::Get(thing) => self.get(mind, acts, here, thing, tick, speed, SETTLE_TICKS),
            Work::Deliver(_) | Work::Make(_) if place != here => {
                self.travel_toward(mind, acts, here, place, tick, speed);
            }
            Work::Deliver(thing) => {
                if code != Some(STEP_DROP) || !same(thing) || settled {
                    acts.set(STEP, Value::Int(STEP_DROP), "putting_down");
                    acts.set(OBJECT, Value::Entity(thing), "putting_down");
                    acts.set(ACT_DROP, Value::Entity(thing), "putting_down");
                }
            }
            Work::Make(recipe) => {
                if code != Some(STEP_MAKE) || felt {
                    acts.set(STEP, Value::Int(STEP_MAKE), "making");
                    acts.set(OBJECT, Value::Entity(recipe), "making");
                    acts.set(ACT_MAKE, Value::Entity(recipe), "making");
                }
            }
            Work::Claim(thing) => {
                if code != Some(STEP_CLAIM) || !same(thing) || felt {
                    acts.set(STEP, Value::Int(STEP_CLAIM), "claiming");
                    acts.set(OBJECT, Value::Entity(thing), "claiming");
                    acts.set(ACT_CLAIM, Value::Entity(thing), "claiming");
                }
            }
        }
    }

    /// Carry the plan to reach `goal` — a place, or a thing in a place — one step further, once
    /// the last step has had time to tell.
    fn travel_toward(
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
            Some(
                STEP_FETCH | STEP_TAKE | STEP_CONSUME | STEP_PICK | STEP_DROP | STEP_MAKE
                | STEP_CLAIM,
            ) => Self::go(acts, goal, speed, "resume"),
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
            // A directed mind stands aside (Amendment A-14).
            if view.read(FactKey::new(me, DIRECTED)).map(|f| f.value) == Some(Value::Bool(true)) {
                continue;
            }
            // Staggered: each mind thinks on its own beat of the world's cadence.
            if (ctx.tick() + me.raw()) % self.think_ticks.max(1) == 0 {
                self.think(view, ctx, me, &mut out);
            }
        }
        out
    }
}
