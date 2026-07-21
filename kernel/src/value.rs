//! Primitive fact values — the kernel's value space (Vol. V Ch. 2 §2.2, fact/triple model).
//!
//! The kernel defines a small, fixed set of primitive value types — the atoms domains
//! compose facts from — as a Datomic-style fact store does (Vol. V Ch. 2 §2.2). It knows
//! `Int`, `Bool`, and `Entity`; it does not know "temperature" or "price"
//! (Vol. IV Ch. 1 §1.5.1). Real-valued quantities are represented as fixed-point integers,
//! never floats, so committed state carries no floating-point nondeterminism
//! (Vol. V Ch. 4); the scale is the owning domain's convention.

use crate::identity::EntityId;

/// A primitive committed value. Fact payloads are built from these atoms.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Value {
    /// A 64-bit signed integer. Real quantities use this as fixed-point (owner-defined scale).
    Int(i64),
    /// A boolean flag.
    Bool(bool),
    /// A reference to another entity — relationships are facts (Vol. V Ch. 2 §2.1).
    Entity(EntityId),
}

impl Value {
    /// The `i64` inside, if this is [`Value::Int`]; otherwise `None`.
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// Canonical, fixed-width little-endian bytes for hashing and persistence.
    ///
    /// A one-byte tag distinguishes the variants so different-typed values never collide;
    /// the encoding is stable across platforms (Vol. V Ch. 4 §4.2).
    pub fn canonical_bytes(&self) -> [u8; 9] {
        let mut out = [0u8; 9];
        match self {
            Value::Int(v) => {
                out[0] = 0;
                out[1..9].copy_from_slice(&v.to_le_bytes());
            }
            Value::Bool(b) => {
                out[0] = 1;
                out[1] = u8::from(*b);
            }
            Value::Entity(id) => {
                out[0] = 2;
                out[1..9].copy_from_slice(&id.raw().to_le_bytes());
            }
        }
        out
    }
}
