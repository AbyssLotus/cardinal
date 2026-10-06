//! Material queries: what an object is made of, and what that composition implies
//! (Vol. III Ch. 1 §1.9, Materials; §1.12, Querying Reality).
//!
//! Materials are representation-independent, exactly as space is (§1.4): a consumer asks a
//! question — *is this flammable, how strong is it, what is it made of* — and gets an answer
//! without depending on how composition is stored or which properties a material happens to
//! expose. A material is an entity carrying property facts ([`crate::schema::MATERIAL_DENSITY`]
//! and friends); an object references its materials through [`crate::schema::MADE_OF`], a
//! cardinality-many link, because composites are the rule, not the exception (§1.9).
//!
//! The property-over-names discipline lives here: nothing in this module — or anywhere in the
//! engine — knows "oak" or "steel". It knows density, hardness, flammability. A composite's
//! behaviour is derived from its constituents' *properties*, and only for aggregates that are
//! well-defined without knowing each material's proportion: a structure is only as strong as
//! its weakest material, and burns or poisons as readily as its worst one. Proportion-weighted
//! aggregates (bulk density, total heat stored) await a per-constituent proportion fact and
//! are deliberately not offered yet.

use crate::schema::{
    MADE_OF, MATERIAL_CONDUCTIVITY, MATERIAL_DENSITY, MATERIAL_FLAMMABILITY, MATERIAL_HARDNESS,
    MATERIAL_THERMAL_CAPACITY, MATERIAL_TOXICITY,
};
use kernel::fact::{FactKey, FactType};
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;

/// The materials `object` is composed of, in deterministic order (Vol. III Ch. 1 §1.9). Empty
/// if the object declares no composition. Each returned entity carries the property facts the
/// accessors below read.
pub fn materials_of(view: &dyn CommittedView, object: EntityId) -> Vec<EntityId> {
    view.read_all(FactKey::new(object, MADE_OF))
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(material) => Some(material),
            _ => None,
        })
        .collect()
}

/// One integer property of a single material entity, or `None` if the material does not expose
/// it. The generic accessor behind the typed ones below — a material exposes only the
/// characteristics it has (§1.9, properties over names).
pub fn property(view: &dyn CommittedView, material: EntityId, property: FactType) -> Option<i64> {
    view.read(FactKey::new(material, property))
        .and_then(|f| f.value.as_int())
}

/// A material's density, in kg/m³ ([`crate::schema::MATERIAL_DENSITY`]).
pub fn density(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_DENSITY)
}

/// A material's hardness / structural strength, 0..=10000 ([`crate::schema::MATERIAL_HARDNESS`]).
pub fn hardness(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_HARDNESS)
}

/// A material's specific heat capacity, J/(kg·K) ([`crate::schema::MATERIAL_THERMAL_CAPACITY`]).
pub fn thermal_capacity(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_THERMAL_CAPACITY)
}

/// A material's flammability, 0..=10000 ([`crate::schema::MATERIAL_FLAMMABILITY`]).
pub fn flammability(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_FLAMMABILITY)
}

/// A material's conductivity, 0..=10000 ([`crate::schema::MATERIAL_CONDUCTIVITY`]).
pub fn conductivity(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_CONDUCTIVITY)
}

/// How far a material can be taken into a body, 0..=10000 ([`crate::schema::MATERIAL_EDIBLE`]).
pub fn edible(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, crate::schema::MATERIAL_EDIBLE)
}

/// Whether `object` can be consumed: it is made of something, and everything it is made of is
/// edible (Amendment A-12).
pub fn is_edible(view: &dyn CommittedView, object: EntityId) -> bool {
    let materials = materials_of(view, object);
    !materials.is_empty()
        && materials
            .iter()
            .all(|m| edible(view, *m).is_some_and(|e| e > 0))
}

/// What `object` weighs, in grams (Amendment A-12): its volume — a body's size is its half-width,
/// half-depth, and height — times the density of the densest material it is made of, the same
/// dominant-constituent estimate as thermal mass. `None` when the world does not know: it has no
/// size, or is made of nothing with a density.
pub fn weight_g(view: &dyn CommittedView, object: EntityId) -> Option<i64> {
    let [hx, hy, h] = crate::index::size_of(view, object)?;
    let density = materials_of(view, object)
        .into_iter()
        .filter_map(|m| density(view, m))
        .max()?;
    let volume_cm3 = (2 * hx as i128) * (2 * hy as i128) * h as i128;
    Some((volume_cm3 * density as i128 / 1000).min(i64::MAX as i128) as i64)
}

/// A material's toxicity, 0..=10000 ([`crate::schema::MATERIAL_TOXICITY`]).
pub fn toxicity(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    property(view, material, MATERIAL_TOXICITY)
}

/// The structural hardness of a composite `object`: the **minimum** hardness across its
/// materials — a structure fails at its weakest material, whatever it is named (Vol. III Ch. 1
/// §1.9, the bridge that "fails because the material cannot support the required stress").
/// `None` if the object has no materials, or none of them expose a hardness.
pub fn structural_hardness(view: &dyn CommittedView, object: EntityId) -> Option<i64> {
    materials_of(view, object)
        .into_iter()
        .filter_map(|m| hardness(view, m))
        .min()
}

/// The flammability of a composite `object`: the **maximum** flammability across its materials
/// — fire spreads where the most flammable constituent satisfies ignition conditions
/// (Vol. III Ch. 1 §1.9). `None` if the object has no materials that expose a flammability.
pub fn flammability_of(view: &dyn CommittedView, object: EntityId) -> Option<i64> {
    materials_of(view, object)
        .into_iter()
        .filter_map(|m| flammability(view, m))
        .max()
}

/// Whether a composite `object` can burn at all: any constituent has a nonzero flammability
/// (Vol. III Ch. 1 §1.9). The threshold for *how readily* it ignites is a consumer's rule, not
/// a hardcoded engine number — this only answers "is it combustible".
pub fn is_flammable(view: &dyn CommittedView, object: EntityId) -> bool {
    flammability_of(view, object).is_some_and(|f| f > 0)
}

/// The toxicity of a composite `object`: the **maximum** toxicity across its materials — a
/// composite is as hazardous as its most toxic part (Vol. III Ch. 1 §1.9). `None` if no
/// constituent exposes a toxicity.
pub fn toxicity_of(view: &dyn CommittedView, object: EntityId) -> Option<i64> {
    materials_of(view, object)
        .into_iter()
        .filter_map(|m| toxicity(view, m))
        .max()
}

/// A material's thermal mass: the heat one cubic metre of it stores per degree, kJ/(m³·K) —
/// its density times its specific heat (Amendment A-6). A wall holds heat by volume, not by
/// weight: timber stores more per kilogram than granite, granite nearly twice as much per cubic
/// metre. `None` unless the material exposes both.
pub fn thermal_mass(view: &dyn CommittedView, material: EntityId) -> Option<i64> {
    Some(density(view, material)?.saturating_mul(thermal_capacity(view, material)?) / 1000)
}

/// The thermal mass of a composite `object`, kJ/(m³·K): the **maximum** across its materials —
/// a proportion-free estimate of its thermal inertia, its resistance to changing temperature
/// (Vol. III Ch. 1 §1.9; Amendment A-6). Like the other composite aggregates here it uses the
/// dominant constituent rather than a volume-weighted mean, which awaits a per-constituent
/// proportion fact. `None` if no material exposes a thermal mass; consumers (the environment's
/// temperature systems) read this to damp a region's swing.
pub fn thermal_mass_of(view: &dyn CommittedView, object: EntityId) -> Option<i64> {
    materials_of(view, object)
        .into_iter()
        .filter_map(|m| thermal_mass(view, m))
        .max()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{
        MATERIAL_FLAMMABILITY, MATERIAL_HARDNESS, MATERIAL_THERMAL_CAPACITY, MATERIAL_TOXICITY,
    };
    use kernel::fact::{Cause, Fact, Provenance, SystemId};
    use kernel::store::MemoryStore;

    fn prov() -> Provenance {
        Provenance::new(SystemId::new("worldgen"), 0, Cause::new("seed"))
    }

    fn seed_int(store: &mut MemoryStore, e: EntityId, ft: FactType, v: i64) {
        store.seed(FactKey::new(e, ft), Fact::new(Value::Int(v), prov()));
    }

    fn made_of(store: &mut MemoryStore, object: EntityId, material: EntityId) {
        store.seed(
            FactKey::new(object, MADE_OF),
            Fact::new(Value::Entity(material), prov()),
        );
    }

    // A door of oak (soft, very flammable) reinforced with an iron band (hard, inert).
    fn oak_and_iron_door() -> (MemoryStore, EntityId) {
        let door = EntityId::from_raw(1);
        let oak = EntityId::from_raw(100);
        let iron = EntityId::from_raw(101);
        let mut store = MemoryStore::new();
        seed_int(&mut store, oak, MATERIAL_HARDNESS, 3000);
        seed_int(&mut store, oak, MATERIAL_FLAMMABILITY, 7000);
        seed_int(&mut store, iron, MATERIAL_HARDNESS, 9000);
        seed_int(&mut store, iron, MATERIAL_FLAMMABILITY, 0);
        seed_int(&mut store, iron, MATERIAL_TOXICITY, 200);
        made_of(&mut store, door, oak);
        made_of(&mut store, door, iron);
        (store, door)
    }

    #[test]
    fn materials_of_lists_every_constituent() {
        let (store, door) = oak_and_iron_door();
        let mats: Vec<u64> = materials_of(&store, door).iter().map(|e| e.raw()).collect();
        assert_eq!(mats, vec![100, 101]); // deterministic, ascending
    }

    #[test]
    fn structure_is_only_as_strong_as_its_weakest_material() {
        // The oak (3000), not the iron band (9000), governs how the door fails.
        let (store, door) = oak_and_iron_door();
        assert_eq!(structural_hardness(&store, door), Some(3000));
    }

    #[test]
    fn a_composite_burns_as_readily_as_its_most_flammable_part() {
        let (store, door) = oak_and_iron_door();
        assert_eq!(flammability_of(&store, door), Some(7000));
        assert!(is_flammable(&store, door));
    }

    #[test]
    fn an_all_inert_object_does_not_burn() {
        let slab = EntityId::from_raw(1);
        let stone = EntityId::from_raw(200);
        let mut store = MemoryStore::new();
        seed_int(&mut store, stone, MATERIAL_FLAMMABILITY, 0);
        seed_int(&mut store, stone, MATERIAL_HARDNESS, 8000);
        made_of(&mut store, slab, stone);
        assert!(!is_flammable(&store, slab));
        assert_eq!(structural_hardness(&store, slab), Some(8000));
    }

    #[test]
    fn thermal_mass_is_heat_stored_per_volume() {
        // A granite wall lined with a timber panel. Timber stores more heat per kilogram, but
        // granite — nearly four times as dense — stores far more per cubic metre, and it is the
        // granite that gives the wall its thermal inertia.
        let wall = EntityId::from_raw(1);
        let stone = EntityId::from_raw(300);
        let timber = EntityId::from_raw(301);
        let mut store = MemoryStore::new();
        seed_int(&mut store, stone, MATERIAL_DENSITY, 2700);
        seed_int(&mut store, stone, MATERIAL_THERMAL_CAPACITY, 790);
        seed_int(&mut store, timber, MATERIAL_DENSITY, 700);
        seed_int(&mut store, timber, MATERIAL_THERMAL_CAPACITY, 1700);
        made_of(&mut store, wall, stone);
        made_of(&mut store, wall, timber);
        assert_eq!(thermal_mass(&store, timber), Some(1190));
        assert_eq!(thermal_mass(&store, stone), Some(2133));
        assert_eq!(thermal_mass_of(&store, wall), Some(2133));
        // A material without a density has no thermal mass to report; nor has no material.
        let mystery = EntityId::from_raw(302);
        seed_int(&mut store, mystery, MATERIAL_THERMAL_CAPACITY, 4000);
        assert_eq!(thermal_mass(&store, mystery), None);
        assert_eq!(thermal_mass_of(&store, EntityId::from_raw(999)), None);
    }

    #[test]
    fn toxicity_takes_the_worst_constituent_and_missing_properties_are_absent() {
        let (store, door) = oak_and_iron_door();
        // Only the iron declares a toxicity; the oak does not, so the max is the iron's.
        assert_eq!(toxicity_of(&store, door), Some(200));
        // An object with no composition has no derived properties.
        let nothing = EntityId::from_raw(999);
        assert!(materials_of(&store, nothing).is_empty());
        assert_eq!(structural_hardness(&store, nothing), None);
        assert!(!is_flammable(&store, nothing));
    }
}
