//! Physical Reality's side of the seam (Amendment A-24): a person crossing from the city tier
//! into the scene is given a body where they stand.
//!
//! Society marks the crossing ([`CROSSING`], read by its published id); the tick after, this
//! system places the person in the place they cross into — at its origin, where the city tier
//! keeps everyone in a place — with a body of the world's size for its residents, able to move.
//! Every other domain gives them its own facts in the same tick.

use crate::schema::{BODY_SIZE, CONTAINED_IN, MOBILE, POSITION};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

/// A resident crossing into the scene, and the place they cross into — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");

const SEAM_READS: &[FactType] = &[CROSSING];
const SEAM_WRITES: &[FactType] = &[CONTAINED_IN, POSITION, BODY_SIZE, MOBILE];

/// Gives each person crossing into the scene a body in their place (Amendment A-24).
pub struct Seam {
    body: [i64; 3],
}

impl Seam {
    /// Bodies of `body` — half-width, half-depth, height, in centimetres.
    pub const fn new(body: [i64; 3]) -> Self {
        Self { body }
    }
}

impl System for Seam {
    fn id(&self) -> SystemId {
        SystemId::new("physical.seam")
    }
    fn reads(&self) -> &'static [FactType] {
        SEAM_READS
    }
    fn writes(&self) -> &'static [FactType] {
        SEAM_WRITES
    }
    fn cadence(&self) -> Cadence {
        Cadence::EveryTick
    }
    fn evaluate(&self, view: &dyn CommittedView, ctx: &TickContext) -> Vec<Proposal> {
        let mut out = Vec::new();
        for who in view.entities_with(CROSSING) {
            let Some(Value::Entity(place)) =
                view.read(FactKey::new(who, CROSSING)).map(|f| f.value)
            else {
                continue;
            };
            for (fact, value) in [
                (CONTAINED_IN, Value::Entity(place)),
                (POSITION, Value::Vec3([0, 0, 0])),
                (BODY_SIZE, Value::Vec3(self.body)),
                (MOBILE, Value::Bool(true)),
            ] {
                out.push(Proposal::new(
                    self.id(),
                    FactKey::new(who, fact),
                    ctx.basis_tick(),
                    Change::Set(value),
                    Cause::new("crossed"),
                ));
            }
        }
        out
    }
}
