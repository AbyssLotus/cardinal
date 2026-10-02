//! Region queries: what a location lies within, what lies within a region, and whether two
//! regions overlap (Vol. III Ch. 1 §1.7, Regions; §1.12, Querying Reality).
//!
//! A location reaches its regions by two links. [`CONTAINED_IN`] is the single containment
//! hierarchy (§1.8) — the chain that also gives a location its coordinate frame
//! (`crate::space`). [`IN_REGION`] is the set of overlapping classifications a location belongs
//! to beyond that chain (§1.7) — a watershed, a climate zone, a territory — which need not nest
//! inside one another or inside the hierarchy. To these queries both links mean the same thing,
//! *lies wholly within*, and that relation is transitive: the regions of an entity are
//! everything reachable by following either link upward, any number of times, in any mix. A
//! farmer in a farmhouse on a farm in a watershed lies in the watershed.
//!
//! Consumers ask these questions rather than walking the links themselves, which keeps them
//! independent of how regions are stored (§1.4, representation independence): a later store
//! might answer from a spatial index or a membership bitmap and no caller would change. A
//! system that asks must declare both [`CONTAINED_IN`] and [`IN_REGION`] in its read set — the
//! kernel's scoped view refuses the read otherwise (Vol. V Ch. 3 §3.5, hermetic evaluation).
//!
//! Regions are semantic, not geometric (§1.7): two regions overlap when reality records a place
//! that lies in both, never because their shapes would intersect on a map. A region with no
//! recorded members overlaps nothing but itself and the regions it lies within.
//!
//! **Determinism.** Every walk visits entities in store order (sorted) and every result is a
//! `BTreeSet`, so answers are identical across runs and platforms (Vol. V Ch. 4 §4.1).
//! **Termination.** Membership is authored data and may contain a cycle — two regions declared
//! within each other, the same extent under two names. Each walk keeps a visited set, so a
//! cycle ends the walk instead of looping, and no entity is ever reported as lying within
//! itself.

use crate::schema::{CONTAINED_IN, IN_REGION};
use kernel::fact::FactKey;
use kernel::identity::EntityId;
use kernel::system::CommittedView;
use kernel::value::Value;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// The regions `entity` names directly as memberships ([`IN_REGION`]), in deterministic order.
/// Only the declared set: its container and anything inherited are not included — see
/// [`regions_of`] for the full answer.
pub fn memberships(view: &dyn CommittedView, entity: EntityId) -> Vec<EntityId> {
    view.read_all(FactKey::new(entity, IN_REGION))
        .into_iter()
        .filter_map(|f| match f.value {
            Value::Entity(region) => Some(region),
            _ => None,
        })
        .collect()
}

/// The entities `entity` lies *immediately* within: its container, if it has one, followed by
/// its declared memberships. One step up both links — the building block of [`regions_of`].
fn parents(view: &dyn CommittedView, entity: EntityId) -> Vec<EntityId> {
    let mut out = Vec::new();
    if let Some(Value::Entity(container)) = view
        .read(FactKey::new(entity, CONTAINED_IN))
        .map(|f| f.value)
    {
        out.push(container);
    }
    out.extend(memberships(view, entity));
    out
}

/// Every region `entity` lies within (Vol. III Ch. 1 §1.7): its whole containment chain, its
/// declared memberships, and everything those lie within in turn. The spec's farmhouse answer —
/// county, state, and continent from the hierarchy; watershed, climate zone, and territory from
/// the overlapping classifications — comes back as one set. Excludes `entity` itself.
///
/// A breadth-first walk up both links with a visited set, so a region reached by two routes (a
/// county that is both the farm's container and, through the climate zone, a member of it) is
/// counted once and a membership cycle terminates.
pub fn regions_of(view: &dyn CommittedView, entity: EntityId) -> BTreeSet<EntityId> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([entity]);
    while let Some(here) = queue.pop_front() {
        for parent in parents(view, here) {
            // `parent != entity`: a cycle that leads back to the start must not report the
            // entity as lying within itself.
            if parent != entity && seen.insert(parent) {
                queue.push_back(parent);
            }
        }
    }
    seen
}

/// Whether `entity` lies within `region` by any route of containment or membership
/// (Vol. III Ch. 1 §1.7) — "is the farmhouse in the watershed". An entity does not lie within
/// itself.
pub fn is_within(view: &dyn CommittedView, entity: EntityId, region: EntityId) -> bool {
    regions_of(view, entity).contains(&region)
}

/// The regions `a` and `b` both lie within — the context two locations share (Vol. III Ch. 1
/// §1.6, §1.7). Two farmers on neighbouring farms share their county and climate zone, and
/// perhaps a watershed; a consumer picks the lens it reasons in (§1.7, "each of these regions
/// answers a different class of questions").
pub fn shared_regions(view: &dyn CommittedView, a: EntityId, b: EntityId) -> BTreeSet<EntityId> {
    let of_b = regions_of(view, b);
    regions_of(view, a)
        .into_iter()
        .filter(|r| of_b.contains(r))
        .collect()
}

/// For each region or container, the entities that lie *immediately* within it, by either
/// link. The forward links are stored on the member — a location names its regions — so asking
/// the other way, "what lies in this region", needs the reverse. One pass over the entities
/// that carry either link fact builds it.
///
/// Built per query, this costs O(links in the world). That is fine at the scale of today's
/// worlds and keeps the store free of a second, derived structure to keep coherent; at planet
/// scale a store would maintain the reverse incrementally instead (§1.15, Scale Through
/// Locality). It is private precisely so that swap changes no caller.
fn children_index(view: &dyn CommittedView) -> BTreeMap<EntityId, Vec<EntityId>> {
    let mut index: BTreeMap<EntityId, Vec<EntityId>> = BTreeMap::new();
    for child in view.entities_with(CONTAINED_IN) {
        if let Some(Value::Entity(container)) = view
            .read(FactKey::new(child, CONTAINED_IN))
            .map(|f| f.value)
        {
            index.entry(container).or_default().push(child);
        }
    }
    for child in view.entities_with(IN_REGION) {
        for region in memberships(view, child) {
            index.entry(region).or_default().push(child);
        }
    }
    index
}

/// Everything that lies within `region` according to `index`: the downward mirror of
/// [`regions_of`]'s walk. Excludes `region` itself.
fn extent(index: &BTreeMap<EntityId, Vec<EntityId>>, region: EntityId) -> BTreeSet<EntityId> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::from([region]);
    while let Some(here) = queue.pop_front() {
        for &child in index.get(&here).map(Vec::as_slice).unwrap_or_default() {
            if child != region && seen.insert(child) {
                queue.push_back(child);
            }
        }
    }
    seen
}

/// Every entity that lies within `region` by any route (Vol. III Ch. 1 §1.7): its declared
/// members, everything they contain, and every region declared within it along with *their*
/// contents. A discontiguous region is simply the places that name it — an island nation is its
/// islands and everything on them — so no geometry is needed to enumerate it. Excludes `region`
/// itself.
pub fn members_of(view: &dyn CommittedView, region: EntityId) -> BTreeSet<EntityId> {
    extent(&children_index(view), region)
}

/// Whether regions `a` and `b` overlap (Vol. III Ch. 1 §1.6, "Overlaps"; §1.7): whether some
/// entity lies within both, or one lies within the other. A region overlaps itself.
///
/// A fox's territory and a farm overlap when the territory takes in one of the farm's fields,
/// even though neither lies within the other — the case a single hierarchy cannot express, and
/// the reason §1.7 exists. Judged on each region's extent including the region itself, so "one
/// lies within the other" and "they share a member" are the same disjointness test.
pub fn overlaps(view: &dyn CommittedView, a: EntityId, b: EntityId) -> bool {
    if a == b {
        return true;
    }
    let index = children_index(view);
    let mut extent_a = extent(&index, a);
    extent_a.insert(a);
    let mut extent_b = extent(&index, b);
    extent_b.insert(b);
    !extent_a.is_disjoint(&extent_b)
}
