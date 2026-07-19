//! Hermetic transformations owned by the Living Systems domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Needs are measurements,
//! never behaviors (Vol. III Ch. 2): thermoregulation adjusts a vital fact, it decides
//! nothing.

use crate::schema::{AMBIENT_TEMPERATURE, BODY_HEAT, CONTAINED_IN};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

/// Reads: the organism's containment (to learn its region) and that region's temperature —
/// both owned by Physical Reality — plus the organism's own body heat. Writes: body heat.
const THERMO_READS: &[FactType] = &[CONTAINED_IN, AMBIENT_TEMPERATURE, BODY_HEAT];
const THERMO_WRITES: &[FactType] = &[BODY_HEAT];

/// Homeostasis for one organism: body heat is defended toward a metabolic set point while
/// the ambient environment pulls it toward the temperature of whatever region the organism
/// is in (Vol. III Ch. 2, the "warmth" need tracks the environment).
///
/// The canonical cross-domain consumer: it reads two Physical Reality facts — where the
/// organism is (containment) and how cold it is there (temperature) — and proposes a change
/// only to its own Living Systems fact (body heat), touching nothing Physical owns
/// (Vol. III Ch. 12 §12.1).
pub struct Thermoregulation {
    organism: EntityId,
    set_point_centi_c: i64,
    warm_response: i64,
    cold_response: i64,
}

impl Thermoregulation {
    /// Configure homeostasis for `organism`. Its region is discovered by reading committed
    /// containment, not passed in. `set_point_centi_c` is the metabolic target;
    /// `warm_response`/`cold_response` are divisors — larger means slower pull toward the set
    /// point / toward ambient (world-package rules, Vol. IV Ch. 2).
    pub const fn new(
        organism: EntityId,
        set_point_centi_c: i64,
        warm_response: i64,
        cold_response: i64,
    ) -> Self {
        Self {
            organism,
            set_point_centi_c,
            warm_response,
            cold_response,
        }
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
        Cadence::EveryTick
    }

    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        // 1. Where am I? Read my containment (a Physical fact) to find my region.
        let region = match view.read(FactKey::new(self.organism, CONTAINED_IN)) {
            Some(fact) => match fact.value {
                Value::Entity(r) => r,
                _ => return Vec::new(),
            },
            None => return Vec::new(),
        };
        // 2. How cold is it here? Read that region's ambient temperature (a Physical fact).
        let ambient = match view.read(FactKey::new(region, AMBIENT_TEMPERATURE)) {
            Some(fact) => match fact.value.as_int() {
                Some(t) => t,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };
        // 3. My own current body heat (a Living fact).
        let current = match view.read(FactKey::new(self.organism, BODY_HEAT)) {
            Some(fact) => match fact.value.as_int() {
                Some(h) => h,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let warm = self.warm_response.max(1);
        let cold = self.cold_response.max(1);
        let delta = (self.set_point_centi_c - current) / warm + (ambient - current) / cold;

        vec![Proposal::new(
            self.id(),
            FactKey::new(self.organism, BODY_HEAT),
            ctx.tick(),
            Change::Delta(delta),
            Cause::new("thermoregulation"),
        )]
    }
}
