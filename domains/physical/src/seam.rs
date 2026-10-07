//! Physical Reality's side of the seam (Amendment A-24): a person crossing from the city tier
//! into the scene is given a body where they stand.
//!
//! Society marks the crossing ([`CROSSING`], read by its published id); the tick after, this
//! system places the person in the place they cross into — at its origin, where the city tier
//! keeps everyone in a place — with a body of the world's size for its residents, able to move.
//! Every other domain gives them its own facts in the same tick.
//!
//! A person leaving the scene ([`LEAVING`], Amendment A-25) is retired from Physical Reality's
//! keeping the tick after: their body, place, motion, intents, and what they could see, all at
//! once, whatever else was about to move them.

use crate::schema::{BODY_SIZE, CONTAINED_IN, MOBILE, POSITION};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

/// A resident crossing into the scene, and the place they cross into — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");
/// A resident leaving the scene — **Society's**.
pub const LEAVING: FactType = FactType::new("society.resident.leaving");
/// A household's larder coming into the scene as its pantry, naming its home — **Society's**
/// (Amendment A-26).
pub const PANTRY_CROSSING: FactType = FactType::new("society.pantry.crossing");
/// A household's pantry leaving the scene — **Society's**.
pub const PANTRY_LEAVING: FactType = FactType::new("society.pantry.leaving");

/// Where in its home a pantry stands: a metre and a half from the middle, where those at home
/// stand, so one walks up to it.
const PANTRY_AT: [i64; 3] = [150, 150, 0];

const SEAM_READS: &[FactType] = &[CROSSING, LEAVING, PANTRY_CROSSING, PANTRY_LEAVING];
const SEAM_WRITES: &[FactType] = &[CONTAINED_IN, POSITION, BODY_SIZE, MOBILE];

/// Gives each person crossing into the scene a body in their place (Amendment A-24), and lets go
/// of each person leaving it (Amendment A-25); and the same for households' pantries (Amendment
/// A-26), which stand in their homes.
pub struct Seam {
    body: [i64; 3],
    pantry: Option<[i64; 3]>,
}

impl Seam {
    /// Bodies of `body`, and pantries of `pantry` if the world has them — half-width, half-depth,
    /// height, in centimetres.
    pub const fn new(body: [i64; 3], pantry: Option<[i64; 3]>) -> Self {
        Self { body, pantry }
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
        let mut out: Vec<Proposal> = view
            .entities_with(LEAVING)
            .into_iter()
            .chain(view.entities_with(PANTRY_LEAVING))
            .map(|who| {
                Proposal::new(
                    self.id(),
                    FactKey::new(who, CONTAINED_IN),
                    ctx.basis_tick(),
                    Change::Retire,
                    Cause::new("left_the_scene"),
                )
            })
            .collect();
        if let Some(size) = self.pantry {
            for pantry in view.entities_with(PANTRY_CROSSING) {
                let Some(Value::Entity(home)) = view
                    .read(FactKey::new(pantry, PANTRY_CROSSING))
                    .map(|f| f.value)
                else {
                    continue;
                };
                for (fact, value) in [
                    (CONTAINED_IN, Value::Entity(home)),
                    (POSITION, Value::Vec3(PANTRY_AT)),
                    (BODY_SIZE, Value::Vec3(size)),
                ] {
                    out.push(Proposal::new(
                        self.id(),
                        FactKey::new(pantry, fact),
                        ctx.basis_tick(),
                        Change::Set(value),
                        Cause::new("crossed"),
                    ));
                }
            }
        }
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
