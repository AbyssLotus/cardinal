//! Living Systems' side of the seam (Amendment A-24): a person crossing from the city tier into
//! the scene is given the vital state of an organism, carried over from the record they cross
//! from.
//!
//! Hunger and fatigue are levels at both tiers (Amendment A-20), moving at the same rates, so they
//! are copied exactly: the person is as hungry and as tired in the scene, at every later tick, as
//! the record would have had them. One who was asleep is resting. Health is whole and body heat at
//! the world's set point, as the city tier assumes; sight is the world's for its residents.
//!
//! A person leaving the scene (Amendment A-25) is retired from Living Systems' keeping the tick
//! after. Society carries their hunger and fatigue back into the record, exactly; what the city
//! tier does not keep — needs that arose, dependence — ends at the seam.

use crate::schema::{BODY_HEAT, FATIGUE, FULL, HEALTH, HUNGER, REST, SIGHT_RANGE};
use kernel::fact::{Cause, FactKey, FactType, SystemId};
use kernel::proposal::{Change, Proposal};
use kernel::system::{Cadence, CommittedView, System, TickContext};
use kernel::value::Value;

/// A resident crossing into the scene — **Society's**.
pub const CROSSING: FactType = FactType::new("society.resident.crossing");
/// A resident leaving the scene — **Society's**.
pub const LEAVING: FactType = FactType::new("society.resident.leaving");
/// A resident's hunger, a level — **Society's**.
pub const RESIDENT_HUNGER: FactType = FactType::new("society.resident.hunger");
/// A resident's fatigue, a level — **Society's**.
pub const RESIDENT_FATIGUE: FactType = FactType::new("society.resident.fatigue");
/// What a resident is doing — **Society's**.
pub const RESIDENT_DOING: FactType = FactType::new("society.resident.doing");
/// Asleep, in Society's record.
pub const SLEEPING: i64 = 1;

const SEAM_READS: &[FactType] = &[
    CROSSING,
    LEAVING,
    RESIDENT_HUNGER,
    RESIDENT_FATIGUE,
    RESIDENT_DOING,
];
const SEAM_WRITES: &[FactType] = &[HUNGER, FATIGUE, HEALTH, BODY_HEAT, SIGHT_RANGE, REST];

/// Gives each person crossing into the scene an organism's vital state (Amendment A-24).
pub struct Seam {
    set_point_centi_c: i64,
    sight_cm: i64,
}

impl Seam {
    /// Body heat at `set_point_centi_c`, and sight to `sight_cm`.
    pub const fn new(set_point_centi_c: i64, sight_cm: i64) -> Self {
        Self {
            set_point_centi_c,
            sight_cm,
        }
    }
}

impl System for Seam {
    fn id(&self) -> SystemId {
        SystemId::new("living.seam")
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
            .map(|who| {
                Proposal::new(
                    self.id(),
                    FactKey::new(who, HUNGER),
                    ctx.basis_tick(),
                    Change::Retire,
                    Cause::new("left_the_scene"),
                )
            })
            .collect();
        for who in view.entities_with(CROSSING) {
            let read = |fact| view.read(FactKey::new(who, fact)).map(|f| f.value);
            let mut facts = vec![
                (HEALTH, Value::Int(FULL)),
                (BODY_HEAT, Value::Int(self.set_point_centi_c)),
                (SIGHT_RANGE, Value::Int(self.sight_cm)),
            ];
            // The levels as they stand, rate and anchor and all.
            for (record, mine) in [(RESIDENT_HUNGER, HUNGER), (RESIDENT_FATIGUE, FATIGUE)] {
                if let Some(level) = read(record) {
                    facts.push((mine, level));
                }
            }
            if read(RESIDENT_DOING) == Some(Value::Int(SLEEPING)) {
                facts.push((REST, Value::Bool(true)));
            }
            for (fact, value) in facts {
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
