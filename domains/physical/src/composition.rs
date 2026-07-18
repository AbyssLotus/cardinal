//! Proposal-composition for Physical Reality-owned fact types (Vol. IV Ch. 2).
//!
//! Composition rules decide how competing proposals against an owned fact reconcile before
//! commit (Vol. V Ch. 3 §3.1, Resolve). Every tunable threshold lives in the world package,
//! never here (Vol. IV Ch. 2 §2.2).

use kernel::domain::{ResolveError, Resolved};
use kernel::proposal::Change;
use kernel::value::Value;

/// Compose competing temperature proposals into one resolved outcome.
///
/// Rule: at most one `Set`/`Create` may establish the base (two competing sets are an
/// undeclared conflict and fail — Vol. V Ch. 3 §3.5, invariant 4); then all `Delta`s sum
/// onto it. With no proposal touching the base, deltas apply to the current committed
/// value. A `Tombstone` may not share the tick with a set or delta. This is the
/// owner-declared rule the kernel's Resolve stage calls (Vol. IV Ch. 2).
pub fn compose_temperature(
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
                    .ok_or(ResolveError::new("temperature must be an integer value"))?;
                if set_seen {
                    return Err(ResolveError::new(
                        "two competing temperature sets with no declared tie-break",
                    ));
                }
                set_seen = true;
                base = Some(n);
            }
            Change::Delta(d) => delta_sum = delta_sum.saturating_add(*d),
            Change::Tombstone => tombstone = true,
        }
    }

    if tombstone {
        if set_seen || delta_sum != 0 {
            return Err(ResolveError::new(
                "temperature tombstone conflicts with a set or delta on the same tick",
            ));
        }
        return Ok(Resolved::Tombstone);
    }

    let start = base.unwrap_or(0);
    Ok(Resolved::Write(Value::Int(start.saturating_add(delta_sum))))
}

#[cfg(test)]
mod tests {
    use super::compose_temperature;
    use kernel::domain::Resolved;
    use kernel::proposal::Change;
    use kernel::value::Value;

    #[test]
    fn deltas_sum_onto_current() {
        // Two systems each nudge temperature; resolution sums their deltas (Vol. IV Ch. 2).
        let r = compose_temperature(
            Some(Value::Int(2000)),
            &[Change::Delta(30), Change::Delta(-12)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(2018)));
    }

    #[test]
    fn set_establishes_base_then_deltas_apply() {
        let r = compose_temperature(
            Some(Value::Int(2000)),
            &[Change::Set(Value::Int(500)), Change::Delta(25)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(525)));
    }

    #[test]
    fn two_competing_sets_are_a_conflict() {
        let err = compose_temperature(
            None,
            &[Change::Set(Value::Int(100)), Change::Set(Value::Int(200))],
        );
        assert!(err.is_err());
    }
}
