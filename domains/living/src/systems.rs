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
pub struct Thermoregulation {
    set_point_centi_c: i64,
    warm_response: i64,
    cold_response: i64,
}

impl Thermoregulation {
    /// Configure homeostasis shared by every organism. `set_point_centi_c` is the metabolic
    /// target; `warm_response`/`cold_response` are divisors — larger means slower pull toward
    /// the set point / toward ambient (world-package rules, Vol. IV Ch. 2).
    pub const fn new(set_point_centi_c: i64, warm_response: i64, cold_response: i64) -> Self {
        Self {
            set_point_centi_c,
            warm_response,
            cold_response,
        }
    }

    /// The body-heat delta for one organism, given its ambient region temperature and current
    /// heat — or `None` if it is not placed in a region with a committed temperature.
    fn delta_for(&self, view: &dyn CommittedView, organism: EntityId) -> Option<i64> {
        // 1. Where am I? Read my containment (a Physical fact) to find my region.
        let region = match view.read(FactKey::new(organism, CONTAINED_IN))?.value {
            Value::Entity(r) => r,
            _ => return None,
        };
        // 2. How cold is it here? Read that region's ambient temperature (a Physical fact).
        let ambient = view
            .read(FactKey::new(region, AMBIENT_TEMPERATURE))?
            .value
            .as_int()?;
        // 3. My own current body heat (a Living fact).
        let current = view
            .read(FactKey::new(organism, BODY_HEAT))?
            .value
            .as_int()?;

        let warm = self.warm_response.max(1);
        let cold = self.cold_response.max(1);
        Some((self.set_point_centi_c - current) / warm + (ambient - current) / cold)
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
        // The organism roster: every entity bearing body heat (Vol. V Ch. 2 §2.1, clause 5).
        view.entities_with(BODY_HEAT)
            .into_iter()
            .filter_map(|organism| {
                self.delta_for(view, organism).map(|delta| {
                    Proposal::new(
                        self.id(),
                        FactKey::new(organism, BODY_HEAT),
                        ctx.basis_tick(),
                        Change::Delta(delta),
                        Cause::new("thermoregulation"),
                    )
                })
            })
            .collect()
    }
}
