//! Proposal-composition for Living Systems-owned fact types (Vol. IV Ch. 2).
//!
//! Composition rules decide how competing proposals against an owned fact reconcile before
//! commit (Vol. V Ch. 3 §3.1, Resolve). Every tunable threshold lives in the world package,
//! never here (Vol. IV Ch. 2 §2.2).

use kernel::domain::{ResolveError, Resolved};
use kernel::proposal::Change;
use kernel::value::Value;

/// Compose competing body-heat proposals into one resolved outcome.
///
/// Rule: at most one `Set`/`Create` establishes the base (two competing sets are an
/// undeclared conflict and fail — Vol. V Ch. 3 §3.5, invariant 4); then all `Delta`s sum
/// onto it. With no proposal touching the base, deltas apply to the current committed value.
/// A `Tombstone` (the organism's heat ceasing to exist, e.g. at death) may not share the
/// tick with a set or delta.
pub fn compose_body_heat(
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
                    .ok_or(ResolveError::new("body heat must be an integer value"))?;
                if set_seen {
                    return Err(ResolveError::new(
                        "two competing body-heat sets with no declared tie-break",
                    ));
                }
                set_seen = true;
                base = Some(n);
            }
            Change::Delta(d) => delta_sum = delta_sum.saturating_add(*d),
            Change::Tombstone => tombstone = true,
            Change::Add(_) | Change::Remove(_) => {
                return Err(ResolveError::new(
                    "body heat is single-valued; use Set/Create/Delta, not Add/Remove",
                ))
            }
        }
    }

    if tombstone {
        if set_seen || delta_sum != 0 {
            return Err(ResolveError::new(
                "body-heat tombstone conflicts with a set or delta on the same tick",
            ));
        }
        return Ok(Resolved::Tombstone);
    }

    let start = base.unwrap_or(0);
    Ok(Resolved::Write(Value::Int(start.saturating_add(delta_sum))))
}

#[cfg(test)]
mod tests {
    use super::compose_body_heat;
    use kernel::domain::Resolved;
    use kernel::proposal::Change;
    use kernel::value::Value;

    #[test]
    fn deltas_sum_onto_current() {
        let r = compose_body_heat(
            Some(Value::Int(3700)),
            &[Change::Delta(-200), Change::Delta(50)],
        )
        .unwrap();
        assert_eq!(r, Resolved::Write(Value::Int(3550)));
    }

    #[test]
    fn two_competing_sets_conflict() {
        let err = compose_body_heat(
            None,
            &[Change::Set(Value::Int(3700)), Change::Set(Value::Int(3600))],
        );
        assert!(err.is_err());
    }
}
