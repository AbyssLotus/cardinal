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

#[cfg(test)]
mod tests {
    use super::{compose_additive, compose_bounded, compose_entity_ref};
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
}
