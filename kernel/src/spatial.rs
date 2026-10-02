//! The store's derived spatial index — Vol. V Ch. 2 §2.1 (clause 5 and Amendment A-2).
//!
//! Spatial questions are contract surface (Vol. V Ch. 2, invariant 8), and a world of people
//! in rooms asks them thousands of times a tick. Scanning every entity per question grows with
//! the square of the population (audit §4.2), so the store keeps an index — but on the
//! architecture's terms:
//!
//! - **Content-free.** The kernel never learns what a position is. The domain that owns space
//!   registers a [`SpatialProjector`]: a pure function from an entity's own committed facts to
//!   a [`Placement`] — the *frame* it is placed in (another entity: a room, a ship, a region)
//!   and a bounding box in that frame's local coordinates. The kernel indexes boxes per frame;
//!   it knows nothing else.
//! - **Committed, always.** The store re-places an entity whenever `apply()` (or world-gen
//!   seeding) touches a fact the projector watches, so the index is exactly the committed tick
//!   (Vol. V Ch. 2 §2.1, clause 3).
//! - **No opinions.** The index answers *candidate* questions — what might lie in this box of
//!   this frame — and the owning domain answers exact questions from them. Every answer must
//!   equal the answer computed by scanning facts (the conformance rule of Amendment A-2), and
//!   reading through the index requires declaring every watched fact type (enforced by the
//!   kernel's scoped view, Vol. V Ch. 3 §3.5).
//!
//! ## Structure: a hierarchical grid per frame
//!
//! Each frame has its own grid, in its own coordinates, so when a container moves (a wagon, a
//! ship) nothing inside it needs re-indexing — only the container's own placement in *its*
//! frame changes. Within a frame, boxes are filed by size into one of [`LEVELS`] grids whose
//! cell edge grows eightfold per level, from [`BASE_CELL_CM`] (4 m) upward. A box goes into the
//! finest level whose cell is at least as large as the box's longest side, so it touches at
//! most two cells per axis (eight in all): a mouse and a mountain both cost a handful of
//! entries, and no cell size needs tuning per world. A query probes, at every level that holds
//! anything, the cells its box overlaps; if that would be many probes (a query box far larger
//! than the things in it), it scans the frame's member list instead, which is then cheaper.
//!
//! ## Determinism
//!
//! Lookups use hash maps with a fixed, seedless hasher ([`IdHasher`]) and are never iterated —
//! a hash map's iteration order is an accident, and accidents diverge replays (Vol. V Ch. 4,
//! Door 2). Every list this module returns is sorted by entity id. Member lists that *are*
//! iterated are `BTreeSet`s.

use crate::fact::FactType;
use crate::identity::EntityId;
use crate::system::CommittedView;
use std::collections::{BTreeSet, HashMap};
use std::fmt;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Arc;

/// An axis-aligned box in one frame's local coordinates, bounds inclusive. The kernel attaches
/// no unit to the numbers; the domain that owns space chooses one (Physical Reality uses
/// centimetres).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Aabb {
    /// The low corner.
    pub min: [i64; 3],
    /// The high corner.
    pub max: [i64; 3],
}

impl Aabb {
    /// The box containing every point: what an unbounded thing occupies, and the query for
    /// "everything in this frame".
    pub const EVERYTHING: Aabb = Aabb {
        min: [i64::MIN; 3],
        max: [i64::MAX; 3],
    };

    /// The degenerate box at a single point.
    pub const fn point(p: [i64; 3]) -> Self {
        Self { min: p, max: p }
    }

    /// The box spanning two corners, in either order.
    pub fn new(a: [i64; 3], b: [i64; 3]) -> Self {
        let mut min = [0; 3];
        let mut max = [0; 3];
        for i in 0..3 {
            min[i] = a[i].min(b[i]);
            max[i] = a[i].max(b[i]);
        }
        Self { min, max }
    }

    /// The cube of half-size `radius` around `center` (saturating at the coordinate limits).
    pub fn around(center: [i64; 3], radius: i64) -> Self {
        let r = radius.max(0);
        Self {
            min: center.map(|c| c.saturating_sub(r)),
            max: center.map(|c| c.saturating_add(r)),
        }
    }

    /// Whether the two boxes share at least one point.
    pub fn intersects(&self, other: &Aabb) -> bool {
        (0..3).all(|i| self.min[i] <= other.max[i] && other.min[i] <= self.max[i])
    }

    /// Whether `p` lies inside the box (bounds inclusive).
    pub fn contains(&self, p: [i64; 3]) -> bool {
        (0..3).all(|i| self.min[i] <= p[i] && p[i] <= self.max[i])
    }

    /// The smallest box containing both.
    pub fn union(&self, other: &Aabb) -> Aabb {
        let mut out = *self;
        for i in 0..3 {
            out.min[i] = out.min[i].min(other.min[i]);
            out.max[i] = out.max[i].max(other.max[i]);
        }
        out
    }

    /// The box moved by `delta` (saturating).
    pub fn translate(&self, delta: [i64; 3]) -> Aabb {
        let mut out = *self;
        for i in 0..3 {
            out.min[i] = out.min[i].saturating_add(delta[i]);
            out.max[i] = out.max[i].saturating_add(delta[i]);
        }
        out
    }

    /// The box grown by `margin` on every side (saturating).
    pub fn expand(&self, margin: i64) -> Aabb {
        let m = margin.max(0);
        Aabb {
            min: self.min.map(|v| v.saturating_sub(m)),
            max: self.max.map(|v| v.saturating_add(m)),
        }
    }

    /// The length of the box's longest side.
    pub fn longest_side(&self) -> u128 {
        (0..3)
            .map(|i| (self.max[i] as i128 - self.min[i] as i128) as u128)
            .max()
            .unwrap_or(0)
    }
}

/// Where an entity sits, for indexing: the frame it is placed in and its extent there.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Placement {
    /// The entity whose local coordinates `anchor` and `bounds` are expressed in.
    pub frame: EntityId,
    /// The entity's reference point in the frame (the owning domain's convention). Valid at
    /// every tick only when `settled`.
    pub anchor: [i64; 3],
    /// A box containing everything the entity occupies in the frame — at every tick, even for
    /// an entity in motion (whose box then spans its whole path).
    pub bounds: Aabb,
    /// Whether `anchor` is the entity's reference point at every tick (it is not moving). When
    /// false, a consumer needing the exact point must compute it from committed facts; `bounds`
    /// is still a guaranteed superset.
    pub settled: bool,
}

/// The rule, registered by the domain that owns space, that turns committed facts into a
/// [`Placement`] (Amendment A-2). Must be a pure function of the entity's **own** facts, read
/// through `view`, so that re-placing exactly the entities whose watched facts changed keeps the
/// whole index exact. It must not consult `view.spatial()` (the index is being updated).
pub trait SpatialProjector: Send + Sync {
    /// Every fact type `place` reads. A change to any of them re-places the entity; a system
    /// reading through the index must declare them all.
    fn watches(&self) -> &'static [FactType];

    /// Where `entity` sits in committed reality, or `None` if it is not placed anywhere.
    fn place(&self, view: &dyn CommittedView, entity: EntityId) -> Option<Placement>;
}

/// The read surface of a spatial index, as systems see it through `CommittedView::spatial`.
pub trait SpatialQuery {
    /// The fact types the index mirrors (see [`SpatialProjector::watches`]).
    fn watches(&self) -> &[FactType];

    /// Where `entity` is placed, if it is.
    fn placement(&self, entity: EntityId) -> Option<Placement>;

    /// Every entity placed in `frame` whose bounds intersect `bounds`, with its placement,
    /// sorted by entity id. Exact for boxes: no false candidates, none missed.
    fn candidates(&self, frame: EntityId, bounds: &Aabb) -> Vec<(EntityId, Placement)>;

    /// Every entity placed directly in `frame`, sorted by id.
    fn children(&self, frame: EntityId) -> Vec<EntityId>;

    /// The entities placed directly in `frame` that are themselves frames (something is placed
    /// in them), sorted by id — the branches a search descends into.
    fn subframes(&self, frame: EntityId) -> Vec<EntityId>;
}

/// The number of grid levels per frame.
pub const LEVELS: usize = 17;

/// The cell edge of the finest level (the kernel's number, not a world rule: it changes the
/// cost of queries, never their answers). With Physical Reality's centimetres, 4 m.
pub const BASE_CELL_CM: i64 = 400;

/// Cell edge at `level`: [`BASE_CELL_CM`] × 8^level. Level 16 is about 1.1 × 10^17, beyond
/// which a box is "oversized" and simply listed per frame.
const fn cell_size(level: usize) -> i64 {
    BASE_CELL_CM << (3 * level)
}

/// A query that would probe more cells than this scans the frame's members instead.
const PROBE_LIMIT: u128 = 512;

/// A seedless multiply-rotate hasher (in the style of rustc's FxHash). Deterministic across
/// runs, which `std`'s randomly-seeded default is not — though this module never iterates its
/// hash maps, so only speed depends on the hasher.
#[derive(Clone, Copy, Default)]
pub struct IdHasher(u64);

impl Hasher for IdHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.write_u64(u64::from_le_bytes(word));
        }
    }
    fn write_u64(&mut self, v: u64) {
        self.0 = (self.0.rotate_left(5) ^ v).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_i64(&mut self, v: i64) {
        self.write_u64(v as u64);
    }
    fn write_u8(&mut self, v: u8) {
        self.write_u64(u64::from(v));
    }
    fn write_usize(&mut self, v: usize) {
        self.write_u64(v as u64);
    }
}

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<IdHasher>>;

/// One grid cell: a frame, a level, and integer cell coordinates.
type CellKey = (EntityId, u8, i64, i64, i64);

/// An entity's entry: its placement and the cells it was filed under.
#[derive(Clone, Copy, Debug)]
struct Slot {
    placement: Placement,
    /// `None` when oversized (listed per frame rather than in cells).
    level: Option<u8>,
    lo: [i64; 3],
    hi: [i64; 3],
}

/// The finest level whose cells are at least as large as `bounds`' longest side, or `None` if
/// the box is larger than any level (oversized).
fn level_for(bounds: &Aabb) -> Option<usize> {
    let side = bounds.longest_side();
    (0..LEVELS).find(|&l| side <= cell_size(l) as u128)
}

/// The cell coordinates `bounds` spans at `level`.
fn cell_range(bounds: &Aabb, level: usize) -> ([i64; 3], [i64; 3]) {
    let s = cell_size(level);
    (
        bounds.min.map(|v| v.div_euclid(s)),
        bounds.max.map(|v| v.div_euclid(s)),
    )
}

/// How many cells the inclusive range `lo..=hi` holds.
fn cell_count(lo: [i64; 3], hi: [i64; 3]) -> u128 {
    (0..3)
        .map(|i| (hi[i] as i128 - lo[i] as i128 + 1) as u128)
        .fold(1u128, |acc, n| acc.saturating_mul(n))
}

/// The store-maintained spatial index (Amendment A-2). Built and updated only by the store;
/// systems read it through [`SpatialQuery`].
#[derive(Clone)]
pub struct SpatialIndex {
    projector: Arc<dyn SpatialProjector>,
    slots: FastMap<EntityId, Slot>,
    cells: FastMap<CellKey, Vec<EntityId>>,
    oversized: FastMap<EntityId, BTreeSet<EntityId>>,
    /// Per frame, how many entries each level holds — so queries skip empty levels.
    level_counts: FastMap<EntityId, [u32; LEVELS]>,
    /// Per frame, everything placed directly in it.
    children: FastMap<EntityId, BTreeSet<EntityId>>,
    /// Per frame, the children that are themselves frames.
    subframes: FastMap<EntityId, BTreeSet<EntityId>>,
}

impl fmt::Debug for SpatialIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpatialIndex")
            .field("placed", &self.slots.len())
            .field("cells", &self.cells.len())
            .finish()
    }
}

impl SpatialIndex {
    /// An empty index governed by `projector`.
    pub fn new(projector: Arc<dyn SpatialProjector>) -> Self {
        Self {
            projector,
            slots: FastMap::default(),
            cells: FastMap::default(),
            oversized: FastMap::default(),
            level_counts: FastMap::default(),
            children: FastMap::default(),
            subframes: FastMap::default(),
        }
    }

    /// The placement rule this index mirrors.
    pub fn projector(&self) -> &Arc<dyn SpatialProjector> {
        &self.projector
    }

    /// Whether `entity` currently has anything placed in it.
    fn is_frame(&self, entity: EntityId) -> bool {
        self.children.get(&entity).is_some_and(|c| !c.is_empty())
    }

    /// Re-place `entity`: remove its old entry (if any) and file `placement` (if any). The one
    /// mutation path of the index, called by the store after every commit that touched one of
    /// the entity's watched facts.
    pub fn update(&mut self, entity: EntityId, placement: Option<Placement>) {
        if let Some(old) = self.slots.remove(&entity) {
            self.unfile(entity, &old);
        }
        if let Some(p) = placement {
            self.file(entity, p);
        }
    }

    fn unfile(&mut self, entity: EntityId, slot: &Slot) {
        let frame = slot.placement.frame;
        match slot.level {
            Some(level) => {
                for key in cells_of(frame, level as usize, slot.lo, slot.hi) {
                    if let Some(list) = self.cells.get_mut(&key) {
                        if let Some(i) = list.iter().position(|e| *e == entity) {
                            list.swap_remove(i);
                        }
                        if list.is_empty() {
                            self.cells.remove(&key);
                        }
                    }
                }
                if let Some(counts) = self.level_counts.get_mut(&frame) {
                    counts[level as usize] = counts[level as usize].saturating_sub(1);
                }
            }
            None => {
                if let Some(set) = self.oversized.get_mut(&frame) {
                    set.remove(&entity);
                }
            }
        }
        // Membership: `entity` leaves `frame`. If `entity` is itself a frame it stops being one
        // of `frame`'s subframes; if `frame` is now empty it stops being a frame at all, so it
        // leaves its own parent's subframe list.
        if let Some(kids) = self.children.get_mut(&frame) {
            kids.remove(&entity);
        }
        if let Some(subs) = self.subframes.get_mut(&frame) {
            subs.remove(&entity);
        }
        if !self.is_frame(frame) {
            if let Some(parent) = self.slots.get(&frame).map(|s| s.placement.frame) {
                if let Some(subs) = self.subframes.get_mut(&parent) {
                    subs.remove(&frame);
                }
            }
        }
    }

    fn file(&mut self, entity: EntityId, placement: Placement) {
        let frame = placement.frame;
        let slot = match level_for(&placement.bounds) {
            Some(level) => {
                let (lo, hi) = cell_range(&placement.bounds, level);
                for key in cells_of(frame, level, lo, hi) {
                    self.cells.entry(key).or_default().push(entity);
                }
                self.level_counts.entry(frame).or_insert([0; LEVELS])[level] += 1;
                Slot {
                    placement,
                    level: Some(level as u8),
                    lo,
                    hi,
                }
            }
            None => {
                self.oversized.entry(frame).or_default().insert(entity);
                Slot {
                    placement,
                    level: None,
                    lo: [0; 3],
                    hi: [0; 3],
                }
            }
        };
        self.slots.insert(entity, slot);
        // Membership: `frame` gains a child. If that makes `frame` a frame for the first time,
        // it joins its parent's subframes; if `entity` is a frame, it joins `frame`'s.
        let became_frame = !self.is_frame(frame);
        self.children.entry(frame).or_default().insert(entity);
        if became_frame {
            if let Some(parent) = self.slots.get(&frame).map(|s| s.placement.frame) {
                self.subframes.entry(parent).or_default().insert(frame);
            }
        }
        if self.is_frame(entity) {
            self.subframes.entry(frame).or_default().insert(entity);
        }
    }
}

/// Every cell key in the inclusive range at `level` of `frame`.
fn cells_of(frame: EntityId, level: usize, lo: [i64; 3], hi: [i64; 3]) -> Vec<CellKey> {
    let mut out = Vec::new();
    for x in lo[0]..=hi[0] {
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                out.push((frame, level as u8, x, y, z));
            }
        }
    }
    out
}

impl SpatialQuery for SpatialIndex {
    fn watches(&self) -> &[FactType] {
        self.projector.watches()
    }

    fn placement(&self, entity: EntityId) -> Option<Placement> {
        self.slots.get(&entity).map(|s| s.placement)
    }

    fn candidates(&self, frame: EntityId, bounds: &Aabb) -> Vec<(EntityId, Placement)> {
        let mut ids: Vec<EntityId> = Vec::new();
        // Plan the probes: per non-empty level, the cells the query spans. Too many in total
        // and a scan of the frame's members is the cheaper honest answer.
        let counts = self
            .level_counts
            .get(&frame)
            .copied()
            .unwrap_or([0; LEVELS]);
        let mut plan = [None; LEVELS];
        let mut total: u128 = 0;
        for (level, &n) in counts.iter().enumerate() {
            if n > 0 {
                let (lo, hi) = cell_range(bounds, level);
                total = total.saturating_add(cell_count(lo, hi));
                plan[level] = Some((lo, hi));
            }
        }
        if total > PROBE_LIMIT {
            ids.extend(self.children.get(&frame).into_iter().flatten().copied());
        } else {
            for (level, range) in plan.iter().enumerate() {
                let Some((lo, hi)) = range else { continue };
                for x in lo[0]..=hi[0] {
                    for y in lo[1]..=hi[1] {
                        for z in lo[2]..=hi[2] {
                            if let Some(list) = self.cells.get(&(frame, level as u8, x, y, z)) {
                                ids.extend_from_slice(list);
                            }
                        }
                    }
                }
            }
            ids.extend(self.oversized.get(&frame).into_iter().flatten().copied());
        }
        // A box spanning several cells was filed in each; a cell is coarser than the boxes in
        // it. Sort, dedupe, and keep exactly the boxes that intersect.
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .filter_map(|e| {
                let p = self.slots.get(&e)?.placement;
                p.bounds.intersects(bounds).then_some((e, p))
            })
            .collect()
    }

    fn children(&self, frame: EntityId) -> Vec<EntityId> {
        self.children
            .get(&frame)
            .map(|c| c.iter().copied().collect())
            .unwrap_or_default()
    }

    fn subframes(&self, frame: EntityId) -> Vec<EntityId> {
        self.subframes
            .get(&frame)
            .map(|c| c.iter().copied().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::{level_for, Aabb, Placement, SpatialIndex, SpatialProjector, SpatialQuery};
    use crate::fact::FactType;
    use crate::identity::EntityId;
    use crate::system::CommittedView;
    use std::sync::Arc;

    struct Never;
    impl SpatialProjector for Never {
        fn watches(&self) -> &'static [FactType] {
            &[]
        }
        fn place(&self, _: &dyn CommittedView, _: EntityId) -> Option<Placement> {
            None
        }
    }

    fn e(id: u64) -> EntityId {
        EntityId::from_raw(id)
    }

    fn at(frame: u64, bounds: Aabb) -> Option<Placement> {
        Some(Placement {
            frame: e(frame),
            anchor: bounds.min,
            bounds,
            settled: true,
        })
    }

    fn ids(v: Vec<(EntityId, Placement)>) -> Vec<u64> {
        v.into_iter().map(|(e, _)| e.raw()).collect()
    }

    #[test]
    fn boxes_are_filed_by_size() {
        assert_eq!(level_for(&Aabb::point([5, 5, 5])), Some(0));
        assert_eq!(level_for(&Aabb::new([0; 3], [400, 0, 0])), Some(0));
        assert_eq!(level_for(&Aabb::new([0; 3], [401, 0, 0])), Some(1));
        assert_eq!(level_for(&Aabb::EVERYTHING), None);
    }

    #[test]
    fn candidates_are_exact_and_sorted() {
        let mut ix = SpatialIndex::new(Arc::new(Never));
        ix.update(e(3), at(1, Aabb::point([100, 0, 0])));
        ix.update(e(2), at(1, Aabb::point([5_000, 0, 0])));
        ix.update(e(4), at(1, Aabb::new([-10_000, -10, 0], [10_000, 10, 300]))); // a long wall
        ix.update(e(9), at(7, Aabb::point([100, 0, 0]))); // another frame
        let q = Aabb::around([0, 0, 0], 1_000);
        assert_eq!(ids(ix.candidates(e(1), &q)), vec![3, 4]);
        assert_eq!(ids(ix.candidates(e(1), &Aabb::EVERYTHING)), vec![2, 3, 4]);
        // Moving an entry moves its answers.
        ix.update(e(3), at(1, Aabb::point([9_000, 0, 0])));
        assert_eq!(ids(ix.candidates(e(1), &q)), vec![4]);
        // Removing it removes it everywhere.
        ix.update(e(4), None);
        assert_eq!(ids(ix.candidates(e(1), &Aabb::EVERYTHING)), vec![2, 3]);
    }

    #[test]
    fn frames_and_subframes_track_membership() {
        // house (2) in city (1); bedroom (3) in house; alice (10) in bedroom.
        let mut ix = SpatialIndex::new(Arc::new(Never));
        ix.update(e(2), at(1, Aabb::point([0; 3])));
        ix.update(e(3), at(2, Aabb::point([0; 3])));
        assert_eq!(
            ix.subframes(e(1)),
            vec![e(2)],
            "the house holds the bedroom"
        );
        assert!(
            ix.subframes(e(2)).is_empty(),
            "the bedroom holds nothing yet"
        );
        ix.update(e(10), at(3, Aabb::point([0; 3])));
        assert_eq!(ix.subframes(e(2)), vec![e(3)], "now it does");
        // Alice leaves for the city: the bedroom is no longer a frame.
        ix.update(e(10), at(1, Aabb::point([0; 3])));
        assert!(ix.subframes(e(2)).is_empty());
        assert_eq!(ix.children(e(1)), vec![e(2), e(10)]);
        // The house is re-parented: it carries its subframe membership with it.
        ix.update(e(2), at(50, Aabb::point([0; 3])));
        assert!(ix.subframes(e(1)).is_empty());
        assert_eq!(ix.subframes(e(50)), vec![e(2)]);
    }
}
