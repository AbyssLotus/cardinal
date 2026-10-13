//! Wear (Amendment A-46): a workplace wears while something is in the making at it.
//!
//! **What Wear does.** Every tick, for each maker with a recipe in the making (Economy's
//! [`ECON_MAKING`], read by its published id), the workplace the recipe is made at
//! ([`ECON_RECIPE_AT`]) loses integrity at the rate the world declares for it
//! ([`WEAR_PER_HOUR`]), over the tick, dithered so nothing is lost to rounding over many ticks —
//! the same wear at any tick length (Law 14). Two makers at one workplace wear it twice as fast.
//! A workplace that declares no rate does not wear; one already broken wears no further. What
//! is worn through breaks as anything does that reaches nothing (Amendment A-27), and makes
//! nothing until it is mended (Amendment A-31).
//!
//! Tools wear too, but by the go, not the hour: that is [`crate::act::Act`]'s, with the go.

use crate::schema::{BROKEN, ECON_MAKING, ECON_RECIPE_AT, INTEGRITY, WEAR_PER_HOUR};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::fixed::div_dither;
use kernel::identity::EntityId;
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::time::Step;
use kernel::value::Value;

const WEAR_READS: &[FactType] = &[ECON_MAKING, ECON_RECIPE_AT, WEAR_PER_HOUR, BROKEN];
const WEAR_WRITES: &[FactType] = &[INTEGRITY];

/// Wears workplaces in use (Amendment A-46).
pub struct Wear {
    step: Step,
}

impl Wear {
    /// Wears every `step`.
    pub const fn new(step: Step) -> Self {
        Self { step }
    }
}

fn entity(view: &dyn CommittedView, key: FactKey) -> Option<EntityId> {
    match view.read(key)?.value {
        Value::Entity(e) => Some(e),
        _ => None,
    }
}

impl System for Wear {
    fn id(&self) -> SystemId {
        SystemId::new("physical.wear")
    }
    fn reads(&self) -> &'static [FactType] {
        WEAR_READS
    }
    fn writes(&self) -> &'static [FactType] {
        WEAR_WRITES
    }
    fn cadence(&self) -> Cadence {
        self.step.cadence()
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        // Ascending id of maker: each one's making wears its workplace once.
        for maker in view.entities_with(ECON_MAKING) {
            let Some(recipe) = entity(view, FactKey::new(maker, ECON_MAKING)) else {
                continue;
            };
            let Some(workplace) = entity(view, FactKey::new(recipe, ECON_RECIPE_AT)) else {
                continue;
            };
            let per_hour = view
                .read(FactKey::new(workplace, WEAR_PER_HOUR))
                .and_then(|f| f.value.as_int())
                .unwrap_or(0);
            let broken = view.read(FactKey::new(workplace, BROKEN)).map(|f| f.value)
                == Some(Value::Bool(true));
            if per_hour <= 0 || broken {
                continue;
            }
            let mut rng = ctx.rng(maker.raw());
            let worn = div_dither(
                per_hour as i128 * self.step.dt_ms as i128,
                3_600_000,
                &mut rng,
            ) as i64;
            if worn > 0 {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(workplace, INTEGRITY),
                    ctx.basis_tick(),
                    Change::Delta(-worn),
                    Cause::new("worn_in_use"),
                ));
            }
        }
        out
    }
}
