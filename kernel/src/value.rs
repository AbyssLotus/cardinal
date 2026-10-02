//! Primitive fact values — the kernel's value space (Vol. V Ch. 2 §2.2, fact/triple model).
//!
//! The kernel defines a small, fixed set of primitive value types — the atoms domains
//! compose facts from — as a Datomic-style fact store does (Vol. V Ch. 2 §2.2). It knows
//! `Int`, `Bool`, `Entity`, and `Vec3`; it does not know "temperature" or "price"
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
    /// Three integers that belong together and change together — a position, a size, a
    /// target point (Amendment A-3). One atomic fact rather than three that could be proposed
    /// inconsistently; fixed-point like [`Value::Int`], scale and meaning the owner's.
    Vec3([i64; 3]),
}

impl Value {
    /// The `i64` inside, if this is [`Value::Int`]; otherwise `None`.
    pub const fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(v) => Some(*v),
            _ => None,
        }
    }

    /// The three integers inside, if this is [`Value::Vec3`]; otherwise `None`.
    pub const fn as_vec3(&self) -> Option<[i64; 3]> {
        match self {
            Value::Vec3(v) => Some(*v),
            _ => None,
        }
    }

    /// Append the value's canonical little-endian bytes for hashing and persistence.
    ///
    /// A one-byte tag distinguishes the variants so different-typed values never collide, and
    /// each variant has a fixed width (`Int`, `Entity`: 8 bytes; `Bool`: 8 bytes, zero-padded;
    /// `Vec3`: 24), so a sequence of values decodes unambiguously. The encoding is stable across
    /// platforms (Vol. V Ch. 4 §4.2).
    pub fn write_canonical(&self, out: &mut Vec<u8>) {
        match self {
            Value::Int(v) => {
                out.push(0);
                out.extend_from_slice(&v.to_le_bytes());
            }
            Value::Bool(b) => {
                out.push(1);
                out.extend_from_slice(&u64::from(*b).to_le_bytes());
            }
            Value::Entity(id) => {
                out.push(2);
                out.extend_from_slice(&id.raw().to_le_bytes());
            }
            Value::Vec3(v) => {
                out.push(3);
                for c in v {
                    out.extend_from_slice(&c.to_le_bytes());
                }
            }
        }
    }

    /// The value's canonical bytes (see [`Value::write_canonical`]).
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(25);
        self.write_canonical(&mut out);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::Value;
    use crate::identity::EntityId;

    #[test]
    fn canonical_bytes_are_tagged_and_distinct() {
        // The same bits under different types must never encode alike.
        let one = Value::Int(1).canonical_bytes();
        let truth = Value::Bool(true).canonical_bytes();
        let entity = Value::Entity(EntityId::from_raw(1)).canonical_bytes();
        assert_ne!(one, truth);
        assert_ne!(one, entity);
        let v = Value::Vec3([1, -2, 3]).canonical_bytes();
        assert_eq!(v.len(), 25);
        assert_eq!(v[0], 3);
        assert_eq!(&v[9..17], &(-2i64).to_le_bytes());
        assert_eq!(Value::Vec3([1, 2, 3]).as_vec3(), Some([1, 2, 3]));
        assert_eq!(Value::Int(4).as_vec3(), None);
    }
}
