//! Hermetic transformations owned by the Living Systems domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Needs are measurements,
//! never behaviors (Vol. III Ch. 2): thermoregulation adjusts a vital fact, it decides
//! nothing.

use crate::schema::{
    AMBIENT_TEMPERATURE, ARISES_FROM_BOND, ARISES_FROM_DEPENDENCE, BODY_HEAT, CONSUMED,
    CONTAINED_IN, DEPENDENCE, DOSE_JUDGED, FALL_HEIGHT, FALL_JUDGED, FATIGUE, FULL, HEALTH, HUNGER,
    IN_VIEW, KIND_ABOVE, KIND_ARISES, KIND_EASE, KIND_HARM, KIND_MET_BY, KIND_RISE, MADE_OF,
    MATERIAL_HABIT, MATERIAL_NUTRITION, MATERIAL_POTENCY, MET_BY_PRESENCE, MOTION_TARGET, NEED,
    NEED_KIND, REST, SIGHT_RANGE, SOCIETY_BOND, TRAVEL_TO,
};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::div_dither;
use kernel::hierarchy::ancestry;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::rng::Rng;
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;

/// Reads: the organism's containment (to learn its region) and that region's temperature —
/// both owned by Physical Reality — plus the organism's own body heat. Writes: body heat.
const THERMO_READS: &[FactType] = &[CONTAINED_IN, AMBIENT_TEMPERATURE, BODY_HEAT, HEALTH];
const THERMO_WRITES: &[FactType] = &[BODY_HEAT];

/// Homeostasis for every organism: body heat is defended toward a metabolic set point while
/// the ambient environment pulls it toward the temperature of whatever region the organism
/// is in (Vol. III Ch. 2, the "warmth" need tracks the environment).
///
/// The canonical cross-domain consumer: for each organism it reads two Physical Reality
/// facts — where the organism is (containment) and how cold it is there (temperature) — and
/// proposes a change only to that organism's Living Systems fact (body heat), touching
/// nothing Physical owns (Vol. III Ch. 12 §12.1). One instance serves the whole world: it
/// discovers the organisms from committed reality — every entity bearing [`BODY_HEAT`] — so
/// an organism born mid-simulation is regulated the tick its body heat commits
/// (Vol. V Ch. 2 §2.1, clause 5).
///
/// **The rule is a pair of time constants** (Amendment A-1): body heat `H` obeys
/// `dH/dt = (S − H)/τ_warm + (A − H)/τ_cold`, set point `S`, ambient `A`. Each step solves it
/// implicitly over the step's simulated duration `dt`:
///
/// ```text
/// H' = (H·τw·τc + dt·τc·S + dt·τw·A) / (τw·τc + dt·τc + dt·τw)
/// ```
///
/// which is stable for any step length and settles at the same equilibrium,
/// `(S·τc + A·τw)/(τc + τw)`, whatever the tick length. The division is rounded without bias
/// ([`kernel::fixed::div_dither`]): at fine tick lengths the per-step change is a fraction of a
/// centidegree, and ordinary rounding would freeze the body short of equilibrium.
pub struct Thermoregulation {
    step: Step,
    set_point_centi_c: i64,
    warm_ms: u64,
    cold_ms: u64,
}

impl Thermoregulation {
    /// Configure homeostasis shared by every organism, stepping as `step` says.
    /// `set_point_centi_c` is the metabolic target; `warm_ms`/`cold_ms` are the time constants
    /// of the pull toward the set point and toward ambient (world-package rules, Vol. IV Ch. 2).
    pub const fn new(step: Step, set_point_centi_c: i64, warm_ms: u64, cold_ms: u64) -> Self {
        Self {
            step,
            set_point_centi_c,
            warm_ms: if warm_ms == 0 { 1 } else { warm_ms },
            cold_ms: if cold_ms == 0 { 1 } else { cold_ms },
        }
    }

    /// The temperature of the air around the organism and its current body heat — or `None` if
    /// nothing enclosing it has a temperature.
    fn state_of(&self, view: &dyn CommittedView, organism: EntityId) -> Option<(i64, i64)> {
        // 1. Where am I, and what air is around me? Walk up my containment (a Physical fact) to
        //    the nearest place with a temperature: the room I stand in, or — riding in a cart, or
        //    carried in a bag — the region the cart or the bag is in. Climate is inherited down
        //    the hierarchy (Amendment A-5), so a body feels the air of whatever encloses it.
        let ambient = ancestry(view, organism, CONTAINED_IN)
            .into_iter()
            .skip(1)
            .find_map(|place| {
                view.read(FactKey::new(place, AMBIENT_TEMPERATURE))
                    .and_then(|f| f.value.as_int())
            })?;
        // 2. My own current body heat (a Living fact).
        let current = view
            .read(FactKey::new(organism, BODY_HEAT))?
            .value
            .as_int()?;
        Some((ambient, current))
    }

    /// Body heat after one step from `current`, given `ambient`, rounded without bias.
    fn settle(&self, ambient: i64, current: i64, rng: &mut Rng) -> i64 {
        let (h, s, a) = (
            current as i128,
            self.set_point_centi_c as i128,
            ambient as i128,
        );
        let (tw, tc, dt) = (
            self.warm_ms as i128,
            self.cold_ms as i128,
            self.step.dt_ms as i128,
        );
        let numerator = h * tw * tc + dt * tc * s + dt * tw * a;
        let denominator = tw * tc + dt * tc + dt * tw;
        div_dither(numerator, denominator, rng) as i64
    }
}

impl System for Thermoregulation {
    fn id(&self) -> SystemId {
        SystemId::new("living.thermoregulation")
    }
    fn reads(&self) -> &'static [FactType] {
        THERMO_READS
    }
    fn writes(&self) -> &'static [FactType] {
        THERMO_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        // The organism roster: every entity bearing body heat (Vol. V Ch. 2 §2.1, clause 5).
        view.entities_with(BODY_HEAT)
            .into_iter()
            .filter(|organism| alive(view, *organism))
            .filter_map(|organism| {
                let (ambient, current) = self.state_of(view, organism)?;
                let next = self.settle(ambient, current, &mut ctx.rng(organism.raw()));
                Some(Proposal::new(
                    self.id(),
                    FactKey::new(organism, BODY_HEAT),
                    ctx.basis_tick(),
                    Change::Delta(next - current),
                    Cause::new("thermoregulation"),
                ))
            })
            .collect()
    }
}

/// Whether `organism` is alive: its health has not reached zero. An organism without a health
/// fact (a world that does not track it) is alive. Death stops all processing here (Vol. III
/// Ch. 2 §2.8, invariant 7) but never removes the organism.
pub fn alive(view: &dyn CommittedView, organism: EntityId) -> bool {
    view.read(FactKey::new(organism, HEALTH))
        .and_then(|f| f.value.as_int())
        != Some(0)
}

fn int(view: &dyn CommittedView, key: FactKey) -> Option<i64> {
    view.read(key).and_then(|f| f.value.as_int())
}

/// `per_hour × dt`, in the same units, rounded without bias so a slow rate at fine ticks still
/// accumulates (Amendment A-1).
fn over(per_hour: i64, dt_ms: u64, rng: &mut Rng) -> i64 {
    div_dither(per_hour as i128 * dt_ms as i128, 3_600_000, rng) as i64
}

const FATIGUE_READS: &[FactType] = &[HEALTH, FATIGUE, REST, MOTION_TARGET, TRAVEL_TO];
const FATIGUE_WRITES: &[FactType] = &[FATIGUE];

/// Tiring and resting (Vol. III Ch. 2 §2.4; Appendix A, Ruling 15; Amendment A-10): every living
/// organism's fatigue rises while it is awake and falls while it rests, at the world's rates. It
/// rests when a decider has asked it to and its body is still — not travelling, not falling.
pub struct Fatigue {
    step: Step,
    tire_per_hour: i64,
    rest_per_hour: i64,
}

impl Fatigue {
    /// Tire at `tire_per_hour` and recover at `rest_per_hour` (hundredths of a percent), stepping
    /// as `step` says.
    pub const fn new(step: Step, tire_per_hour: i64, rest_per_hour: i64) -> Self {
        Self {
            step,
            tire_per_hour,
            rest_per_hour,
        }
    }
}

impl System for Fatigue {
    fn id(&self) -> SystemId {
        SystemId::new("living.fatigue")
    }
    fn reads(&self) -> &'static [FactType] {
        FATIGUE_READS
    }
    fn writes(&self) -> &'static [FactType] {
        FATIGUE_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        view.entities_with(FATIGUE)
            .into_iter()
            .filter(|o| alive(view, *o))
            .filter_map(|organism| {
                let now = int(view, FactKey::new(organism, FATIGUE))?;
                let still = view.read(FactKey::new(organism, MOTION_TARGET)).is_none()
                    && view.read(FactKey::new(organism, TRAVEL_TO)).is_none();
                let asked = view.read(FactKey::new(organism, REST)).map(|f| f.value)
                    == Some(kernel::value::Value::Bool(true));
                let resting = asked && still;
                let rate = if resting {
                    -self.rest_per_hour
                } else {
                    self.tire_per_hour
                };
                let next = (now + over(rate, self.step.dt_ms, &mut ctx.rng(organism.raw())))
                    .clamp(0, FULL);
                (next != now).then(|| {
                    Proposal::new(
                        self.id(),
                        FactKey::new(organism, FATIGUE),
                        ctx.basis_tick(),
                        Change::Set(kernel::value::Value::Int(next)),
                        Cause::new(if resting { "rested" } else { "awake" }),
                    )
                })
            })
            .collect()
    }
}

/// What harms an organism and how it heals — the world's rules (Amendment A-10).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HarmRules {
    /// Body heat, in centidegrees, below which the cold harms.
    pub hypothermia_below_centi_c: i64,
    /// Health lost per degree below that line, per hour.
    pub cold_harm_per_degree_hour: i64,
    /// The highest fall, in centimetres, that does no harm.
    pub safe_fall_cm: i64,
    /// Health lost per metre fallen beyond the safe drop.
    pub fall_harm_per_metre: i64,
    /// Health recovered per hour while nothing harms the organism.
    pub heal_per_hour: i64,
    /// Hunger past which the organism is starving (Amendment A-16).
    pub starving_above: i64,
    /// Health lost per hour while starving.
    pub starving_harm_per_hour: i64,
}

const HEALTH_READS: &[FactType] = &[
    HUNGER,
    BODY_HEAT,
    HEALTH,
    FALL_HEIGHT,
    FALL_JUDGED,
    NEED,
    NEED_KIND,
    KIND_HARM,
];
// At death the organism stops: its senses go, and its body no longer walks where it was going.
const HEALTH_WRITES: &[FactType] = &[HEALTH, FALL_JUDGED, SIGHT_RANGE, TRAVEL_TO];

/// Harm and healing (Vol. III Ch. 2 §2.4; Appendix A, Rulings 9 and 15; Amendment A-10). Every
/// living organism's health falls while its body is colder than the world's line, and once for
/// each fall Physical Reality reports beyond the safe drop; it heals slowly while nothing harms it.
/// Health reaching zero is death, chronicled with its cause; with it the organism stops — its
/// sight goes, and the travel it had asked for is cancelled — though its body and history remain
/// (Vol. III Ch. 2 §2.8, invariant 7).
pub struct Health {
    step: Step,
    rules: HarmRules,
}

impl Health {
    /// Judge harm and healing by `rules`, stepping as `step` says.
    pub const fn new(step: Step, rules: HarmRules) -> Self {
        Self { step, rules }
    }
}

impl System for Health {
    fn id(&self) -> SystemId {
        SystemId::new("living.health")
    }
    fn reads(&self) -> &'static [FactType] {
        HEALTH_READS
    }
    fn writes(&self) -> &'static [FactType] {
        HEALTH_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let r = &self.rules;
        let mut out = Vec::new();
        for organism in view.entities_with(HEALTH) {
            let Some(health) = int(view, FactKey::new(organism, HEALTH)).filter(|h| *h > 0) else {
                continue;
            };
            let mut rng = ctx.rng(organism.raw());
            // The cold: degrees below the line, for as long as the step lasts.
            let below = int(view, FactKey::new(organism, BODY_HEAT))
                .map_or(0, |b| (r.hypothermia_below_centi_c - b).max(0));
            let cold = over(
                below * r.cold_harm_per_degree_hour / 100,
                self.step.dt_ms,
                &mut rng,
            );
            // A fall not yet judged: metres beyond the safe drop.
            let mut fall = 0;
            if let Some(f) = view.read(FactKey::new(organism, FALL_HEIGHT)) {
                let judged = int(view, FactKey::new(organism, FALL_JUDGED)).unwrap_or(0);
                if f.provenance.tick as i64 > judged {
                    let beyond = (f.value.as_int().unwrap_or(0) - r.safe_fall_cm).max(0);
                    fall = beyond * r.fall_harm_per_metre / 100;
                    out.push(Proposal::new(
                        self.id(),
                        FactKey::new(organism, FALL_JUDGED),
                        ctx.basis_tick(),
                        Change::Set(kernel::value::Value::Int(f.provenance.tick as i64)),
                        Cause::new("fall_judged"),
                    ));
                }
            }
            // Needs felt strongly enough to hurt, as their kinds declare (Amendment A-11).
            let mut aching = 0;
            for (object, level) in view.read_about(organism, NEED) {
                let Some(kernel::value::Value::Entity(kind)) = view
                    .read(FactKey::pair(organism, NEED_KIND, object))
                    .map(|f| f.value)
                else {
                    continue;
                };
                let harm = int(view, FactKey::new(kind, KIND_HARM)).unwrap_or(0);
                let felt = level.value.as_int().unwrap_or(0);
                aching += over(harm * felt / FULL, self.step.dt_ms, &mut rng);
            }
            // Starving (Amendment A-16).
            let hungry = int(view, FactKey::new(organism, HUNGER)).unwrap_or(0);
            let starving = if hungry > r.starving_above {
                over(r.starving_harm_per_hour, self.step.dt_ms, &mut rng)
            } else {
                0
            };
            let heal = if cold == 0 && fall == 0 && aching == 0 && starving == 0 {
                over(r.heal_per_hour, self.step.dt_ms, &mut rng)
            } else {
                0
            };
            let next = (health - cold - fall - aching - starving + heal).clamp(0, FULL);
            if next == health {
                continue;
            }
            // The cause: the worst of what harmed it, or healing.
            let worst = fall.max(cold).max(aching).max(starving);
            let why = match (next, worst) {
                (_, 0) => "healing",
                (0, w) if w == fall => "died_of_a_fall",
                (0, w) if w == cold => "died_of_cold",
                (0, w) if w == starving => "died_of_hunger",
                (0, _) => "died_of_need",
                (_, w) if w == fall => "fall",
                (_, w) if w == cold => "cold",
                (_, w) if w == starving => "starving",
                _ => "need",
            };
            out.push(Proposal::new(
                self.id(),
                FactKey::new(organism, HEALTH),
                ctx.basis_tick(),
                Change::Set(kernel::value::Value::Int(next)),
                Cause::new(why),
            ));
            if next == 0 {
                for fact in [SIGHT_RANGE, TRAVEL_TO] {
                    out.push(Proposal::new(
                        self.id(),
                        FactKey::new(organism, fact),
                        ctx.basis_tick(),
                        Change::Tombstone,
                        Cause::new(why),
                    ));
                }
            }
        }
        out
    }
}

const NEEDS_READS: &[FactType] = &[
    HUNGER,
    MATERIAL_NUTRITION,
    BODY_HEAT,
    HEALTH,
    NEED,
    NEED_KIND,
    KIND_ARISES,
    KIND_ABOVE,
    KIND_MET_BY,
    KIND_RISE,
    KIND_EASE,
    SOCIETY_BOND,
    CONTAINED_IN,
    IN_VIEW,
    DEPENDENCE,
    DOSE_JUDGED,
    CONSUMED,
    MADE_OF,
    MATERIAL_POTENCY,
    MATERIAL_HABIT,
];
const NEEDS_WRITES: &[FactType] = &[NEED, NEED_KIND, DEPENDENCE, DOSE_JUDGED, HUNGER];

/// Needs that arise, and the dependence some of them arise from (Vol. III Ch. 2 §2.4;
/// Amendments A-11, A-13). For every living organism, each step:
///
/// - **doses** — a consumption Physical Reality reported and not yet judged builds dependence on
///   each habit-forming material in it (by the material's habit), and marks those materials
///   dosed;
/// - **dependence fades** at the world's daily rate for every substance not dosed;
/// - **arising** — a need of each declared kind comes into being for each thing its origin points
///   at: a kind that arises from a bond, for each person the organism is bonded to (Society's);
///   a kind that arises from dependence, for each substance depended on past the kind's line;
/// - **ending** — a need whose origin is gone ends;
/// - **growing and easing** — a need met by presence eases while its object is in the same place
///   or in view, and grows while it is not; a need met by a dose grows at its rate scaled by the
///   dependence beneath it, and each dose eases it by the kind's measure times the potency.
pub struct Needs {
    step: Step,
    dependence_fade_per_day: i64,
    hunger_per_hour: i64,
}

impl Needs {
    /// Needs stepping as `step` says; dependence fading at `dependence_fade_per_day`; hunger
    /// rising at `hunger_per_hour` (Amendment A-16).
    pub const fn new(step: Step, dependence_fade_per_day: i64, hunger_per_hour: i64) -> Self {
        Self {
            step,
            dependence_fade_per_day,
            hunger_per_hour,
        }
    }
}

impl System for Needs {
    fn id(&self) -> SystemId {
        SystemId::new("living.needs")
    }
    fn reads(&self) -> &'static [FactType] {
        NEEDS_READS
    }
    fn writes(&self) -> &'static [FactType] {
        NEEDS_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        use kernel::value::Value;
        use std::collections::BTreeMap;
        let of_kind = |kind: EntityId, fact| int(view, FactKey::new(kind, fact)).unwrap_or(0);
        let kinds = view.entities_with(KIND_ARISES);
        let first = |arises: i64| {
            kinds
                .iter()
                .copied()
                .find(|k| of_kind(*k, KIND_ARISES) == arises)
        };
        let (from_bond, from_dependence) = (first(ARISES_FROM_BOND), first(ARISES_FROM_DEPENDENCE));
        let mut out = Vec::new();
        for organism in view.entities_with(BODY_HEAT) {
            if !alive(view, organism) {
                continue;
            }
            let mut push = |key: FactKey, change, why| {
                out.push(Proposal::new(
                    self.id(),
                    key,
                    ctx.basis_tick(),
                    change,
                    Cause::new(why),
                ));
            };
            let mut rng = ctx.rng(organism.raw());

            // Doses: a consumption not yet judged.
            let mut dosed: BTreeMap<EntityId, i64> = BTreeMap::new(); // material -> potency
            let mut fed = 0; // nutrition eaten
            let mut built: BTreeMap<EntityId, i64> = BTreeMap::new(); // material -> habit
            if let Some(c) = view.read(FactKey::new(organism, CONSUMED)) {
                let judged = int(view, FactKey::new(organism, DOSE_JUDGED)).unwrap_or(0);
                if let (Value::Entity(thing), true) = (c.value, c.provenance.tick as i64 > judged) {
                    for m in view.read_all(FactKey::new(thing, MADE_OF)) {
                        let Value::Entity(m) = m.value else { continue };
                        dosed.insert(m, int(view, FactKey::new(m, MATERIAL_POTENCY)).unwrap_or(0));
                        fed += int(view, FactKey::new(m, MATERIAL_NUTRITION)).unwrap_or(0);
                        let habit = int(view, FactKey::new(m, MATERIAL_HABIT)).unwrap_or(0);
                        if habit > 0 {
                            built.insert(m, habit);
                        }
                    }
                    let key = FactKey::new(organism, DOSE_JUDGED);
                    push(
                        key,
                        Change::Set(Value::Int(c.provenance.tick as i64)),
                        "dose_judged",
                    );
                }
            }

            // Hunger: rising by the hour, lowered by what was eaten (Amendment A-16).
            if let Some(hungry) = int(view, FactKey::new(organism, HUNGER)) {
                let next = (hungry + over(self.hunger_per_hour, self.step.dt_ms, &mut rng) - fed)
                    .clamp(0, FULL);
                if next != hungry {
                    let why = if fed > 0 { "ate" } else { "hungrier" };
                    push(
                        FactKey::new(organism, HUNGER),
                        Change::Set(Value::Int(next)),
                        why,
                    );
                }
            }

            // Dependence: built by doses, fading without them.
            let mut dependence: BTreeMap<EntityId, i64> = view
                .read_about(organism, DEPENDENCE)
                .into_iter()
                .map(|(m, f)| (m, f.value.as_int().unwrap_or(0)))
                .collect();
            for m in built.keys() {
                dependence.entry(*m).or_insert(0);
            }
            let fade = kernel::fixed::div_dither(
                self.dependence_fade_per_day as i128 * self.step.dt_ms as i128,
                86_400_000,
                &mut rng,
            ) as i64;
            for (m, level) in dependence.iter_mut() {
                let before = view
                    .read(FactKey::pair(organism, DEPENDENCE, *m))
                    .and_then(|f| f.value.as_int());
                let next = match built.get(m) {
                    Some(habit) => (*level + habit).min(FULL),
                    None => (*level - fade).max(0),
                };
                *level = next;
                let key = FactKey::pair(organism, DEPENDENCE, *m);
                match (before, next) {
                    (Some(b), n) if b == n => {}
                    (_, 0) => push(key, Change::Tombstone, "free_of_it"),
                    (_, n) => {
                        let why = if built.contains_key(m) {
                            "dosed"
                        } else {
                            "abstinent"
                        };
                        push(key, Change::Set(Value::Int(n)), why)
                    }
                }
            }

            // What the organism's needs should be about, and of what kind.
            let mut wanted: BTreeMap<EntityId, EntityId> = BTreeMap::new();
            if let Some(kind) = from_bond {
                for (other, _) in view.read_about(organism, SOCIETY_BOND) {
                    wanted.insert(other, kind);
                }
            }
            if let Some(kind) = from_dependence {
                let line = of_kind(kind, KIND_ABOVE);
                for (m, level) in &dependence {
                    if *level > line {
                        wanted.insert(*m, kind);
                    }
                }
            }
            let held: Vec<(EntityId, i64)> = view
                .read_about(organism, NEED)
                .into_iter()
                .map(|(o, f)| (o, f.value.as_int().unwrap_or(0)))
                .collect();
            for (object, kind) in &wanted {
                if !held.iter().any(|(o, _)| o == object) {
                    let key = FactKey::pair(organism, NEED, *object);
                    push(key, Change::Set(Value::Int(0)), "arose");
                    let key = FactKey::pair(organism, NEED_KIND, *object);
                    push(key, Change::Set(Value::Entity(*kind)), "arose");
                }
            }
            let here = view
                .read(FactKey::new(organism, CONTAINED_IN))
                .map(|f| f.value);
            let in_view: Vec<EntityId> = view
                .read_all(FactKey::new(organism, IN_VIEW))
                .into_iter()
                .filter_map(|f| match f.value {
                    Value::Entity(e) => Some(e),
                    _ => None,
                })
                .collect();
            for (object, level) in held {
                let Some(kind) = wanted.get(&object).copied() else {
                    push(
                        FactKey::pair(organism, NEED, object),
                        Change::Tombstone,
                        "origin_gone",
                    );
                    let key = FactKey::pair(organism, NEED_KIND, object);
                    push(key, Change::Tombstone, "origin_gone");
                    continue;
                };
                let (next, why) = if of_kind(kind, KIND_MET_BY) == MET_BY_PRESENCE {
                    let together = in_view.contains(&object)
                        || (here.is_some()
                            && view
                                .read(FactKey::new(object, CONTAINED_IN))
                                .map(|f| f.value)
                                == here);
                    let rate = if together {
                        -of_kind(kind, KIND_EASE)
                    } else {
                        of_kind(kind, KIND_RISE)
                    };
                    (
                        level + over(rate, self.step.dt_ms, &mut rng),
                        if together { "together" } else { "apart" },
                    )
                } else {
                    // Met by a dose: grows with the dependence beneath it; a dose eases it.
                    let depth = dependence.get(&object).copied().unwrap_or(0);
                    let grow = over(
                        of_kind(kind, KIND_RISE) * depth / FULL,
                        self.step.dt_ms,
                        &mut rng,
                    );
                    match dosed.get(&object) {
                        Some(potency) => (
                            level - of_kind(kind, KIND_EASE) * potency / FULL + grow,
                            "dosed",
                        ),
                        None => (level + grow, "craving"),
                    }
                };
                let next = next.clamp(0, FULL);
                if next != level {
                    push(
                        FactKey::pair(organism, NEED, object),
                        Change::Set(Value::Int(next)),
                        why,
                    );
                }
            }
        }
        out
    }
}
