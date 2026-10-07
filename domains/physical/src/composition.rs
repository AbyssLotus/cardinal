//! Proposal-composition for Physical Reality-owned fact types (Vol. IV Ch. 2).
//!
//! Composition rules decide how competing proposals against an owned fact reconcile before
//! commit (Vol. V Ch. 3 §3.1, Resolve). Scalar fields sum their deltas; percentage fields
//! sum then clamp to their range; containment is set-valued. Every tunable threshold lives
//! in the world package, never here (Vol. IV Ch. 2 §2.2).

use kernel::domain::{ResolveError, Resolved};
use kernel::proposal::Change;
use kernel::value::Value;

/// Sum deltas onto the current value: at most one `Set`/`Create` establishes the base (two
/// competing sets are an undeclared conflict and fail — Vol. V Ch. 3 §3.5, invariant 4),
/// then all `Delta`s sum onto it. A `Tombstone` may not share the tick with a set or delta.
/// The base for a fact with no committed value is zero. Used by temperature and elevation.
pub fn compose_additive(
    current: Option<Value>,
    changes: &[Change],
) -> Result<Resolved, ResolveError> {
    let mut base: Option<i64> = current.and_then(|v| v.as_int());
    let mut set_seen = false;
    let mut delta_sum: i64 = 0;
    let mut tombstone = false;

    for change in changes {
        match change {
            Change::Set(v) | Change::Create(v) => {
                let n = v
                    .as_int()
                    .ok_or(ResolveError::new("this fact requires an integer value"))?;
                if set_seen {
                    return Err(ResolveError::new(
                        "two competing sets with no declared tie-break",
                    ));
                }
                set_seen = true;
                base = Some(n);
            }
            Change::Delta(d) => delta_sum = delta_sum.saturating_add(*d),
            Change::Tombstone => tombstone = true,
            Change::Add(_) | Change::Remove(_) => {
                return Err(ResolveError::new(
                    "this fact is single-valued; use Set/Create/Delta, not Add/Remove",
                ))
            }
        }
    }

    if tombstone {
        if set_seen || delta_sum != 0 {
            return Err(ResolveError::new(
                "tombstone conflicts with a set or delta on the same tick",
            ));
        }
        return Ok(Resolved::Tombstone);
    }

    let start = base.unwrap_or(0);
    Ok(Resolved::Write(Value::Int(start.saturating_add(delta_sum))))
}

/// Sum deltas as [`compose_additive`], then clamp the result into `[min, max]`. Used by the
/// percentage fields (illumination, humidity), so a random-walk of weather deltas can never
/// drive them out of range and abort a tick. Tombstones pass through unclamped.
pub fn compose_bounded(
    current: Option<Value>,
    changes: &[Change],
    min: i64,
    max: i64,
) -> Result<Resolved, ResolveError> {
    match compose_additive(current, changes)? {
        Resolved::Write(Value::Int(n)) => Ok(Resolved::Write(Value::Int(n.clamp(min, max)))),
        other => Ok(other),
    }
}

/// Resolve a single entity-reference fact: a `Set`/`Create` names the referenced entity;
/// two competing references with no declared tie-break fail; a `Tombstone` clears it. Such a
/// fact takes no numeric delta and no Add/Remove. Used by `contained_in` (an entity's
/// container) and `wind_toward` (the downwind region).
pub fn compose_entity_ref(
    current: Option<Value>,
    changes: &[Change],
) -> Result<Resolved, ResolveError> {
    let mut container: Option<Value> = current;
    let mut set_seen = false;
    let mut tombstone = false;

    for change in changes {
        match change {
            Change::Set(v) | Change::Create(v) => {
                if !matches!(v, Value::Entity(_)) {
                    return Err(ResolveError::new("this fact must reference an entity"));
                }
                if set_seen {
                    return Err(ResolveError::new(
                        "two competing entity references with no declared tie-break",
                    ));
                }
                set_seen = true;
                container = Some(*v);
            }
            Change::Delta(_) => {
                return Err(ResolveError::new(
                    "an entity-reference fact cannot take a numeric delta",
                ))
            }
            Change::Add(_) | Change::Remove(_) => {
                return Err(ResolveError::new(
                    "an entity-reference fact is single-valued; use Set/Create, not Add/Remove",
                ))
            }
            Change::Tombstone => tombstone = true,
        }
    }

    if tombstone {
        if set_seen {
            return Err(ResolveError::new(
                "an entity-reference tombstone conflicts with a set on the same tick",
            ));
        }
        return Ok(Resolved::Tombstone);
    }

    container.map(Resolved::Write).ok_or(ResolveError::new(
        "entity-reference fact resolved with no value",
    ))
}

/// Resolve a decider's intent naming an entity — where to travel, what to open or shut
/// (Ruling 13; Amendment A-9). As [`compose_entity_ref`], except that a fresh intent outlives
/// the clearing of the old one: when a decider asks for something new in the very tick Physical
/// Reality clears the intent it has just fulfilled (or considered), the new wish stands. Two
/// deciders asking for different things at once is still a conflict.
pub fn compose_intent(
    current: Option<Value>,
    changes: &[Change],
) -> Result<Resolved, ResolveError> {
    let wishes: Vec<Change> = changes
        .iter()
        .copied()
        .filter(|c| !matches!(c, Change::Tombstone))
        .collect();
    if wishes.is_empty() || wishes.len() == changes.len() {
        return compose_entity_ref(current, changes);
    }
    compose_entity_ref(current, &wishes)
}

/// Resolve a three-component fact — a position, a size, a motion target (Amendment A-3): at
/// most one `Set`/`Create` replaces the whole value (two competing ones fail, as for any
/// single-valued fact), a `Tombstone` clears it, and nothing else applies — a position is moved
/// by setting where it now is, never by nudging one axis. When `bounds` is given, each component
/// is clamped into it (sizes cannot be negative).
pub fn compose_vec3(
    current: Option<Value>,
    changes: &[Change],
    bounds: Option<(i64, i64)>,
) -> Result<Resolved, ResolveError> {
    let mut value = current;
    let mut set_seen = false;
    let mut tombstone = false;
    for change in changes {
        match change {
            Change::Set(v) | Change::Create(v) => {
                let mut v3 = v.as_vec3().ok_or(ResolveError::new(
                    "this fact requires a three-component value",
                ))?;
                if let Some((lo, hi)) = bounds {
                    v3 = v3.map(|c| c.clamp(lo, hi));
                }
                // Two sets that agree are not in competition: a body that arrives on the tick it
                // sets out again is placed where it stands by both the arrival and the departure
                // (Amendment A-3). Only sets that disagree need a tie-break, and have none.
                if set_seen && value != Some(Value::Vec3(v3)) {
                    return Err(ResolveError::new(
                        "two competing sets with no declared tie-break",
                    ));
                }
                set_seen = true;
                value = Some(Value::Vec3(v3));
            }
            Change::Tombstone => tombstone = true,
            Change::Delta(_) | Change::Add(_) | Change::Remove(_) => {
                return Err(ResolveError::new(
                    "a three-component fact is replaced whole: use Set, not Delta or Add/Remove",
                ))
            }
        }
    }
    if tombstone {
        if set_seen {
            return Err(ResolveError::new(
                "tombstone conflicts with a set on the same tick",
            ));
        }
        return Ok(Resolved::Tombstone);
    }
    value.map(Resolved::Write).ok_or(ResolveError::new(
        "three-component fact resolved with no value",
    ))
}

/// Resolve one field of a motion segment — its target, start, or end (Amendment A-3) — under
/// the segment's declared tie-break: **a new segment supersedes the closing of the old one.**
///
/// Closing a segment (tombstoning its fields, as an arrival or a halt does) and opening a new
/// one (setting them, as a departure does) can land on the same tick: a body arrives and is
/// immediately sent on, or is halted and redirected. Both describe the same moment, and the
/// newer intent — to move — is the one reality keeps; without this rule the tick would fail on a
/// conflict nobody meant (Vol. V Ch. 3 §3.5, invariant 4: conflicts need a declared tie-break,
/// and this is it). Sets that agree merge; sets that disagree still conflict — two systems
/// sending one body two ways at once is a genuine contradiction. Deltas and set operations do
/// not apply to segment fields.
pub fn compose_segment_field(
    current: Option<Value>,
    changes: &[Change],
) -> Result<Resolved, ResolveError> {
    let mut set: Option<Value> = None;
    let mut tombstone = false;
    for change in changes {
        match change {
            Change::Set(v) | Change::Create(v) => match set {
                Some(existing) if existing != *v => {
                    return Err(ResolveError::new(
                        "two competing motion segments for one body on one tick",
                    ))
                }
                _ => set = Some(*v),
            },
            Change::Tombstone => tombstone = true,
            Change::Delta(_) | Change::Add(_) | Change::Remove(_) => {
                return Err(ResolveError::new(
                    "a motion segment is set or closed whole, never adjusted",
                ))
            }
        }
    }
    match (set, tombstone) {
        (Some(v), _) => Ok(Resolved::Write(v)),
        (None, true) => Ok(Resolved::Tombstone),
        (None, false) => current
            .map(Resolved::Write)
            .ok_or(ResolveError::new("motion field resolved with no value")),
    }
}

/// Resolve a boolean flag — solid, opaque, enclosed, open, mobile, blocked (Amendment A-4): a
/// `Set`/`Create` names the value, sets that agree merge, sets that disagree conflict (two
/// systems opening and shutting one door on one tick is a contradiction), a `Tombstone` clears
/// it, and nothing else applies.
pub fn compose_bool(current: Option<Value>, changes: &[Change]) -> Result<Resolved, ResolveError> {
    let mut set: Option<bool> = None;
    let mut tombstone = false;
    for change in changes {
        match change {
            Change::Set(Value::Bool(b)) | Change::Create(Value::Bool(b)) => match set {
                Some(existing) if existing != *b => {
                    return Err(ResolveError::new(
                        "a flag set both true and false on one tick",
                    ))
                }
                _ => set = Some(*b),
            },
            Change::Tombstone => tombstone = true,
            _ => {
                return Err(ResolveError::new(
                    "a flag takes only a true/false set or a clear",
                ))
            }
        }
    }
    match (set, tombstone) {
        (Some(_), true) => Err(ResolveError::new("a flag both set and cleared on one tick")),
        (Some(b), false) => Ok(Resolved::Write(Value::Bool(b))),
        (None, true) => Ok(Resolved::Tombstone),
        (None, false) => current
            .map(Resolved::Write)
            .ok_or(ResolveError::new("flag resolved with no value")),
    }
}

/// Resolve a compass heading (hundredths of a degree): sets and turns compose like
/// [`compose_additive`] — a body may be turned by several deltas in one tick — and the result
/// wraps into `0..36000`, so turning past north comes back round rather than overflowing.
pub fn compose_heading(
    current: Option<Value>,
    changes: &[Change],
) -> Result<Resolved, ResolveError> {
    match compose_additive(current, changes)? {
        Resolved::Write(Value::Int(h)) => Ok(Resolved::Write(Value::Int(h.rem_euclid(36_000)))),
        other => Ok(other),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        compose_additive, compose_bounded, compose_entity_ref, compose_heading,
        compose_segment_field, compose_vec3,
    };
    use kernel::domain::Resolved;
    use kernel::identity::EntityId;
    use kernel::proposal::Change;
    use kernel::value::Value;

    #[test]
    fn additive_sums_deltas_onto_current() {
        let r = compose_additive(
            Some(Value::Int(2000)),
            &[Change::Delta(30), Change::Delta(-12)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(2018)));
    }

    #[test]
    fn additive_set_then_deltas() {
        let r = compose_additive(
            Some(Value::Int(2000)),
            &[Change::Set(Value::Int(500)), Change::Delta(25)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(525)));
    }

    #[test]
    fn additive_two_sets_conflict() {
        assert!(compose_additive(
            None,
            &[Change::Set(Value::Int(1)), Change::Set(Value::Int(2))]
        )
        .is_err());
    }

    #[test]
    fn bounded_clamps_into_range() {
        // A large positive delta is clamped to the ceiling.
        let hi = compose_bounded(Some(Value::Int(9900)), &[Change::Delta(500)], 0, 10000).unwrap();
        assert_eq!(hi, Resolved::Write(Value::Int(10000)));
        // A large negative delta is clamped to the floor.
        let lo = compose_bounded(Some(Value::Int(100)), &[Change::Delta(-500)], 0, 10000).unwrap();
        assert_eq!(lo, Resolved::Write(Value::Int(0)));
    }

    #[test]
    fn entity_ref_sets_the_reference() {
        let region = Value::Entity(EntityId::from_raw(7));
        let r = compose_entity_ref(None, &[Change::Create(region)]).unwrap();
        assert_eq!(r, Resolved::Write(region));
    }

    #[test]
    fn entity_ref_rejects_a_numeric_delta() {
        assert!(compose_entity_ref(None, &[Change::Delta(1)]).is_err());
    }

    #[test]
    fn vec3_is_set_whole_and_never_nudged() {
        let r = compose_vec3(
            Some(Value::Vec3([1, 2, 3])),
            &[Change::Set(Value::Vec3([4, 5, 6]))],
            None,
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Vec3([4, 5, 6])));
        assert!(compose_vec3(Some(Value::Vec3([1, 2, 3])), &[Change::Delta(1)], None).is_err());
        assert!(compose_vec3(None, &[Change::Set(Value::Int(1))], None).is_err());
        // Sizes clamp into range.
        let r = compose_vec3(
            None,
            &[Change::Set(Value::Vec3([-5, 10, 20]))],
            Some((0, 15)),
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Vec3([0, 10, 15])));
    }

    #[test]
    fn heading_wraps_round_the_compass() {
        let r = compose_heading(Some(Value::Int(35_000)), &[Change::Delta(2_000)]).unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(1_000)));
        let r = compose_heading(None, &[Change::Set(Value::Int(-9_000))]).unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(27_000)));
    }

    #[test]
    fn a_new_segment_supersedes_closing_the_old_one() {
        let target = Value::Vec3([5, 5, 0]);
        // Arrival closes, departure opens, same tick: the departure stands.
        let r = compose_segment_field(
            Some(Value::Vec3([1, 1, 0])),
            &[Change::Tombstone, Change::Set(target)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(target));
        // Closing alone closes.
        let r = compose_segment_field(Some(target), &[Change::Tombstone]).unwrap();
        assert_eq!(r, Resolved::Tombstone);
        // Two different destinations at once is a real contradiction.
        let two = [Change::Set(target), Change::Set(Value::Vec3([9, 9, 0]))];
        assert!(compose_segment_field(None, &two).is_err());
        // Agreeing sets merge — for segments and for positions alike.
        let same = [Change::Set(target), Change::Set(target)];
        assert!(compose_segment_field(None, &same).is_ok());
        assert!(compose_vec3(None, &same, None).is_ok());
        assert!(compose_vec3(None, &two, None).is_err());
    }
}
