//! Hermetic transformations owned by the Living Systems domain (Vol. V Ch. 3 §3.1).
//!
//! Each system declares its read/write sets and cadence, reads committed reality, and emits
//! proposals — mutating nothing directly (Vol. V Ch. 3 §3.1-3.2). Needs are measurements,
//! never behaviors (Vol. III Ch. 2): thermoregulation adjusts a vital fact, it does not
//! decide anything.

use crate::schema::{AMBIENT_TEMPERATURE, BODY_HEAT};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};

/// Reads: the organism's own body heat, and its region's ambient temperature (owned by
/// Physical Reality — a cross-domain read). Writes: body heat only.
const THERMO_READS: &[FactType] = &[AMBIENT_TEMPERATURE, BODY_HEAT];
const THERMO_WRITES: &[FactType] = &[BODY_HEAT];

/// Homeostasis for one organism: body heat is defended toward a metabolic set point while
/// the ambient environment pulls it toward the region's temperature (Vol. III Ch. 2, the
/// "warmth" need tracks the environment).
///
/// This is the canonical cross-domain consumer: it reads a Physical Reality fact (the
/// region's temperature) and proposes a change only to its own Living Systems fact (body
/// heat), touching nothing Physical owns (Vol. III Ch. 12 §12.1).
pub struct Thermoregulation {
    organism: EntityId,
    region: EntityId,
    set_point_centi_c: i64,
    warm_response: i64,
    cold_response: i64,
}

impl Thermoregulation {
    /// Configure homeostasis for `organism` living in `region`. `set_point_centi_c` is the
    /// metabolic target; `warm_response`/`cold_response` are divisors — larger means slower
    /// pull toward the set point / toward ambient (world-package rules, Vol. IV Ch. 2).
    pub const fn new(
        organism: EntityId,
        region: EntityId,
        set_point_centi_c: i64,
        warm_response: i64,
        cold_response: i64,
    ) -> Self {
        Self {
            organism,
            region,
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
        // Cross-domain read of committed reality: the region's ambient temperature.
        let ambient = match view.read(FactKey::new(self.region, AMBIENT_TEMPERATURE)) {
            Some(fact) => match fact.value.as_int() {
                Some(t) => t,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };
        // The organism's own current body heat.
        let current = match view.read(FactKey::new(self.organism, BODY_HEAT)) {
            Some(fact) => match fact.value.as_int() {
                Some(h) => h,
                None => return Vec::new(),
            },
            None => return Vec::new(),
        };

        let warm = self.warm_response.max(1);
        let cold = self.cold_response.max(1);
        // Metabolism defends the set point; the environment pulls toward ambient. The two
        // integer terms telescope to a stable equilibrium between set point and ambient.
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
