//! Hermetic transformations owned by the Living Systems domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Needs are measurements,
//! never behaviors (Vol. III Ch. 2): thermoregulation adjusts a vital fact, it decides
//! nothing.

use crate::schema::{AMBIENT_TEMPERATURE, BODY_HEAT, CONTAINED_IN};
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
const THERMO_READS: &[FactType] = &[CONTAINED_IN, AMBIENT_TEMPERATURE, BODY_HEAT];
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
